use super::*;

pub(crate) async fn run(address: SocketAddr) -> Result {
  let client = Client::new(address);

  let sessions = client.list().await.context("failed to list sessions")?;

  if sessions.is_empty() {
    return Ok(());
  }

  println!("NAME\tCDP ENDPOINT");

  for session in sessions {
    println!("{}\t{}", session.id, session.cdp_endpoint);
  }

  Ok(())
}
