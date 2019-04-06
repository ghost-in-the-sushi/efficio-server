use failure::Fail;
use redis::RedisError;
use serde::Serialize;
use std::error::Error;
use std::fmt::Display;

pub const USERNAME_TAKEN: i32 = 100;
pub const INVALID_USER_OR_PWD: i32 = 150;
pub const UNAUTHORISED: i32 = 200;
pub const INVALID_PARAMS: i32 = 2;
pub const INTERNAL_ERROR: i32 = 500;

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

pub type Result<T> = std::result::Result<T, ServerError>;
