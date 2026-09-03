use super::*;

const START_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) struct Chromium {
  child: Child,
  endpoint: String,
}

impl Chromium {
  pub(crate) fn endpoint(&self) -> &str {
    &self.endpoint
  }

  fn executable(path: PathBuf) -> Result<PathBuf> {
    if path.is_file() {
      Ok(path)
    } else {
      Err(
        anyhow!("Chromium executable does not exist: {}", path.display())
          .into(),
      )
    }
  }

  pub(crate) fn find_executable(
    configured: Option<PathBuf>,
  ) -> Result<PathBuf> {
    if let Some(path) = configured {
      return Self::executable(path);
    }

    let application_paths = [
      "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
      "/Applications/Chromium.app/Contents/MacOS/Chromium",
      "/Applications/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing",
      "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
    ];

    for path in application_paths {
      let path = PathBuf::from(path);

      if path.is_file() {
        return Ok(path);
      }
    }

    for name in [
      "chromium",
      "chromium-browser",
      "google-chrome",
      "google-chrome-stable",
      "chrome",
    ] {
      if let Some(path) = Self::find_on_path(name) {
        return Ok(path);
      }
    }

    Err(
      anyhow!(
        "could not find Chromium; pass --chromium or set CHROMAD_CHROMIUM"
      )
      .into(),
    )
  }

  fn find_on_path(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;

    env::split_paths(&path)
      .map(|directory| directory.join(name))
      .find(|candidate| candidate.is_file())
  }

  pub(crate) fn is_running(&mut self) -> Result<bool> {
    Ok(
      self
        .child
        .try_wait()
        .context("failed to check Chromium process")?
        .is_none(),
    )
  }

  pub(crate) async fn launch(
    executable: &Path,
    user_data_dir: &Path,
    headless: bool,
  ) -> Result<Self> {
    tokio_fs::create_dir_all(user_data_dir)
      .await
      .with_context(|| {
        format!(
          "failed to create profile directory {}",
          user_data_dir.display()
        )
      })?;

    let active_port_file = user_data_dir.join("DevToolsActivePort");

    if let Err(error) = tokio_fs::remove_file(&active_port_file).await
      && error.kind() != ErrorKind::NotFound
    {
      return Err(
        anyhow!(error)
          .context("failed to remove stale DevToolsActivePort")
          .into(),
      );
    }

    let log_path = user_data_dir
      .parent()
      .unwrap_or(user_data_dir)
      .join("chromium.log");

    let stdout = Self::open_log(&log_path)?;

    let stderr = stdout
      .try_clone()
      .context("failed to clone Chromium log file")?;

    let mut command = TokioCommand::new(executable);

    command
      .arg("--remote-debugging-address=127.0.0.1")
      .arg("--remote-debugging-port=0")
      .arg(format!("--user-data-dir={}", user_data_dir.display()))
      .arg("--no-first-run")
      .arg("--no-default-browser-check")
      .arg("about:blank")
      .stdin(Stdio::null())
      .stdout(Stdio::from(stdout))
      .stderr(Stdio::from(stderr))
      .kill_on_drop(true);

    if headless {
      command.arg("--headless=new");
    }

    let mut child = command.spawn().with_context(|| {
      format!("failed to launch Chromium at {}", executable.display())
    })?;

    let endpoint = if let Ok(result) = timeout(
      START_TIMEOUT,
      Self::wait_for_endpoint(&mut child, &active_port_file),
    )
    .await
    {
      result?
    } else {
      let _ = child.kill().await;
      return Err(
        anyhow!(
          "Chromium did not expose a CDP endpoint within {} seconds; see {}",
          START_TIMEOUT.as_secs(),
          log_path.display()
        )
        .into(),
      );
    };

    Ok(Self { child, endpoint })
  }

  fn open_log(path: &Path) -> Result<File> {
    Ok(
      OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?,
    )
  }

  fn parse_active_port(contents: &str) -> Result<String> {
    let mut lines = contents.lines();

    let port: u16 = lines
      .next()
      .ok_or_else(|| anyhow!("missing CDP port"))?
      .parse()
      .context("invalid CDP port")?;

    let path = lines
      .next()
      .filter(|line| line.starts_with('/'))
      .ok_or_else(|| anyhow!("missing CDP browser path"))?;

    Ok(format!("ws://127.0.0.1:{port}{path}"))
  }

  pub(crate) fn pid(&self) -> u32 {
    self.child.id().unwrap_or_default()
  }

  pub(crate) async fn terminate(&mut self) -> Result {
    if !self.is_running()? {
      return Ok(());
    }

    let closing =
      if let Ok((mut socket, _)) = connect_async(&self.endpoint).await {
        socket
          .send(TungsteniteMessage::Text(
            r#"{"id":1,"method":"Browser.close"}"#.into(),
          ))
          .await
          .is_ok()
      } else {
        false
      };

    if !closing
      || timeout(Duration::from_secs(5), self.child.wait())
        .await
        .is_err()
    {
      self
        .child
        .kill()
        .await
        .context("failed to stop Chromium process")?;
    }

    Ok(())
  }

  async fn wait_for_endpoint(
    child: &mut Child,
    active_port_file: &Path,
  ) -> Result<String> {
    loop {
      if let Some(status) = child
        .try_wait()
        .context("failed to check Chromium process")?
      {
        return Err(
          anyhow!("Chromium exited before CDP was ready ({status})").into(),
        );
      }

      match tokio_fs::read_to_string(active_port_file).await {
        Ok(contents) => match Self::parse_active_port(&contents) {
          Ok(endpoint) => return Ok(endpoint),
          Err(_) => sleep(Duration::from_millis(50)).await,
        },
        Err(error) if error.kind() == ErrorKind::NotFound => {
          sleep(Duration::from_millis(50)).await;
        }
        Err(error) => {
          return Err(
            anyhow!(error)
              .context("failed to read DevToolsActivePort")
              .into(),
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parses_devtools_active_port() {
    let endpoint = Chromium::parse_active_port(
      "49152\n/devtools/browser/7c56e839-4591-4f91-ae91\n",
    )
    .unwrap();

    assert_eq!(
      endpoint,
      "ws://127.0.0.1:49152/devtools/browser/7c56e839-4591-4f91-ae91"
    );
  }

  #[test]
  fn rejects_incomplete_devtools_active_port() {
    assert!(Chromium::parse_active_port("49152\n").is_err());

    assert!(
      Chromium::parse_active_port("not-a-port\n/devtools/browser/id\n")
        .is_err()
    );
  }
}
