use crate::error::SocketError;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::request::RequestDataInput;
use crate::response::ResponseData;
use crate::types::ReqId;
use crate::types::SocketResult;
use bytes::Bytes;
use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

#[derive(Clone)]
pub struct Socket {
    pub id: String,
    tx: mpsc::UnboundedSender<Bytes>,
    pending: Arc<DashMap<Arc<ReqId>, oneshot::Sender<String>>>,
    counter: Arc<AtomicU64>,
    max_duration: Duration,
}

impl fmt::Debug for Socket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Socket").field("id", &self.id).finish()
    }
}

impl Socket {
    pub fn new(id: String, tx: mpsc::UnboundedSender<Bytes>, max_duration: Duration) -> Self {
        Self {
            id,
            tx,
            pending: Arc::new(DashMap::new()),
            counter: Arc::new(AtomicU64::new(1)),
            max_duration,
        }
    }

    pub async fn request(&self, input: RequestDataInput<'_>) -> SocketResult<ResponseData> {
        let req_id = self.get_new_id();
        let request_data = Arc::new(input.from_id(req_id));

        let (tx_resp, rx_resp) = oneshot::channel::<String>();

        let line: String = request_data.to_string();

        print!("Request: {}", line);

        //TODO Find a better way to handle clone
        self.pending.insert(request_data.id.clone().into(), tx_resp);

        self.tx
            .send(Bytes::from(line))
            .map_err(|_| SocketError::WriteChannelClosed(self.id.clone()))?;

        let resp: String = timeout(self.max_duration, rx_resp)
            .await
            .map_err(|_| SocketError::Timeout {
                socket_id: self.id.clone(),
                req_id: request_data.id.clone(),
            })?
            .map_err(|_| SocketError::ResponseChannelClosed {
                socket_id: self.id.clone(),
                req_id: request_data.id.clone(),
            })?;

        let response_data = ResponseData::from_str(resp.as_str())?;

        Ok(response_data)
    }

    //Para Manejar una respuesta asincrona, lo llamamos desde la tarea lectora
    pub fn handle_response(&self, req_id: ReqId, payload: String) {
        println!("Response {req_id} payload={payload}");

        if let Some((_, tx)) = self.pending.remove(&req_id) {
            let _ = tx.send(payload);
        } else {
            // Log útil para ver si llega un RES que nadie espera
            eprintln!(
                "[{}] RES desconocido id={}, payload={}",
                self.id, req_id, payload
            );
        }
    }

    // Para Responder a una Request
    pub fn send_res(&self, response: ResponseData) -> SocketResult<()> {
        let line = response.to_string();

        self.tx
            .send(Bytes::from(line))
            .map_err(|_| SocketError::WriteChannelClosed(self.id.clone()))
    }

    pub fn send_raw(&self, bytes: bytes::Bytes) -> SocketResult<()> {
        self.tx
            .send(bytes)
            .map_err(|_| SocketError::WriteChannelClosed(self.id.clone()))
    }

    pub fn get_new_id(&self) -> ReqId {
        self.counter.fetch_add(1, Ordering::Relaxed).to_string()
    }
}
