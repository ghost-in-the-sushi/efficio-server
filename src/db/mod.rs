use lazy_static::lazy_static;
use redis::{self, Client, Connection};

pub mod sessions;
pub mod stores;
pub mod users;

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

#[cfg(test)]
mod tests {
  use super::*;

  pub fn reset_db() {
    let c = get_connection().expect("should have connection");
    let _: () = redis::cmd("FLUSHDB").query(&c).expect("error on flush");
  }
}
