use super::*;

#[derive(Debug)]
pub(crate) enum Error {
  BadRequest(String),
  Conflict(String),
  Internal(AnyhowError),
  NotFound(String),
}

#[derive(Deserialize, Serialize)]
struct ErrorBody {
  error: String,
  status: u16,
}

impl Error {
  pub(crate) fn bad_request(message: impl Into<String>) -> Self {
    Self::BadRequest(message.into())
  }

  pub(crate) fn conflict(message: impl Into<String>) -> Self {
    Self::Conflict(message.into())
  }

  pub(crate) async fn from_response(response: ReqwestResponse) -> Self {
    let status = response.status();
    let message = match response.json::<ErrorBody>().await {
      Ok(body) => body.error,
      Err(_) => format!("daemon returned HTTP {status}"),
    };

    match status {
      StatusCode::BAD_REQUEST => Self::BadRequest(message),
      StatusCode::CONFLICT => Self::Conflict(message),
      StatusCode::NOT_FOUND => Self::NotFound(message),
      _ => Self::Internal(anyhow!(message)),
    }
  }

  pub(crate) fn internal(error: impl Into<AnyhowError>) -> Self {
    Self::Internal(error.into())
  }

  pub(crate) fn not_found(message: impl Into<String>) -> Self {
    Self::NotFound(message.into())
  }

  fn public_message(&self) -> &str {
    match self {
      Self::BadRequest(message)
      | Self::Conflict(message)
      | Self::NotFound(message) => message,
      Self::Internal(_) => "internal server error",
    }
  }

  fn status(&self) -> StatusCode {
    match self {
      Self::BadRequest(_) => StatusCode::BAD_REQUEST,
      Self::Conflict(_) => StatusCode::CONFLICT,
      Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
      Self::NotFound(_) => StatusCode::NOT_FOUND,
    }
  }
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      Self::Internal(error) => write!(f, "{error}"),
      _ => write!(f, "{}", self.public_message()),
    }
  }
}

impl From<AnyhowError> for Error {
  fn from(error: AnyhowError) -> Self {
    Self::Internal(error)
  }
}

impl IntoResponse for Error {
  fn into_response(self) -> AxumResponse {
    let status = self.status();

    if let Self::Internal(error) = &self {
      warn!(%error, "internal request error");
    }

    (
      status,
      Json(ErrorBody {
        error: self.public_message().to_owned(),
        status: status.as_u16(),
      }),
    )
      .into_response()
  }
}

impl StdError for Error {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      Self::Internal(error) => Some(error.as_ref()),
      _ => None,
    }
  }
}
