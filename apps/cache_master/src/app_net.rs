use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bytes::Bytes;
use dashmap::DashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;

use crate::app_common::SocketError;

pub type ReqId = String;

pub type SocketResult<T> = Result<T, SocketError>;

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

    pub async fn request(&self, payload: &str) -> SocketResult<String> {
        let req_id = self.get_new_id();

        let (tx_resp, rx_resp) = oneshot::channel::<String>();

        let line = format!("REQ {req_id} {payload}\n");

        self.pending.insert(req_id.clone(), tx_resp);

        self.tx
            .send(Bytes::from(line))
            .map_err(|_| SocketError::WriteChannelClosed(self.id.clone()))?;

        let resp = timeout(self.max_duration, rx_resp)
            .await
            .map_err(|_| SocketError::Timeout {
                socket_id: self.id.clone(),
                req_id: req_id.clone().to_string(),
            })?
            .map_err(|_| SocketError::ResponseChannelClosed {
                socket_id: self.id.clone(),
                req_id: req_id.clone().to_string(),
            })?;

        Ok(resp)
    }

    //Para Manejar una respuesta asincrona, lo llamamos desde la tarea lectora
    pub fn handle_response(&self, req_id: ReqId, payload: String) {
        if let Some((_, tx)) = self.pending.remove(&req_id) {
            let _ = tx.send(payload);
        }
    }

    // Para Responder a una Request
    pub fn send_res(&self, req_id: ReqId, payload: &str) -> SocketResult<()> {
        let line = format!("RES {req_id} {payload}\n");
        self.tx
            .send(Bytes::from(line))
            .map_err(|_| SocketError::WriteChannelClosed(self.id.clone()))
    }

    pub fn get_new_id(&self) -> Arc<ReqId> {
        Arc::new(self.counter.fetch_add(1, Ordering::Relaxed).to_string())
    }
}

pub enum ParsedMsg<'a> {
    Req { id: ReqId, payload: &'a str },
    Res { id: ReqId, payload: &'a str },
    Other(&'a str), // Línea cualquiera (compat/log)
}

pub fn parse_line(line: &str) -> Result<ParsedMsg<'_>, SocketError> {
    let msg = line.trim();

    if let Some(rest) = msg.strip_prefix("REQ ") {
        let (id_str, payload) = split_once_space(rest)?;

        let id = id_str
            .parse::<ReqId>()
            .map_err(|_| SocketError::BadMessage(msg.to_string()))?;

        return Ok(ParsedMsg::Req { id, payload });
    }

    if let Some(rest) = msg.strip_prefix("RES ") {
        let (id_str, payload) = split_once_space(rest)?;

        let id = id_str
            .parse::<ReqId>()
            .map_err(|_| SocketError::BadMessage(msg.to_string()))?;

        return Ok(ParsedMsg::Res { id, payload });
    }

    Ok(ParsedMsg::Other(msg))
}

fn split_once_space(input: &str) -> Result<(&str, &str), SocketError> {
    input
        .split_once(' ')
        .ok_or_else(|| SocketError::BadMessage(input.to_string()))
}
