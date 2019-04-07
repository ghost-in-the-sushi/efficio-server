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
pub struct StoreId(pub u32);

impl_deref!(UserId, u32);
impl_deref!(StoreId, u32);
