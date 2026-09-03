use super::*;

#[utoipa::path(
  get,
  path = "/session/{id}",
  tag = "cdp",
  description = "Establish a WebSocket connection to the Chrome DevTools Protocol endpoint of an existing Chromium session. After the connection is upgraded, Chrome DevTools Protocol messages are proxied bidirectionally between the caller and Chromium.",
  params(
    (
      "id" = String,
      Path,
      description = "Session identifier.",
      example = "github"
    )
  ),
  responses(
    (
      status = StatusCode::SWITCHING_PROTOCOLS,
      description = "The connection was upgraded to a WebSocket and proxied to the session's Chrome DevTools Protocol endpoint."
    ),
    (
      status = StatusCode::NOT_FOUND,
      description = "A session with the given identifier does not exist.",
      body = ErrorBody
    ),
    (
      status = StatusCode::INTERNAL_SERVER_ERROR,
      description = "Internal server error.",
      body = ErrorBody
    )
  )
)]
pub(crate) async fn connect(
  State(manager): State<Arc<Mutex<SessionManager>>>,
  AxumPath(id): AxumPath<String>,
  upgrade: WebSocketUpgrade,
) -> Result<AxumResponse> {
  let endpoint = {
    let mut manager = manager.lock().await;

    manager.remove_exited()?;

    manager
      .sessions
      .get(&id)
      .map(|session| session.chromium.endpoint().to_owned())
      .ok_or_else(|| {
        Error::not_found(format!("session '{id}' does not exist"))
      })?
  };

  Ok(
    upgrade
      .on_upgrade(move |socket| proxy(socket, endpoint))
      .into_response(),
  )
}

async fn proxy(client: WebSocket, endpoint: String) {
  let (upstream, _) = match connect_async(&endpoint).await {
    Ok(connection) => connection,
    Err(error) => {
      warn!(%error, "failed to connect to Chromium CDP endpoint");
      return;
    }
  };

  let (mut client_tx, mut client_rx) = client.split();
  let (mut upstream_tx, mut upstream_rx) = upstream.split();

  loop {
    tokio::select! {
      message = client_rx.next() => {
        match message {
          Some(Ok(message)) => {
            let Some(message) = to_upstream(message) else { break };
            if upstream_tx.send(message).await.is_err() { break; }
          }
          Some(Err(error)) => {
            warn!(%error, "CDP client connection failed");
            break;
          }
          None => break,
        }
      }
      message = upstream_rx.next() => {
        match message {
          Some(Ok(message)) => {
            let Some(message) = to_client(message) else { break };
            if client_tx.send(message).await.is_err() { break; }
          }
          Some(Err(error)) => {
            warn!(%error, "Chromium CDP connection failed");
            break;
          }
          None => break,
        }
      }
    }
  }
}

fn to_client(message: TungsteniteMessage) -> Option<AxumMessage> {
  match message {
    TungsteniteMessage::Text(value) => {
      Some(AxumMessage::Text(value.to_string().into()))
    }
    TungsteniteMessage::Binary(value) => {
      Some(AxumMessage::Binary(value.to_vec().into()))
    }
    TungsteniteMessage::Ping(value) => {
      Some(AxumMessage::Ping(value.to_vec().into()))
    }
    TungsteniteMessage::Pong(value) => {
      Some(AxumMessage::Pong(value.to_vec().into()))
    }
    TungsteniteMessage::Close(_) | TungsteniteMessage::Frame(_) => None,
  }
}

fn to_upstream(message: AxumMessage) -> Option<TungsteniteMessage> {
  match message {
    AxumMessage::Text(value) => {
      Some(TungsteniteMessage::Text(value.to_string().into()))
    }
    AxumMessage::Binary(value) => {
      Some(TungsteniteMessage::Binary(value.to_vec().into()))
    }
    AxumMessage::Ping(value) => {
      Some(TungsteniteMessage::Ping(value.to_vec().into()))
    }
    AxumMessage::Pong(value) => {
      Some(TungsteniteMessage::Pong(value.to_vec().into()))
    }
    AxumMessage::Close(_) => None,
  }
}
