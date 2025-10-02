use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bytes::Bytes;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
    task::JoinHandle,
};

use app_core::utils::generate_short_id;
use app_net::{ParsedMsg, RequestDataInput, ResponseData, Socket, parse_line};
use tracing::error;

use crate::errors::AppError;

#[derive(Clone, Debug)]
pub struct CacheClientConfig {
    pub node_ips: Vec<String>,
    pub connect_timeout: Duration,
    pub request_timeout: Duration,
    pub retry_backoff: Duration,
}

impl CacheClientConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let cache_nodes_var = env::var("CACHE_IPS")
            .map_err(|_| AppError::ConnectionError("CACHE_IPS not set".into()))?;

        let node_ips: Vec<String> = cache_nodes_var
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if node_ips.is_empty() {
            return Err(AppError::ConnectionError("CACHE_IPS is empty".into()));
        }

        Ok(Self {
            node_ips,
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
            retry_backoff: Duration::from_millis(300),
        })
    }
}

impl Default for CacheClientConfig {
    fn default() -> Self {
        Self {
            node_ips: vec!["127.0.0.1:5555".to_string()],
            connect_timeout: Duration::from_secs(5),
            request_timeout: Duration::from_secs(10),
            retry_backoff: Duration::from_millis(300),
        }
    }
}

/// A lightweight client that connects to one master at a time and fails over if needed.
pub struct CacheClient {
    cfg: CacheClientConfig,
    node_id: Arc<str>,
    /// Index of the *last* successfully connected master (for sticky reconnects).
    current_idx: AtomicUsize,
    /// The active logical socket abstraction used to send requests and receive responses.
    socket: parking_lot::RwLock<Option<Arc<Socket>>>,
    /// IO tasks associated with the current connection (writer and reader).
    io_writer: parking_lot::Mutex<Option<JoinHandle<()>>>,
    io_reader: parking_lot::Mutex<Option<JoinHandle<Result<(), AppError>>>>,
}

impl CacheClient {
    /// Build a client and eagerly connect to the first available master.
    pub async fn connect_with(cfg: CacheClientConfig) -> Result<Arc<Self>, AppError> {
        let node_id = Arc::<str>::from(generate_short_id(8));
        let client = Arc::new(Self {
            cfg,
            node_id,
            current_idx: AtomicUsize::new(0),
            socket: parking_lot::RwLock::new(None),
            io_writer: parking_lot::Mutex::new(None),
            io_reader: parking_lot::Mutex::new(None),
        });

        client.ensure_connected().await?;
        Ok(client)
    }

    /// Public helper to check and (re)establish a connection when needed.
    pub async fn ensure_connected(&self) -> Result<(), AppError> {
        if self.socket.read().is_some() {
            return Ok(());
        }
        self.try_connect_any().await
    }

    /// Send a raw request; auto-reconnects once if the first attempt fails.
    pub async fn request_raw(&self, action: &str, payload: &str) -> Result<ResponseData, AppError> {
        self.ensure_connected().await?;
        match self.do_request(action, payload).await {
            Ok(s) => Ok(s),
            Err(_) => {
                // One-shot failover retry
                self.break_connection();
                self.try_connect_any().await?;
                self.do_request(action, payload).await
            }
        }
    }

    /// High-level convenience: GET (returns raw string). Use `get_opt` for `Option` handling.
    pub async fn get(&self, key: &str) -> Result<ResponseData, AppError> {
        self.request_raw("GET", key).await
    }

