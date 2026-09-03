use super::*;

#[derive(Debug, Parser)]
#[command(
  name = "chromad",
  version,
  about = "A daemon for persistent Chromium sessions",
  arg_required_else_help = true,
  disable_help_subcommand = true,
  propagate_version = true,
  help_template = "{bin} {version}\n\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
pub(crate) struct Arguments {
  #[arg(
    long,
    global = true,
    env = "CHROMAD_ADDRESS",
    default_value = "127.0.0.1:9223",
    help = "Daemon address"
  )]
  address: SocketAddr,
  #[command(subcommand)]
  subcommand: Subcommand,
}

impl Arguments {
  pub(crate) async fn run(self) -> Result {
    self.subcommand.run(self.address).await
  }
}
