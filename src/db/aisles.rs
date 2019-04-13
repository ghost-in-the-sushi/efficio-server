use derive_new::new;
use redis::{self, Commands, PipelineCommands};
use serde::Serialize;

use crate::db::{self, get_connection};
use crate::error::{self, *};
use crate::types::*;

const NEXT_AISLE_ID: &str = "next_aisle_id";
const AISLE_NAME: &str = "name";
const AISLE_WEIGHT: &str = "sort_weight";
const AISLE_OWNER: &str = "owner_id";
const AISLE_STORE: &str = "store_id";

#[derive(new, Serialize)]
pub struct Aisle {
  aisle_id: u32,
  name: String,
  sort_weight: f32,
  // TODO product list
}

fn aisle_key(id: &AisleId) -> String {
  format!("aisle:{}", **id)
}

fn aisles_in_store(id: &StoreId) -> String {
  format!("aisles_in_store:{}", **id)
}

pub fn save_aisle(auth: &Auth, store_id: &StoreId, name: &str) -> Result<Aisle> {
  let c = get_connection()?;
  let aisle_id = AisleId(c.incr(NEXT_AISLE_ID, 1)?);
  let aisle_key = aisle_key(&aisle_id);
  let aisle_in_store_key = aisles_in_store(&store_id);
  let user_id = db::sessions::get_user_id(&c, &auth)?;
  redis::transaction(&c, &[&aisle_key, &aisle_in_store_key], |pipe| {
    pipe
      .hset(&aisle_key, AISLE_NAME, name)
      .ignore()
      .hset(&aisle_key, AISLE_WEIGHT, 0.0f32)
      .ignore()
      .hset(&aisle_key, AISLE_OWNER, *user_id)
      .ignore()
      .hset(&aisle_key, AISLE_STORE, **store_id)
      .ignore()
      .sadd(&aisle_in_store_key, **store_id)
      .query(&c)
  })?;

  Ok(Aisle::new(*aisle_id, name.to_owned(), 0.0))
}

pub fn edit_aisle(auth: &Auth, aisle_id: &AisleId, new_name: &str) -> Result<()> {
  let c = get_connection()?;
  let wanted_user_id = db::sessions::get_user_id(&c, &auth)?;
  let aisle_key = aisle_key(&aisle_id);
  let aisle_owner: u32 = c.hget(&aisle_key, AISLE_OWNER)?;
  if aisle_owner != *wanted_user_id {
    Err(ServerError::new(
      error::PERMISSION_DENIED,
      "User does not have permission to edit this resource",
    ))
  } else {
    Ok(c.hset(&aisle_key, AISLE_NAME, new_name)?)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::stores::*;
  use crate::db::tests::*;
  use crate::db::{sessions::tests::*, users::tests::*};

  const NAME: &str = "Aisle1";
  const RENAMED: &str = "AisleRenamed";

  fn save_aisle_test() {
    store_user_for_test_with_reset();
    store_session_for_test(&AUTH);
    let store_id = save_store(&AUTH, "MyStore").unwrap();
    assert_eq!(true, save_aisle(&AUTH, &store_id, NAME).is_ok());
    let c = get_connection().unwrap();
    let key = aisle_key(&AisleId(1));
    let res: bool = c.exists(&key).unwrap();
    assert_eq!(true, res);
    let res: bool = c.exists(&aisles_in_store(&store_id)).unwrap();
    assert_eq!(true, res);
    let name: String = c.hget(&key, AISLE_NAME).unwrap();
    assert_eq!(NAME, name.as_str());
    let weight: f32 = c.hget(&key, AISLE_WEIGHT).unwrap();
    assert!(weight - 0.0f32 < std::f32::EPSILON);
    let aisle_store: u32 = c.hget(&key, AISLE_STORE).unwrap();
    assert_eq!(1, aisle_store);
    let res: bool = c.sismember(&aisles_in_store(&store_id), 1).unwrap();
    assert_eq!(true, res);
  }

  #[test]
  fn edit_aisle_test() {
    save_aisle_test();
    assert_eq!(true, edit_aisle(&AUTH, &AisleId(1), RENAMED).is_ok());
    let c = get_connection().unwrap();
    let name: String = c.hget(&aisle_key(&AisleId(1)), AISLE_NAME).unwrap();
    assert_eq!(RENAMED, name.as_str());
  }
}
