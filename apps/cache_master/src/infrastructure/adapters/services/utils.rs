use std::sync::Arc;

use app_net::{RequestDataInput, ResponseData, SocketError, types::SocketResult};
use tokio::task::JoinSet;

use crate::infrastructure::app_state::AppNetworkNode;

pub async fn request_all_race_first_abort_rest(
    sockets: &[Arc<AppNetworkNode>],
    input: RequestDataInput<'_>,
) -> SocketResult<ResponseData> {
    println!("Racing request to {} nodes", sockets.len());

    if sockets.is_empty() {
        return Err(SocketError::ConnectionError("no hay sockets".into()));
    }

    let action_backing = Arc::<str>::from(input.action);
    let payload_backing = Arc::<str>::from(input.payload);

    let mut set = JoinSet::new();

    for s in sockets.iter().cloned() {
        let action = Arc::clone(&action_backing);
        let payload = Arc::clone(&payload_backing);

        // cada future hace su request independiente
        set.spawn(async move {
            let socket_input = RequestDataInput {
                action: &action, // &Arc<str> -> &str
                payload: &payload,
            };

            s.socket.request(socket_input).await
        });
    }

    let mut last_err: Option<SocketError> = None;

    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(Ok(resp)) => {
                // ¡Ganador! aborta el resto
                set.abort_all();
                return Ok(resp);
            }
            Ok(Err(e)) => {
                last_err = Some(e);
            }
            Err(join_err) => {
                // task panicked o fue abortada: lo consideramos como error
                last_err = Some(SocketError::Internal(join_err.to_string()));
            }
        }
    }

    Err(last_err.unwrap_or_else(|| SocketError::Timeout {
        socket_id: "all".into(),
        req_id: "unknown".into(),
    }))
}
