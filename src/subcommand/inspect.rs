use super::*;

#[derive(Debug, Parser)]
pub(crate) struct Inspect {
  #[arg(help = "Session name")]
  id: String,
}

impl Inspect {
  pub(crate) async fn run(self, address: SocketAddr) -> Result {
    let client = Client::new(address);

    let session = client
      .inspect(&self.id)
      .await
      .context("failed to inspect session")?;

    println!("Name: {}", session.id);
    println!("CDP endpoint: {}", session.cdp_endpoint);
    println!("Profile: {}", session.user_data_dir);
    println!("PID: {}", session.pid);
    println!("Created: {}", session.created_at_unix_ms);

    Ok(())
  }
}
