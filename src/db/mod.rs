use lazy_static::lazy_static;
use redis::{self, Client, Connection};

pub mod aisles;
pub mod products;
pub mod sessions;
pub mod stores;
pub mod users;

use crate::error::*;
use crate::types::*;
pub use users::save_user;

#[cfg(debug_assertions)]
const SERVER_ADDR: &str = "redis://127.0.0.1:6379/0";

#[cfg(not(debug_assertions))]
const SERVER_ADDR: &str = "redis://127.0.0.1:6379/8";

lazy_static! {
  static ref DB_CLIENT: Client = get_client();
}

fn get_client() -> Client {
  Client::open(SERVER_ADDR).expect("Error while creating redis client.")
}

pub fn get_connection() -> redis::RedisResult<Connection> {
  DB_CLIENT.get_connection()
}

pub(crate) fn verify_permission(wanted_user_id: &UserId, user_id: &UserId) -> Result<()> {
  if wanted_user_id != user_id {
    Err(ServerError::new(
      PERMISSION_DENIED,
      "User does not have permission to edit this resource",
    ))
  } else {
    Ok(())
  }
}

pub(crate) fn verify_permission_auth(c: &Connection, auth: &Auth, user_id: &UserId) -> Result<()> {
  let wanted_user_id = sessions::get_user_id(&c, &auth)?;
  verify_permission(&wanted_user_id, &user_id)
}

#[cfg(test)]
mod tests {
  use super::*;

  pub fn reset_db() {
    let c = get_connection().expect("should have connection");
    let _: () = redis::cmd("FLUSHDB").query(&c).expect("error on flush");
  }
}
