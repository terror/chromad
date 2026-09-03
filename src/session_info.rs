use super::*;

/// Information about a persistent Chromium session.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema)]
pub(crate) struct SessionInfo {
  /// WebSocket endpoint that proxies messages to the session's `Chrome DevTools Protocol`.
  pub(crate) cdp_endpoint: String,
  /// Unix timestamp in milliseconds at which the session was created.
  pub(crate) created_at_unix_ms: u128,
  /// Session identifier.
  pub(crate) id: String,
  /// Process ID of the session's Chromium instance.
  pub(crate) pid: u32,
  /// Absolute path to the session's user data directory.
  pub(crate) user_data_dir: String,
}
