use std::error::Error;
use std::fmt::Display;

use r2d2;
use redis::RedisError;
use serde::Serialize;
use thiserror::Error;
use warp::http::StatusCode;

pub const USERNAME_TAKEN: StatusCode = StatusCode::NOT_ACCEPTABLE;
pub const INVALID_USER_OR_PWD: StatusCode = StatusCode::BAD_REQUEST;
pub const UNAUTHORISED: StatusCode = StatusCode::UNAUTHORIZED;
pub const PERMISSION_DENIED: StatusCode = StatusCode::FORBIDDEN;
pub const INTERNAL_ERROR: StatusCode = StatusCode::INTERNAL_SERVER_ERROR;

#[derive(Debug, Clone, Error, Serialize, PartialEq)]
pub struct ServerError {
    #[serde(skip)]
    pub status: StatusCode,
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
            status: INTERNAL_ERROR,
            msg: err.description().to_string(),
        }
    }
}

impl From<ServerError> for RedisError {
    fn from(err: ServerError) -> Self {
        (redis::ErrorKind::ExtensionError, "", err.msg).into()
    }
}

impl From<r2d2::Error> for ServerError {
    fn from(err: r2d2::Error) -> Self {
        ServerError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            msg: err.description().to_string(),
        }
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;

impl ServerError {
    pub fn new(status: StatusCode, msg: &str) -> Self {
        ServerError {
            status,
            msg: msg.to_owned(),
        }
    }
}
