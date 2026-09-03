use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Create {
  #[arg(help = "Session name")]
  id: String,
}

impl Create {
  pub(crate) async fn run(self, address: SocketAddr) -> Result {
    let client = Client::new(address);

    let session = client
      .create(&self.id)
      .await
      .context("failed to create session")?;

    println!("{}", session.cdp_endpoint);

    Ok(())
  }
}
