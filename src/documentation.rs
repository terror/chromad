use super::*;

#[derive(OpenApi)]
#[openapi(
  info(
    title = "chromad",
    description = "A chromium orchestration daemon for agents. Manage persistent Chromium sessions over HTTP and proxy Chrome DevTools Protocol messages over WebSocket."
  ),
  paths(
    cdp::connect,
    server::health,
    sessions::create,
    sessions::inspect,
    sessions::kill,
    sessions::list
  ),
  components(
    schemas(
      error::ErrorBody,
      session_info::SessionInfo
    )
  ),
  tags(
    (name = "cdp", description = "All Chrome DevTools Protocol proxy endpoints."),
    (name = "health", description = "Daemon health endpoints."),
    (name = "sessions", description = "All session related endpoints.")
  )
)]
pub(crate) struct Documentation;
