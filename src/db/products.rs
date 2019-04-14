use derive_more::Constructor;
use redis::{self, Commands, PipelineCommands};
use serde::Serialize;

use crate::db;
use crate::error::*;
use crate::types::*;
#[derive(Serialize, Constructor)]
pub struct Product {
  product_id: u32,
  name: String,
  quantity: u32,
  is_done: bool,
  sort_weight: f32,
}

const NEXT_PROD_ID: &str = "next_product_id";
const PROD_NAME: &str = "name";
const PROD_SORT_WEIGHT: &str = "sort_weight";
const PROD_STATE: &str = "state";
const PROD_OWNER: &str = "product_owner";
const PROD_QTY: &str = "quantity";

fn product_key(id: &ProductId) -> String {
  format!("product:{}", **id)
}

fn products_in_aisle_key(id: &AisleId) -> String {
  format!("products_in_aisle:{}", **id)
}

fn get_product_owner(c: &redis::Connection, id: &ProductId) -> Result<UserId> {
  Ok(UserId(c.hget(&product_key(&id), PROD_OWNER)?))
}

pub fn save_product(auth: &Auth, name: &str, aisle_id: &AisleId) -> Result<Product> {
  let c = db::get_connection()?;
  let aisle_owner = db::aisles::get_aisle_owner(&c, &aisle_id)?;
  let user_id = db::sessions::get_user_id(&c, &auth)?;
  db::verify_permission(&user_id, &aisle_owner)?;
  let prod_id = ProductId(c.incr(NEXT_PROD_ID, 1)?);
  let prod_key = product_key(&prod_id);
  let prod_in_aisle_key = products_in_aisle_key(&aisle_id);
  redis::transaction(&c, &[&prod_key, &prod_in_aisle_key], |pipe| {
    pipe
      .hset(&prod_key, PROD_NAME, name)
      .ignore()
      .hset(&prod_key, PROD_QTY, 1)
      .ignore()
      .hset(&prod_key, PROD_SORT_WEIGHT, 0f32)
      .ignore()
      .hset(&prod_key, PROD_STATE, false)
      .ignore()
      .hset(&prod_key, PROD_OWNER, *user_id)
      .ignore()
      .sadd(&prod_in_aisle_key, *prod_id)
      .query(&c)
  })?;
  Ok(Product::new(*prod_id, name.to_owned(), 1, false, 0.0f32))
}

pub fn rename_product(auth: &Auth, new_name: &str, product_id: &ProductId) -> Result<()> {
  let c = db::get_connection()?;
  let product_owner = get_product_owner(&c, &product_id)?;
  db::verify_permission_auth(&c, &auth, &product_owner)?;
  Ok(c.hset(&product_key(&product_id), PROD_NAME, new_name)?)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db;
  use crate::db::sessions::tests::*;

  const NAME: &str = "product1";
  const RENAME: &str = "product2";

  fn save_product_test() {
    db::users::tests::store_user_for_test_with_reset();
    db::sessions::tests::store_session_for_test(&AUTH);
    let store_id = db::stores::save_store(&AUTH, "MyStore").unwrap();
    db::aisles::save_aisle(&AUTH, &store_id, db::aisles::tests::NAME).unwrap();
    assert_eq!(true, save_product(&AUTH, NAME, &AisleId(1)).is_ok());
    let c = db::get_connection().unwrap();
    let prod_key = product_key(&ProductId(1));
    let name: String = c.hget(&prod_key, PROD_NAME).unwrap();
    assert_eq!(NAME, &name);
    let qty: u32 = c.hget(&prod_key, PROD_QTY).unwrap();
    assert_eq!(1, qty);
    let sort: f32 = c.hget(&prod_key, PROD_SORT_WEIGHT).unwrap();
    assert!(sort - 0f32 < std::f32::EPSILON);
    let is_done: bool = c.hget(&prod_key, PROD_STATE).unwrap();
    assert_ne!(false, is_done);
    let owner: u32 = c.hget(&prod_key, PROD_OWNER).unwrap();
    assert_eq!(1, owner);
    let res: bool = c.sismember(&products_in_aisle_key(&AisleId(1)), 1).unwrap();
    assert_eq!(true, res);
  }

  #[test]
  fn rename_product_test() {
    save_product_test();
    assert_eq!(true, rename_product(&AUTH, RENAME, &ProductId(1)).is_ok());
    let c = db::get_connection().unwrap();
    let name: String = c.hget(&product_key(&ProductId(1)), PROD_NAME).unwrap();
    assert_eq!(RENAME, &name);
  }
}
