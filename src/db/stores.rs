use redis::{Commands, PipelineCommands};
use std::string::ToString;

use crate::db::{get_connection, sessions};
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

pub fn save_store(auth: &Auth, name: &str) -> Result<StoreId> {
  let c = get_connection()?;
  let store_id = StoreId::new(c.incr(NEXT_STORE_ID, 1)?);
  let user_id = sessions::get_user_id(&c, &auth)?;
  let store_key = store_key(&store_id);
  let user_stores_key = user_stores_list_key(&user_id);
  redis::transaction(&c, &[&store_key, &user_stores_key], |pipe| {
    pipe
      .hset(&store_key, STORE_NAME, name)
      .ignore()
      .hset(&store_key, STORE_OWNER, *user_id)
      .ignore()
      .sadd(&user_stores_key, *store_id)
      .query(&c)
  })?;
  Ok(store_id)
}

pub fn edit_store(id: &StoreId, new_name: &str) -> Result<()> {
  let c = get_connection()?;
  Ok(c.hset(&store_key(&id), STORE_NAME, new_name)?)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::users::tests::*;
  use sessions::tests::*;

  const STORE_TEST_NAME: &str = "storetest";
  const NEW_STORE_NAME: &str = "new_store_name";

  #[test]
  fn save_store_test() {
    store_user_for_test_with_reset();
    store_session_for_test(&AUTH);
    assert_eq!(true, save_store(&AUTH, STORE_TEST_NAME).is_ok());
    let c = get_connection().unwrap();
    let store_key = store_key(&StoreId::new(1));
    let res: bool = c.exists(&store_key).unwrap();
    assert_eq!(true, res);
    let store_name: String = c.hget(&store_key, STORE_NAME).unwrap();
    assert_eq!(STORE_TEST_NAME, &store_name);
    let store_owner: u32 = c.hget(&store_key, STORE_OWNER).unwrap();
    assert_eq!(1, store_owner);
    let user_stores_list_key = user_stores_list_key(&UserId(1));
    let res: bool = c.exists(&user_stores_list_key).unwrap();
    assert_eq!(true, res);
    let res: bool = c.sismember(&user_stores_list_key, 1).unwrap();
    assert_eq!(true, res);
  }

  #[test]
  fn edit_store_test() {
    save_store_test();
    edit_store(&StoreId::new(1), NEW_STORE_NAME).unwrap();
    let c = get_connection().unwrap();
    let store_key = store_key(&StoreId::new(1));
    let store_name: String = c.hget(&store_key, STORE_NAME).unwrap();
    assert_eq!(NEW_STORE_NAME, &store_name);
  }
}
