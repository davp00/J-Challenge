pub mod cache;
pub mod utils;

use std::sync::Arc;
use std::time::Duration;

use app_net::request::RequestData;
use app_net::request::data::RequestDataOwned;
use bytes::Bytes;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::mpsc;

use crate::app_common::AppError;
use crate::cache::Cache;
use app_net::{ParsedMsg, RequestDataInput, ResponseData, Socket, parse_line};

pub mod app_common;

pub(crate) struct AppData {
    cache: Arc<Cache<String, String>>,
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let socket = TcpStream::connect("127.0.0.1:5555")
        .await
        .map_err(|e| AppError::SocketError(e.to_string()))?;
    let (reader, mut writer) = socket.into_split();

    let app_data = Arc::new(AppData {
        cache: Cache::new(),
    });

    let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();
    let connection_socket = Socket::new("Nodo1".to_string(), tx, Duration::from_secs(10));

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
                    handle_request_async(app_data.clone(), connection_socket.clone(), data).await;
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

async fn handle_request_async(app_data: Arc<AppData>, socket: Socket, data: RequestData<'_>) {
    let data = RequestDataOwned::from(data);

    tokio::spawn(async move {
        let reply = handle_request(&data.action, &data.payload, app_data).await;

        let response = ResponseData::new(data.id, 200, reply);

        let _ = socket.send_res(response);
    });
}

async fn handle_request(action: &str, payload: &str, app_data: Arc<AppData>) -> String {
    let mut parts = payload.splitn(2, ' ');
    let key = parts.next().unwrap_or_default();

    match action.to_ascii_uppercase().as_str() {
        "PING" => "pong".to_string(),
        "PUT" => {
            let value = parts.next().unwrap_or_default();

            if key.is_empty() || value.is_empty() {
                return "EMPTY".to_string();
            }

            app_data.cache.put(key.to_string(), value.to_string(), None);

            "".to_string()
        }
        "GET" => {
            let key_value = app_data.cache.get(&key.to_string());

            if let Some(value) = key_value {
                return format!("{}", value);
            }

            "".to_string()
        }
        other => format!("echo:{other}"),
    }
}
