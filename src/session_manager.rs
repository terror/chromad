use super::*;

pub(crate) struct SessionManager {
  pub(crate) creating: HashSet<String>,
  pub(crate) executable: PathBuf,
  pub(crate) headless: bool,
  pub(crate) public_ws_origin: String,
  pub(crate) sessions: HashMap<String, Session>,
  pub(crate) sessions_dir: PathBuf,
}

impl SessionManager {
  pub(crate) fn remove_exited(&mut self) -> Result {
    let mut exited = Vec::new();

    for (id, session) in &mut self.sessions {
      if !session.chromium.is_running().map_err(Error::internal)? {
        exited.push(id.clone());
      }
    }

    for id in exited {
      self.sessions.remove(&id);
      warn!(session = %id, "removed exited session");
    }

    Ok(())
  }

  pub(crate) async fn shutdown(&mut self) {
    let sessions = self
      .sessions
      .drain()
      .map(|(_, session)| session)
      .collect::<Vec<_>>();

    for mut session in sessions {
      if let Err(error) = session.chromium.terminate().await {
        warn!(session = %session.id, %error, "failed to stop session");
      }
    }
  }
}
