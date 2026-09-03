use super::*;

#[derive(Debug)]
pub(crate) struct Client {
  base_url: String,
  http: ReqwestClient,
}

impl Client {
  pub(crate) async fn create(&self, id: &str) -> Result<SessionInfo> {
    self
      .response(
        self
          .http
          .post(self.session_url(id))
          .send()
          .await
          .context("failed to send daemon request")?,
      )
      .await
  }

  async fn empty_response(&self, response: ReqwestResponse) -> Result {
    if response.status().is_success() {
      return Ok(());
    }

    Err(Error::from_response(response).await)
  }

  pub(crate) async fn inspect(&self, id: &str) -> Result<SessionInfo> {
    self
      .response(
        self
          .http
          .get(self.session_url(id))
          .send()
          .await
          .context("failed to send daemon request")?,
      )
      .await
  }

  pub(crate) async fn kill(&self, id: &str) -> Result {
    self
      .empty_response(
        self
          .http
          .delete(self.session_url(id))
          .send()
          .await
          .context("failed to send daemon request")?,
      )
      .await
  }

  pub(crate) async fn list(&self) -> Result<Vec<SessionInfo>> {
    self
      .response(
        self
          .http
          .get(format!("{}/sessions", self.base_url))
          .send()
          .await
          .context("failed to send daemon request")?,
      )
      .await
  }

  pub(crate) fn new(address: SocketAddr) -> Self {
    let address = if address.ip().is_unspecified() {
      SocketAddr::from(([127, 0, 0, 1], address.port()))
    } else {
      address
    };

    Self {
      base_url: format!("http://{address}"),
      http: ReqwestClient::new(),
    }
  }

  async fn response<T: DeserializeOwned>(
    &self,
    response: ReqwestResponse,
  ) -> Result<T> {
    if response.status().is_success() {
      return Ok(
        response
          .json()
          .await
          .context("failed to decode daemon response")?,
      );
    }

    Err(Error::from_response(response).await)
  }

  fn session_url(&self, id: &str) -> String {
    format!("{}/sessions/{id}", self.base_url)
  }
}
