use std::{env, net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use app_core::UseCaseValidatable;
use bytes::Bytes;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use uuid::Uuid;

use app_net::{
    ParsedMsg, ResponseData, Socket, SocketError, parse_line,
    request::{RequestData, data::RequestDataOwned},
    types::SocketResult,
};

use crate::{
    core::domain::models::{
        AppError, EntryNode, NodeType,
        usecases::{RemoveNodeUseCaseInput, assign_node_use_case::AssignNodeUseCaseInput},
    },
    infrastructure::{
        adapters::controllers::request_controller::RequestController,
        app_state::{AppNetworkNode, AppState},
        di::CacheMasterModule,
    },
};

pub mod core;
pub mod infrastructure;
pub mod tests;

async fn handle_request_async(
    request_controller: Arc<RequestController>,
    socket: Arc<Socket>,
    data: RequestData<'_>,
) {
    let data = RequestDataOwned::from(data);

    let request_controller = request_controller.clone();
    tokio::spawn(async move {
        let reply = request_controller
            .handle_request(&data.action, &data.payload)
            .await;

        let response = if let Ok(reply) = reply {
            ResponseData::new(data.id, 200, reply)
        } else {
            ResponseData::new(data.id, 500, format!("ERROR {}", reply.err().unwrap()))
        };

        let _ = socket.send_res(response);
    });
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let port = env::var("PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(5555);

    let addr: String = format!("0.0.0.0:{port}");

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| AppError::SocketError(format!("bind error: {e}")))?;

    info!("App listen in: {:?}", listener.local_addr().unwrap());

    let app_state = AppState::new_shared();
    let module_dependencies = Arc::new(CacheMasterModule::build_from_state(app_state.clone()));
    let request_controller = Arc::new(RequestController::new(module_dependencies.clone()));

    /*
    let service = module_dependencies.tcp_network_service.clone();
    //let app_state_clone = app_state.clone();

    //let modules_dependencies_clone = module_dependencies.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(10));

        loop {
            interval.tick().await;
            println!("--- Network State {:?} ---", time::Instant::now());
            service.pretty_print();
            println!("---------------------------");
        }
    });*/

    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .map_err(|e| AppError::SocketError(format!("accept error: {e}")))?;

        let app_state = app_state.clone();
        let module_dependencies = module_dependencies.clone();
        let request_controller = request_controller.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_conn(
                socket,
                addr,
                app_state,
                module_dependencies,
                request_controller,
            )
            .await
            {
                error!("conn error: {e}");
            }
        });
    }
}

async fn handle_conn(
    socket: TcpStream,
    addr: SocketAddr,
    app_state: Arc<AppState>,
    module_dependencies: Arc<CacheMasterModule>,
    request_controller: Arc<RequestController>,
) -> SocketResult<()> {
    let (reader, mut writer) = socket.into_split();

    let mut first_line = String::new();

    let mut reader = BufReader::new(reader);

    let node_id =
        match tokio::time::timeout(Duration::from_secs(5), reader.read_line(&mut first_line)).await
        {
            Ok(Ok(n)) if n > 0 => first_line.trim().to_string(),
            _ => Uuid::new_v4().to_string(),
        };

    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();

    //TODO Remap error
    let entry_node = EntryNode::from_str(node_id.as_str()).unwrap();
    let id: Arc<str> = Arc::from(entry_node.id.as_str());

    let connection_socket = Arc::new(Socket::new(
        entry_node.id.clone(),
        tx,
        Duration::from_secs(2),
    ));
    let network_node = AppNetworkNode::new_shared(connection_socket.clone(), id.clone());

    match entry_node.node_type {
        NodeType::Master | NodeType::Replica => {
            app_state
                .network_state
                .nodes_registry
                .insert(id.clone(), network_node.clone());

            let _ = module_dependencies
                .assign_node_use_case
                .validate_and_execute(AssignNodeUseCaseInput {
                    node_id: entry_node.id,
                    node_type: entry_node.node_type,
                })
                .await
                .ok();
        }
        NodeType::Client => {}
    };

    info!("Conectado {} desde {addr}", id);

    let writer_task = {
        let node_id = id.clone();

        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if let Err(e) = writer.write_all(&bytes).await {
                    error!("[{node_id}] write error: {e}");
                    break;
                }
            }
            info!("[{node_id}] writer task ended");
        })
    };

    let mut line = String::new();
    loop {
        line.clear();

        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| SocketError::BadMessage(format!("read_line error: {e}")))?;

        if n == 0 {
            break; // EOF
        }

        match parse_line(&line)? {
            ParsedMsg::Res { id, raw_response } => {
                // Relacionamos respuesta pendiente
                connection_socket.handle_response(id, raw_response.to_string());
            }
            ParsedMsg::Req { data } => {
                handle_request_async(request_controller.clone(), connection_socket.clone(), data)
                    .await;
            }
            ParsedMsg::Other(msg) => {
                info!("Other Req: [] -> {msg}");
            }
        }
    }

    module_dependencies
        .delete_node_use_case
        .validate_and_execute(RemoveNodeUseCaseInput {
            node_id: id.to_string(),
        })
        .await
        .ok();

    //writer_task.abort();
    drop(connection_socket);

    let _ = writer_task.await;
    println!("Desconectado {} desde {addr}", id);
    Ok(())
}
