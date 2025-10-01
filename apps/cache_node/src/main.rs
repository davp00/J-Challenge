pub mod cache;
pub mod utils;

use std::fmt::format;
use std::time::Duration;

use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::app_common::{AppError, SocketError};
use crate::app_net::{ParsedMsg, Socket, parse_line};

pub mod app_common;
pub mod app_net;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let socket = TcpStream::connect("127.0.0.1:5555")
        .await
        .map_err(|e| AppError::SocketError(e.to_string()))?;
    let (reader, mut writer) = socket.into_split();

    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();
    let connection_socket = Socket::new("Nodo1".to_string(), tx, Duration::from_secs(1));

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
        .send_raw(Bytes::from_static(b"Nodo1\n"))
        .map_err(|e| AppError::SocketError(format!("Failed on identification: {}", e)))?;

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
                ParsedMsg::Req { id, payload } => {
                    let reply = handle_request(payload).await;
                    let _ = connection_socket.send_res(id, &reply);
                }
                ParsedMsg::Res { id, payload } => {
                    connection_socket.handle_response(id, payload.to_string());
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

async fn handle_request(payload: &str) -> String {
    match payload {
        "ping" => "pong".to_string(),
        other => format!("echo:{other}"),
    }
}
