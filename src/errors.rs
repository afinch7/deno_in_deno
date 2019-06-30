use serde::{Serialize};

pub type TsDenoResult<T> = std::result::Result<T, TsDenoError>;

#[derive(Serialize)]
pub struct TsDenoError {
  message: String,
}

pub fn new_error(message: &str) -> TsDenoError {
  TsDenoError {
    message: message.to_string(),
  }
}