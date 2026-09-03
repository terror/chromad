use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Kill {
  #[arg(help = "Session name")]
  id: String,
}

impl Kill {
  pub(crate) async fn run(self, address: SocketAddr) -> Result {
    let client = Client::new(address);

    client
      .kill(&self.id)
      .await
      .context("failed to kill session")?;

    Ok(())
  }
}
