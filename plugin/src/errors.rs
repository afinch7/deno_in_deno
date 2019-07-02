use serde::{Serialize};

pub type DIDResult<T> = std::result::Result<T, DIDError>;

#[derive(Serialize)]
pub struct DIDError {
  message: String,
}

pub fn new_error(message: &str) -> DIDError {
  DIDError {
    message: message.to_string(),
  }
}