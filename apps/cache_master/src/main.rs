use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

use app_core::UseCaseValidatable;
use bytes::Bytes;
use dashmap::DashMap;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use uuid::Uuid;

use app_net::{
    ParsedMsg, RequestDataInput, ResponseData, Socket, SocketError, parse_line, types::SocketResult,
};

use crate::{
    core::domain::models::{
        AppError, EntryNode, NodeType, usecases::assign_node_use_case::AssignNodeUseCaseInput,
    },
    infrastructure::{
        app_state::{AppNetworkNode, AppState},
        di::CacheMasterModule,
        utils::NodeKind,
    },
};

pub mod core;
pub mod infrastructure;

type Registry = Arc<DashMap<String, Socket>>;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let listener = TcpListener::bind("0.0.0.0:5555")
        .await
        .map_err(|e| AppError::SocketError(format!("bind error: {e}")))?;

    println!("App listen in: {:?}", listener.local_addr().unwrap());

    let app_state = AppState::new_shared();
    let module_dependencies = Arc::new(CacheMasterModule::build_from_state(app_state.clone()));

    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .map_err(|e| AppError::SocketError(format!("accept error: {e}")))?;

        let app_state = app_state.clone();
        let module_dependencies = module_dependencies.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_conn(socket, addr, app_state, module_dependencies).await {
                eprintln!("conn error: {e}");
            }
        });
    }
}

async fn handle_conn(
    socket: TcpStream,
    addr: SocketAddr,
    app_state: Arc<AppState>,
    module_dependencies: Arc<CacheMasterModule>,
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

    let sock = Arc::new(Socket::new(
        entry_node.id.clone(),
        tx,
        Duration::from_secs(2),
    ));
    let network_node = AppNetworkNode::new_shared(sock.clone());

    match entry_node.node_type {
        NodeType::Master => {
            app_state
                .network_state
                .nodes_registry
                .insert(id.clone(), network_node.clone());
            let _ = module_dependencies
                .assign_node_use_case
                .validate_and_execute(AssignNodeUseCaseInput {
                    node_id: entry_node.id,
                    node_type: NodeType::Master,
                })
                .await;
        }
        NodeType::Replica => {
            app_state
                .network_state
                .nodes_registry
                .insert(id.clone(), network_node.clone());
            let _ = module_dependencies
                .assign_node_use_case
                .validate_and_execute(AssignNodeUseCaseInput {
                    node_id: entry_node.id,
                    node_type: NodeType::Replica,
                })
                .await;
        }
        NodeType::Client => {
            println!("Client connected: {}", entry_node.id);
        }
    };

    println!("Conectado {} desde {addr}", id);

    let writer_task = {
        let node_id = id.clone();
        let registry = app_state.network_state.nodes_registry.clone();

        tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if let Err(e) = writer.write_all(&bytes).await {
                    eprintln!("[{node_id}] write error: {e}");
                    break;
                }
            }
            registry.remove(&node_id);
        })
    };

    let sock_copy = network_node.socket.clone();

    tokio::spawn(async move {
        println!(
            "Result PUT: {:?}",
            sock_copy
                .request(RequestDataInput::new("PUT", "test value"))
                .await
        );

        println!(
            "Result GET: {:?}",
            sock_copy
                .request(RequestDataInput::new("GET", "test"))
                .await
        );

        println!(
            "Result GET 2: {:?}",
            sock_copy
                .request(RequestDataInput::new("GET", "test2"))
                .await
        );
    });

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
                sock.handle_response(id, raw_response.to_string());
            }
            ParsedMsg::Req { data } => {
                let reply = if data.action == "PING" {
                    "PONG"
                } else {
                    data.payload
                };

                let dummy_response = ResponseData::new(data.id, 200, reply.to_string());

                let _ = sock.send_res(dummy_response);
            }
            ParsedMsg::Other(msg) => {
                println!("Other Req: [] -> {msg}");
            }
        }
    }

    writer_task.abort();

    Ok(())
}
