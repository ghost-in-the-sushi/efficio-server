use redis::{Commands, PipelineCommands};
use std::string::ToString;

use crate::db::{get_connection, sessions, users};
use crate::error::*;
use crate::types::*;

const NEXT_STORE_ID: &str = "next_store_id";
const STORE_NAME: &str = "name";
const STORE_OWNER: &str = "owner_id";

fn store_key(id: &StoreId) -> String {
  format!("store:{}", id.to_string())
}

fn user_stores_list_key(user_id: &UserId) -> String {
  format!("stores:{}", user_id.to_string())
}

pub fn save_store(name: &str, auth: &str) -> Result<StoreId> {
  let c = get_connection()?;
  let store_id = StoreId(c.incr(NEXT_STORE_ID, 1)?);
  let user_id = sessions::get_user_id(&c, &auth)?;
  let store_key = store_key(&store_id);
  let user_stores_key = user_stores_list_key(&user_id);
  redis::transaction(&c, &[&store_key, &user_stores_key], |pipe| {
    pipe
      .hset(&store_key, STORE_NAME, name)
      .ignore()
      .hset(&store_key, STORE_OWNER, user_id.0)
      .ignore()
      .sadd(&user_stores_key, store_id.0)
      .query(&c)
  })?;
  Ok(store_id)
}
