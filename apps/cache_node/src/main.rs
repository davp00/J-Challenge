use std::env;
use std::sync::Arc;
use std::time::Duration;

use app_core::utils::generate_short_id;
use app_net::request::RequestData;
use app_net::request::data::RequestDataOwned;
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::core::domain::models::{AppError, Response};
use crate::core::services::ActionParserService;
use crate::infrastructure::di::CacheNodeModule;
use app_net::{ParsedMsg, RequestDataInput, ResponseData, Socket, parse_line};

pub mod app_common;
pub mod core;
pub mod infrastructure;

async fn handle_request_async(
    app_module: Arc<CacheNodeModule>,
    socket: Socket,
    data: RequestData<'_>,
) {
    let data = RequestDataOwned::from(data);

    let app_module_clone = app_module.clone();
    tokio::spawn(async move {
        let reply = handle_request(app_module_clone, &data.action, &data.payload).await;

        let response = ResponseData::new(data.id, 200, reply);

        println!("Response: {:?}", response);
        let _ = socket.send_res(response);
    });
}

async fn handle_request(app_module: Arc<CacheNodeModule>, action: &str, payload: &str) -> String {
    let cmd = ActionParserService::parse(action, payload);
    let res: Response = app_module.request_controller_service.handle(cmd).await;
    res.to_wire()
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let role = env::var("ROLE").unwrap_or_else(|_| "MASTER".to_string());
    let short_id = generate_short_id(8);

    let node_identity = format!("{role} {short_id}");

    println!("Node Identity: {node_identity}");

    let app_module = Arc::new(CacheNodeModule::init_dependencies());

    let socket = TcpStream::connect("127.0.0.1:5555")
        .await
        .map_err(|e| AppError::SocketError(e.to_string()))?;
    let (reader, mut writer) = socket.into_split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();
    let connection_socket = Socket::new(node_identity.clone(), tx, Duration::from_secs(10));

    let writer_task = {
        let id = connection_socket.id.clone();
        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if let Err(e) = writer.write_all(&bytes).await {
                    eprintln!("[{id}] write error: {e}");
                    break;
                }
            }
        })
    };

    connection_socket
        .send_raw(Bytes::from(format!("{node_identity}\n")))
        .map_err(|e| AppError::SocketError(format!("Failed on identification: {}", e)))?;

    let req_socket = connection_socket.clone();

    tokio::spawn(async move {
        println!(
            "Request Response: {:?}",
            req_socket.request(RequestDataInput::new("PING", "")).await
        );
    });

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
                println!("[{}] servidor cerró la conexión", connection_socket.id);
                break;
            }

            let current_line = parse_line(&line).map_err(|e| {
                AppError::SocketReadingError(format!("Failed Reading Line: {:?}", e))
            })?;

            match current_line {
                ParsedMsg::Req { data } => {
                    println!("{:?}", data);
                    handle_request_async(app_module.clone(), connection_socket.clone(), data).await;
                }
                ParsedMsg::Res { id, raw_response } => {
                    connection_socket.handle_response(id, raw_response.to_string());
                }
                ParsedMsg::Other(msg) => {
                    println!("[srv] {msg}");
                }
            }
        }

        Ok::<(), AppError>(())
    });

    let _ = reader_task.await;
    writer_task.abort();

    Ok(())
}
