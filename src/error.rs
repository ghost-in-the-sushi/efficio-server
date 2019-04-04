use failure::Fail;
use redis::RedisError;
use serde::Serialize;
use std::error::Error as StdError;
use std::fmt::Display;

pub const USERNAME_TAKEN: i32 = 100;
pub const INVALID_PARAMS: i32 = 2;

#[derive(Debug, Clone, Fail, Serialize)]
pub struct ServerError {
  pub status: i32,
  pub msg: String,
}

impl Display for ServerError {
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl From<RedisError> for ServerError {
  fn from(err: RedisError) -> Self {
    ServerError {
      status: -1,
      msg: err.description().to_string(),
    }
  }
}
