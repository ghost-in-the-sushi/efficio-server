use lazy_static::lazy_static;
use redis::{self, Client, Connection};

pub mod user;

pub use user::store_user;

lazy_static! {
  static ref DB_CLIENT: Client = get_client();
}

fn get_client() -> Client {
  Client::open("redis://127.0.0.1:6379/").expect("Error while creating redis client.")
}

pub fn get_connection() -> redis::RedisResult<Connection> {
  DB_CLIENT.get_connection()
}
