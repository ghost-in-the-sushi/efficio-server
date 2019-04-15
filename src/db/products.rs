use std::str::FromStr;

use derive_more::Constructor;
use redis::{self, Commands, PipelineCommands};
use serde::Serialize;

use crate::db;
use crate::error::*;
use crate::types::*;

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum Unit {
  Unit,
  Gram,
  Ml,
}

impl std::convert::From<Unit> for u32 {
  fn from(o: Unit) -> u32 {
    match o {
      Unit::Unit => 0,
      Unit::Gram => 1,
      Unit::Ml => 2,
    }
  }
}

impl std::convert::From<u32> for Unit {
  fn from(o: u32) -> Self {
    if o == 1 {
      Unit::Gram
    } else if o == 2 {
      Unit::Ml
    } else {
      Unit::Unit
    }
  }
}

#[derive(Serialize, Constructor)]
pub struct Product {
  product_id: u32,
  name: String,
  quantity: u32,
  is_done: bool,
  unit: Unit,
  sort_weight: f32,
}

#[derive(Constructor)]
pub struct EditProduct<'a> {
  name: Option<&'a str>,
  quantity: Option<u32>,
  unit: Option<Unit>,
  is_done: Option<bool>,
}

const NEXT_PROD_ID: &str = "next_product_id";
const PROD_NAME: &str = "name";
const PROD_SORT_WEIGHT: &str = "sort_weight";
const PROD_STATE: &str = "is_done";
const PROD_OWNER: &str = "product_owner";
const PROD_QTY: &str = "quantity";
const PROD_UNIT: &str = "unit";

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
      .hset(&prod_key, PROD_UNIT, u32::from(Unit::Unit))
      .ignore()
      .sadd(&prod_in_aisle_key, *prod_id)
      .query(&c)
  })?;
  Ok(Product::new(
    *prod_id,
    name.to_owned(),
    1,
    false,
    Unit::Unit,
    0.0f32,
  ))
}

pub fn modify_product(auth: &Auth, edit_data: &EditProduct, product_id: &ProductId) -> Result<()> {
  let c = db::get_connection()?;
  let product_owner = get_product_owner(&c, &product_id)?;
  db::verify_permission_auth(&c, &auth, &product_owner)?;
  let product_key = product_key(&product_id);
  if let Some(new_name) = edit_data.name {
    c.hset(&product_key, PROD_NAME, new_name)?;
  }
  if let Some(qty) = edit_data.quantity {
    c.hset(&product_key, PROD_QTY, qty)?;
  }
  if let Some(is_done) = edit_data.is_done {
    c.hset(&product_key, PROD_STATE, is_done)?;
  }
  if let Some(unit) = &edit_data.unit {
    c.hset(&product_key, PROD_UNIT, u32::from(unit.clone()))?;
  }
  Ok(())
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
    let is_done: String = c.hget(&prod_key, PROD_STATE).unwrap();
    let is_done: bool = bool::from_str(&is_done).unwrap();
    assert_eq!(false, is_done);
    let owner: u32 = c.hget(&prod_key, PROD_OWNER).unwrap();
    assert_eq!(1, owner);
    let res: bool = c.sismember(&products_in_aisle_key(&AisleId(1)), 1).unwrap();
    assert_eq!(true, res);
  }

  #[test]
  fn modify_product_test() {
    save_product_test();
    let data = EditProduct::new(Some(RENAME), Some(2), None, Some(true));
    assert_eq!(true, modify_product(&AUTH, &data, &ProductId(1)).is_ok());
    let c = db::get_connection().unwrap();
    let product_key = product_key(&ProductId(1));
    let name: String = c.hget(&product_key, PROD_NAME).unwrap();
    assert_eq!(RENAME, &name);
    let qty: u32 = c.hget(&product_key, PROD_QTY).unwrap();
    assert_eq!(2, qty);
    let unit: u32 = c.hget(&product_key, PROD_UNIT).unwrap();
    let unit = Unit::from(unit);
    assert_eq!(Unit::Unit, unit);
    let state: String = c.hget(&product_key, PROD_STATE).unwrap();
    let state: bool = bool::from_str(&state).unwrap();
    assert_eq!(true, state);
  }
}
