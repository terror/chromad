use super::*;

pub(crate) struct Session {
  pub(crate) chromium: Chromium,
  pub(crate) created_at_unix_ms: u128,
  pub(crate) id: String,
  pub(crate) user_data_dir: PathBuf,
}

impl Session {
  pub(crate) fn info(&self, public_ws_origin: &str) -> SessionInfo {
    SessionInfo {
      cdp_endpoint: format!("{public_ws_origin}/session/{}", self.id),
      created_at_unix_ms: self.created_at_unix_ms,
      id: self.id.clone(),
      pid: self.chromium.pid(),
      user_data_dir: self.user_data_dir.display().to_string(),
    }
  }

  pub(crate) fn validate_id(id: &str) -> Result {
    let valid_length = !id.is_empty() && id.len() <= 64;

    let mut characters = id.chars();

    let valid_start = characters
      .next()
      .is_some_and(|character| character.is_ascii_alphanumeric());

    let valid_rest = characters.all(|character| {
      character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.')
    });

    if valid_length && valid_start && valid_rest {
      Ok(())
    } else {
      Err(Error::bad_request(
        "session IDs must be 1-64 characters, start with a letter or number, and contain only ASCII letters, numbers, '-', '_', or '.'",
      ))
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn accepts_safe_session_ids() {
    for id in ["github", "my-session", "session_1", "stripe.com"] {
      assert!(Session::validate_id(id).is_ok(), "{id}");
    }
  }

  #[test]
  fn rejects_unsafe_session_ids() {
    for id in ["", "-session", "../session", "has space", "github/one"] {
      assert!(Session::validate_id(id).is_err(), "{id}");
    }
  }
}
