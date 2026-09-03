use super::*;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SessionInfo {
  pub(crate) cdp_endpoint: String,
  pub(crate) created_at_unix_ms: u128,
  pub(crate) id: String,
  pub(crate) pid: u32,
  pub(crate) user_data_dir: String,
}
