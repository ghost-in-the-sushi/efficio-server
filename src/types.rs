use serde::Serialize;
use std::ops::Deref;

macro_rules! impl_deref {
  ($s:ty, $t:ty) => {
    impl Deref for $s {
      type Target = $t;
      fn deref(&self) -> &$t {
        &self.0
      }
    }
  };
}

pub struct UserId(pub u32);

#[derive(Serialize)]
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

impl_deref!(UserId, u32);
