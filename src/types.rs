use serde::Serialize;
use std::ops::Deref;

#[derive(Debug)]
pub struct UserId(pub u32);

impl Deref for UserId {
  type Target = u32;
  fn deref(&self) -> &u32 {
    &self.0
  }
}

#[derive(Serialize, Debug)]
pub struct StoreId {
  store_id: u32,
}

impl StoreId {
  pub fn new(id: u32) -> Self {
    StoreId { store_id: id }
  }
}

impl Deref for StoreId {
  type Target = u32;
  fn deref(&self) -> &u32 {
    &self.store_id
  }
}

pub struct Auth<'a>(pub &'a str);
impl<'a> Deref for Auth<'a> {
  type Target = str;
  fn deref(&self) -> &str {
    &self.0
  }
}
