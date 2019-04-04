use redis::{self, Commands};

use crate::db::get_connection;
use crate::error::ServerError;

const SESSIONS_LIST: &str = "sessions";

pub fn store_session(auth: &str, user_id: u32) -> Result<(), ServerError> {
  let c = get_connection()?;
  c.hset(SESSIONS_LIST, auth, user_id)?;
  Ok(())
}
