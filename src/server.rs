use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Server {
  #[arg(
    long,
    env = "CHROMAD_CHROMIUM",
    help = "Path to the Chromium executable",
    value_hint = clap::ValueHint::FilePath
  )]
  chromium: Option<PathBuf>,
  #[arg(
    long,
    env = "CHROMAD_DATA_DIR",
    help = "Directory used to store session data",
    value_hint = clap::ValueHint::DirPath
  )]
  data_dir: Option<PathBuf>,
  #[arg(long, help = "Run Chromium without a visible window")]
  headless: bool,
}

impl Server {
  fn app(manager: Arc<Mutex<SessionManager>>) -> Router {
    Router::new()
      .route("/health", get(|| async { StatusCode::NO_CONTENT }))
      .route("/sessions", get(sessions::list))
      .route(
        "/sessions/{id}",
        get(sessions::inspect)
          .post(sessions::create)
          .delete(sessions::kill),
      )
      .route("/session/{id}", get(cdp::connect))
      .with_state(manager)
  }

  pub(crate) async fn run(self, address: SocketAddr) -> Result {
    tracing_subscriber::fmt()
      .with_env_filter(
        EnvFilter::builder()
          .with_default_directive(Level::INFO.into())
          .from_env_lossy(),
      )
      .init();

    let executable = Chromium::find_executable(self.chromium)?;

    let data_dir = match self.data_dir {
      Some(data_dir) => data_dir,
      None => ProjectDirs::from("", "", "chromad")
        .map(|directories| directories.data_local_dir().to_path_buf())
        .ok_or_else(|| anyhow!("could not determine the data directory"))?,
    };

    let sessions_dir = data_dir.join("sessions");

    tokio_fs::create_dir_all(&sessions_dir)
      .await
      .with_context(|| {
        format!("failed to create data directory {}", sessions_dir.display())
      })?;

    let listener = TcpListener::bind(address)
      .await
      .with_context(|| format!("failed to listen on {address}"))?;

    let local_address = listener
      .local_addr()
      .context("failed to read server address")?;

    let public_address = if local_address.ip().is_unspecified() {
      SocketAddr::from(([127, 0, 0, 1], local_address.port()))
    } else {
      local_address
    };

    let manager = Arc::new(Mutex::new(SessionManager {
      creating: HashSet::new(),
      executable,
      headless: self.headless,
      public_ws_origin: format!("ws://{public_address}"),
      sessions: HashMap::new(),
      sessions_dir,
    }));

    info!(address = %listener.local_addr().context("failed to read server address")?, "chromad is listening");

    axum::serve(listener, Self::app(manager.clone()))
      .with_graceful_shutdown(Self::shutdown_signal())
      .await
      .context("server failed")?;

    info!("stopping Chromium sessions");

    manager.lock().await.shutdown().await;

    Ok(())
  }

  async fn shutdown_signal() {
    #[cfg(unix)]
    {
      let mut terminate =
        match signal::unix::signal(signal::unix::SignalKind::terminate()) {
          Ok(terminate) => terminate,
          Err(error) => {
            warn!(%error, "failed to listen for termination signal");
            return;
          }
        };

      tokio::select! {
        result = signal::ctrl_c() => {
          if let Err(error) = result {
            warn!(%error, "failed to listen for interrupt signal");
          }
        }
        _ = terminate.recv() => {}
      }
    }

    #[cfg(not(unix))]
    if let Err(error) = signal::ctrl_c().await {
      warn!(%error, "failed to listen for interrupt signal");
    }
  }
}
