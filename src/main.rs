use {
  anyhow::{Context, Error as AnyhowError, anyhow},
  arguments::Arguments,
  axum::{
    Json, Router,
    extract::{
      Path as AxumPath, State, WebSocketUpgrade,
      ws::{Message as AxumMessage, WebSocket},
    },
    http::StatusCode,
    response::{IntoResponse, Response as AxumResponse},
    routing::get,
  },
  chromium::Chromium,
  clap::Parser,
  client::Client,
  directories::ProjectDirs,
  documentation::Documentation,
  error::{Error, ErrorBody},
  futures_util::{SinkExt, StreamExt},
  reqwest::{Client as ReqwestClient, Response as ReqwestResponse},
  serde::{Deserialize, Serialize, de::DeserializeOwned},
  server::Server,
  session::Session,
  session_info::SessionInfo,
  session_manager::SessionManager,
  std::{
    backtrace::BacktraceStatus,
    collections::{HashMap, HashSet},
    error::Error as StdError,
    fmt::{self, Display, Formatter},
    fs::{File, OpenOptions},
    io::ErrorKind,
    net::SocketAddr,
    path::{Path, PathBuf},
    process::{self, Stdio},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
  },
  subcommand::Subcommand,
  tokio::{
    fs as tokio_fs,
    net::TcpListener,
    process::{Child, Command as TokioCommand},
    signal,
    sync::Mutex,
    time::{sleep, timeout},
  },
  tokio_tungstenite::{
    connect_async, tungstenite::Message as TungsteniteMessage,
  },
  tracing::{Level, info, warn},
  tracing_subscriber::EnvFilter,
  utoipa::{OpenApi, ToSchema},
  utoipa_scalar::{Scalar, Servable},
};

mod arguments;
mod cdp;
mod chromium;
mod client;
mod documentation;
mod error;
mod server;
mod session;
mod session_info;
mod session_manager;
mod sessions;
mod subcommand;

type Result<T = (), E = Error> = std::result::Result<T, E>;

#[tokio::main]
async fn main() {
  if let Err(error) = Arguments::parse().run().await {
    eprintln!("error: {error}");

    if let Error::Internal(error) = error {
      for (i, error) in error.chain().skip(1).enumerate() {
        if i == 0 {
          eprintln!();
          eprintln!("because:");
        }

        eprintln!("- {error}");
      }

      let backtrace = error.backtrace();

      if backtrace.status() == BacktraceStatus::Captured {
        eprintln!("backtrace:");
        eprintln!("{backtrace}");
      }
    }

    process::exit(1);
  }
}
