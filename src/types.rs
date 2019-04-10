use derive_deref::Deref;
use derive_new::new;
use serde::Serialize;

#[derive(Deref)]
pub struct Auth<'a>(pub &'a str);

#[derive(Debug, Deref)]
pub struct UserId(pub u32);

#[derive(Serialize, Debug, Deref, new)]
pub struct StoreId {
  store_id: u32,
}

#[derive(Serialize, Debug, Deref, new)]
pub struct SectionId {
  section_id: u32,
}