    /// GET but mapped to Option: treats "EMPTY" (or empty line) as None.
    pub async fn get_opt(&self, key: &str) -> Result<Option<String>, AppError> {
        let raw = self.get(key).await?;
        let trimmed = raw.payload.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("EMPTY") {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    /// High-level convenience: PUT ("key value" or with ttl if provided).
    pub async fn put(
        &self,
        key: &str,
        value: &str,
        ttl_secs: Option<u64>,
    ) -> Result<ResponseData, AppError> {
        let payload = match ttl_secs {
            Some(ttl) => format!("{} \"{}\" {}", key, value, ttl),
            None => format!("{} \"{}\"", key, value),
        };
        self.request_raw("PUT", &payload).await
    }

    // --- Internals ---

    async fn do_request(&self, action: &str, payload: &str) -> Result<ResponseData, AppError> {
        let sock = self
            .socket
            .read()
            .as_ref()
            .cloned()
            .ok_or_else(|| AppError::ConnectionError("no active connection".into()))?;
        let res = sock
            .request(RequestDataInput::new(action, payload))
            .await
            .map_err(|e| {
                AppError::SocketError(format!("request failed: {} {} => {}", action, payload, e))
            })?;
        // `res` is likely some ResponseData type. Convert to String in a way your API supports.
        // If `ResponseData` already implements `ToString`, this works; otherwise adjust accordingly.
        Ok(res)
    }

    async fn try_connect_any(&self) -> Result<(), AppError> {
        if self.cfg.node_ips.is_empty() {
            return Err(AppError::ConnectionError(
                "no master addresses provided".into(),
            ));
        }

        let start = self.current_idx.load(Ordering::Relaxed) % self.cfg.node_ips.len();
        // Try from `start`, wrap once.
        for attempt in 0..self.cfg.node_ips.len() {
            let idx = (start + attempt) % self.cfg.node_ips.len();
            match self.open_and_handshake(idx).await {
                Ok(()) => return Ok(()),
                Err(e) => {
                    tracing::warn!(?e, addr = %self.cfg.node_ips[idx], "connect attempt failed; trying next");
                    tokio::time::sleep(self.cfg.retry_backoff).await;
                }
            }
        }

        Err(AppError::ConnectionError("all masters unreachable".into()))
    }

    async fn open_and_handshake(&self, idx: usize) -> Result<(), AppError> {
        let addr = self.cfg.node_ips[idx].clone();
        let stream = tokio::time::timeout(self.cfg.connect_timeout, TcpStream::connect(&addr))
            .await
            .map_err(|_| AppError::ConnectionError(format!("connect timeout to {}", addr)))
            .and_then(|r| {
                r.map_err(|e| AppError::SocketError(format!("connect error to {}: {}", addr, e)))
            })?;

        let (reader, mut writer) = stream.into_split();
        let (tx, mut rx) = mpsc::unbounded_channel::<Bytes>();

        let socket = Arc::new(Socket::new(
            self.node_id.to_string(),
            tx,
            self.cfg.request_timeout,
        ));

        // Writer task: forward outbound bytes to TCP writer
        let writer_id = socket.id.clone();
        let writer_task = tokio::spawn(async move {
            while let Some(bytes) = rx.recv().await {
                if let Err(e) = writer.write_all(&bytes).await {
                    error!("[{}] write error: {}", writer_id, e);
                    break;
                }
            }
        });

        // Identify ourselves once connected
        socket
            .send_raw(Bytes::from(format!("{}\n", self.node_id)))
            .map_err(|e| AppError::SocketError(format!("Failed on identification: {}", e)))?;

        // Reader task: route server lines into `socket.handle_response`
        let reader_socket = socket.clone();
        let reader_task = tokio::spawn(async move {
            let mut br = BufReader::new(reader);
            let mut line = String::new();
            loop {
                line.clear();
                let n = br
                    .read_line(&mut line)
                    .await
                    .map_err(|e| AppError::SocketError(e.to_string()))?;
                if n == 0 {
                    break;
                }
                let current_line = parse_line(&line)
                    .map_err(|e| AppError::SocketError(format!("Failed Reading Line: {:?}", e)))?;
                match current_line {
                    ParsedMsg::Res { id, raw_response } => {
                        reader_socket.handle_response(id, raw_response.to_string())
                    }
                    ParsedMsg::Req { data } => {
                        // Client-side we don't expect server-initiated REQ, but print for visibility.
                        tracing::info!(?data, "server -> client REQ");
                    }
                    ParsedMsg::Other(msg) => tracing::info!(%msg, "server line"),
                }
            }
            Ok::<(), AppError>(())
        });

        // Swap current connection (and abort old one if present)
        self.replace_connection(idx, socket, writer_task, reader_task);
        Ok(())
    }

    fn replace_connection(
        &self,
        idx: usize,
        sock: Arc<Socket>,
        writer: JoinHandle<()>,
        reader: JoinHandle<Result<(), AppError>>,
    ) {
        // Abort previous tasks (if any)
        if let Some(h) = self.io_writer.lock().take() {
            h.abort();
        }
        if let Some(h) = self.io_reader.lock().take() {
            h.abort();
        }
        // Install new
        *self.socket.write() = Some(sock);
        *self.io_writer.lock() = Some(writer);
        *self.io_reader.lock() = Some(reader);
        self.current_idx.store(idx, Ordering::Relaxed);
    }

    /// Break the current connection (forces next request to reconnect/failover).
    pub fn break_connection(&self) {
        if let Some(h) = self.io_writer.lock().take() {
            h.abort();
        }
        if let Some(h) = self.io_reader.lock().take() {
            h.abort();
        }
        *self.socket.write() = None;
    }
}
