use std::env;
use std::sync::Arc;
use std::time::Duration;

use app_core::utils::generate_short_id;
use app_net::request::data::RequestDataOwned;
use app_net::{
    ParsedMsg, RequestDataInput, ResponseData, Socket, parse_line, request::RequestData,
};
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tracing::{error, info, trace};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::core::domain::models::{AppError, Response};
use crate::core::services::ActionParserService;
use crate::infrastructure::di::CacheNodeModule;

pub mod app_common;
pub mod core;
pub mod infrastructure;
pub mod tests;

// ---------- helpers ----------

async fn handle_request(app_module: Arc<CacheNodeModule>, action: &str, payload: &str) -> String {
    let cmd = ActionParserService::parse(action, payload);
    let res: Response = app_module.request_controller_service.handle(cmd).await;
    res.to_wire()
}

async fn handle_request_async(
    app_module: Arc<CacheNodeModule>,
    socket: Arc<Socket>,
    data: RequestData<'_>,
) {
    let data = RequestDataOwned::from(data);
    let app_module_clone = app_module.clone();
    tokio::spawn(async move {
        let reply = handle_request(app_module_clone, &data.action, &data.payload).await;
        let response = ResponseData::new(data.id, 200, reply);
        let _ = socket.send_res(response);
    });
}

// ---------- main ----------
#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    dotenvy::dotenv().ok();
    let _ = dotenvy::from_filename(concat!(env!("CARGO_MANIFEST_DIR"), "/.env"));
    let _ = dotenvy::from_filename(".env");

    let role = env::var("ROLE").unwrap_or_else(|_| "MASTER".to_string());
    let short_id = generate_short_id(8);
    let node_identity = format!("{role} {short_id}");
    info!("Node Identity: {node_identity}");

    let app_module = Arc::new(CacheNodeModule::init_dependencies());

    let addrs = parse_master_ips();
    info!("Master IPs: {:?}", addrs);

    // una tarea por servidor
    let mut set = JoinSet::new();
    for s in parse_master_ips() {
        let app = app_module.clone();
        let ident = node_identity.clone();
        let addr_arc: Arc<str> = Arc::<str>::from(s); // de String -> Arc<str>
        set.spawn(run_connection_loop(app, ident, addr_arc));
    }

    // Mantén vivo el proceso: si alguna tarea termina, la reportamos y seguimos.
    loop {
        if let Some(res) = set.join_next().await {
            match res {
                Ok(Ok(())) => info!("Conexión terminó (Ok)"),
                Ok(Err(e)) => error!("Conexión terminó con error: {e:?}"),
                Err(join_err) => error!("Conexión paniqueó: {join_err:?}"),
            }
        }
    }
}

fn parse_master_ips() -> Vec<String> {
    let raw = env::var("MASTER_IPS").unwrap_or_else(|_| "".to_string());
    raw.split(|c| c == ',' || c == ' ')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

// Lanza y mantiene una conexión (con reconexión) a un addr específico
async fn run_connection_loop(
    app_module: Arc<CacheNodeModule>,
    node_identity: String,
    addr: Arc<str>,
) -> Result<(), AppError> {
    let mut backoff = Duration::from_millis(500);
    let max_backoff = Duration::from_secs(10);

    loop {
        // ——— CLON LOCAL PARA ESTA ITERACIÓN ———
        let addr_iter = addr.clone();

        info!(target: "conn", "Conectando a {}...", &*addr_iter);

        match TcpStream::connect(&*addr_iter).await {
            Ok(stream) => {
                info!(target: "conn", "Conectado a {}", &*addr_iter);
                let (reader, mut writer) = stream.into_split();

                let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();
                let connection_socket = Arc::new(Socket::new(
                    node_identity.clone(),
                    tx,
                    Duration::from_secs(10),
                ));

                // writer_task
                let writer_id = connection_socket.id.clone();
                let writer_task = tokio::spawn(async move {
                    while let Some(bytes) = rx.recv().await {
                        if let Err(e) = writer.write_all(&bytes).await {
                            error!(target:"conn", "[{writer_id}] write error: {e}");
                            break;
                        }
                    }
                });

                // Identificación
                connection_socket
                    .send_raw(Bytes::from(format!("{}\n", node_identity)))
                    .map_err(|e| {
                        AppError::SocketError(format!("Failed on identification: {}", e))
                    })?;

                // PING (usa otro clon)
                {
                    let req_socket = connection_socket.clone();
                    let addr_ping = addr_iter.clone();
                    tokio::spawn(async move {
                        trace!(
                            "PING({}): {:?}",
                            &*addr_ping,
                            req_socket.request(RequestDataInput::new("PING", "")).await
                        );
                    });
                }

                // reader_task (usa otro clon)
                let reader_socket = connection_socket.clone();
                let app_module_clone = app_module.clone();
                let addr_reader = addr_iter.clone();
                let reader_task = tokio::spawn(async move {
                    let mut br = BufReader::new(reader);
                    let mut line = String::new();

                    loop {
                        line.clear();
                        let n = br
                            .read_line(&mut line)
                            .await
                            .map_err(|e| AppError::SocketReadingError(e.to_string()))?;

                        if n == 0 {
                            info!(target:"conn",
                                  "[{}] servidor cerró la conexión ({})",
                                  reader_socket.id, &*addr_reader);
                            break;
                        }

                        let current_line = parse_line(&line).map_err(|e| {
                            AppError::SocketReadingError(format!("Failed Reading Line: {:?}", e))
                        })?;

                        match current_line {
                            ParsedMsg::Req { data } => {
                                handle_request_async(
                                    app_module_clone.clone(),
                                    reader_socket.clone(),
                                    data,
                                )
                                .await;
                            }
                            ParsedMsg::Res { id, raw_response } => {
                                reader_socket.handle_response(id, raw_response.to_string());
                            }
                            ParsedMsg::Other(msg) => {
                                info!(target:"srv", "[{}] {}", &*addr_reader, msg);
                            }
                        }
                    }

                    Ok::<(), AppError>(())
                });

                // Espera fin del reader; corta writer; backoff
                let res = reader_task.await;
                writer_task.abort();

                match res {
                    Ok(Ok(())) => info!(target:"conn", "Reader finalizó para {}", &*addr_iter),
                    Ok(Err(e)) => error!(target:"conn", "Reader error en {}: {:?}", &*addr_iter, e),
                    Err(e) => error!(target:"conn", "Reader panic en {}: {:?}", &*addr_iter, e),
                }

                info!(target:"conn", "Reintentando {} en {:?}...", &*addr_iter, backoff);
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
            Err(e) => {
                error!(target:"conn",
                    "No se pudo conectar a {}: {}. Reintentando en {:?}...",
                    &*addr_iter, e, backoff);
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(max_backoff);
            }
        }
        // aquí termina la vida de `addr_iter`; en la siguiente vuelta clonamos `addr` de nuevo
    }
}
