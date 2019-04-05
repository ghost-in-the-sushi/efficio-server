use redis::{self, Commands};

use crate::db::get_connection;
use crate::error::{self, Result, ServerError};

const SESSIONS_LIST: &str = "sessions";

pub fn store_session(auth: &str, user_id: u32) -> Result<()> {
  let c = get_connection()?;
  c.hset(SESSIONS_LIST, auth, user_id)?;
  Ok(())
}

pub fn validate_session(c: &redis::Connection, auth: &str) -> Result<()> {
  if c.hexists(SESSIONS_LIST, auth)? {
    Ok(())
  } else {
    Err(ServerError {
      status: error::UNAUTHORISED,
      msg: "Not logged in".to_string(),
    })
  }
}

pub fn del_session(auth: &str) -> Result<()> {
  let c = get_connection()?;
  validate_session(&c, auth)?;
  c.hdel(SESSIONS_LIST, auth)?;
  Ok(())
}
