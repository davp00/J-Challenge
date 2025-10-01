use std::{net::SocketAddr, sync::Arc, time::Duration};

use bytes::Bytes;
use dashmap::DashMap;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::mpsc,
};
use uuid::Uuid;

use crate::{
    app_common::{AppError, SocketError},
    app_net::{ParsedMsg, Socket, SocketResult, parse_line},
};

pub mod app_common;
pub mod app_net;

type Registry = Arc<DashMap<String, Socket>>;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let listener = TcpListener::bind("0.0.0.0:5555")
        .await
        .map_err(|e| AppError::SocketError(format!("bind error: {e}")))?;

    println!("App listen in: {:?}", listener.local_addr().unwrap());

    let registry: Registry = Arc::new(DashMap::new());

    loop {
        let (socket, addr) = listener
            .accept()
            .await
            .map_err(|e| AppError::SocketError(format!("accept error: {e}")))?;

        let registry = registry.clone();

        tokio::spawn(async move {
            if let Err(e) = handle_conn(socket, addr, registry).await {
                eprintln!("conn error: {e}");
            }
        });
    }
}

async fn handle_conn(socket: TcpStream, addr: SocketAddr, registry: Registry) -> SocketResult<()> {
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
    let sock = Socket::new(node_id.clone(), tx, Duration::from_secs(2));
    registry.insert(node_id.clone(), sock.clone());

    println!("Conectado {node_id} desde {addr}");

    let writer_task = {
        let node_id = node_id.clone();
        let registry = registry.clone();

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
            ParsedMsg::Res { id, payload } => {
                // Relacionamos respuesta pendiente
                sock.handle_response(id, payload.to_string());
            }
            ParsedMsg::Req { id, payload } => {
                let reply = if payload == "ping" { "pong" } else { payload };
                let _ = sock.send_res(id, reply);
            }
            ParsedMsg::Other(msg) => {
                println!("Other Req: [{node_id}] -> {msg}");
            }
        }
    }

    writer_task.abort();

    Ok(())
}
