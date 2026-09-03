use super::*;

pub(crate) async fn create(
  State(manager): State<Arc<Mutex<SessionManager>>>,
  AxumPath(id): AxumPath<String>,
) -> Result<(StatusCode, Json<SessionInfo>)> {
  Session::validate_id(&id)?;

  let (executable, profile_dir, headless, public_ws_origin) = {
    let mut manager = manager.lock().await;

    if let Some(session) = manager.sessions.get_mut(&id)
      && session.chromium.is_running().map_err(Error::internal)?
    {
      return Err(Error::conflict(format!("session '{id}' already exists")));
    }

    if !manager.creating.insert(id.clone()) {
      return Err(Error::conflict(format!(
        "session '{id}' is already being created"
      )));
    }

    manager.sessions.remove(&id);

    (
      manager.executable.clone(),
      manager.sessions_dir.join(&id).join("profile"),
      manager.headless,
      manager.public_ws_origin.clone(),
    )
  };

  let chromium =
    match Chromium::launch(&executable, &profile_dir, headless).await {
      Ok(chromium) => chromium,
      Err(error) => {
        manager.lock().await.creating.remove(&id);
        return Err(Error::internal(error));
      }
    };

  let created_at_unix_ms = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap_or_default()
    .as_millis();

  let session = Session {
    chromium,
    created_at_unix_ms,
    id: id.clone(),
    user_data_dir: profile_dir,
  };

  let info = session.info(&public_ws_origin);

  let mut manager = manager.lock().await;
  manager.creating.remove(&id);
  manager.sessions.insert(id.clone(), session);

  info!(session = %id, "created session");

  Ok((StatusCode::CREATED, Json(info)))
}

pub(crate) async fn inspect(
  State(manager): State<Arc<Mutex<SessionManager>>>,
  AxumPath(id): AxumPath<String>,
) -> Result<Json<SessionInfo>> {
  let mut manager = manager.lock().await;

  manager.remove_exited()?;

  let session = manager.sessions.get(&id).ok_or_else(|| {
    Error::not_found(format!("session '{id}' does not exist"))
  })?;

  Ok(Json(session.info(&manager.public_ws_origin)))
}

pub(crate) async fn kill(
  State(manager): State<Arc<Mutex<SessionManager>>>,
  AxumPath(id): AxumPath<String>,
) -> Result<StatusCode> {
  let mut session =
    manager.lock().await.sessions.remove(&id).ok_or_else(|| {
      Error::not_found(format!("session '{id}' does not exist"))
    })?;

  session
    .chromium
    .terminate()
    .await
    .map_err(Error::internal)?;

  info!(session = %id, "killed session");

  Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list(
  State(manager): State<Arc<Mutex<SessionManager>>>,
) -> Result<Json<Vec<SessionInfo>>> {
  let mut manager = manager.lock().await;

  manager.remove_exited()?;

  let mut sessions: Vec<_> = manager
    .sessions
    .values()
    .map(|session| session.info(&manager.public_ws_origin))
    .collect();

  sessions.sort_by(|left, right| left.id.cmp(&right.id));

  Ok(Json(sessions))
}
