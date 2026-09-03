use super::*;

mod create;
mod inspect;
mod kill;
mod list;

#[derive(Debug, Parser)]
pub(crate) enum Subcommand {
  #[command(about = "Create a persistent Chromium session")]
  Create(create::Create),
  #[command(about = "Inspect a Chromium session")]
  Inspect(inspect::Inspect),
  #[command(about = "Kill a Chromium session")]
  Kill(kill::Kill),
  #[command(about = "List Chromium sessions")]
  List,
  #[command(about = "Start the chromad daemon")]
  Serve(Server),
}

impl Subcommand {
  pub(crate) async fn run(self, address: SocketAddr) -> Result {
    match self {
      Self::Create(create) => create.run(address).await,
      Self::Inspect(inspect) => inspect.run(address).await,
      Self::Kill(kill) => kill.run(address).await,
      Self::List => list::run(address).await,
      Self::Serve(server) => server.run(address).await,
    }
  }
}
