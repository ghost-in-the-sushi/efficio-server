use derive_deref::Deref;
use derive_more::Constructor;
use serde::Serialize;

#[derive(Deref, PartialEq, Eq)]
pub struct Auth<'a>(pub &'a str);

#[derive(Debug, Deref, PartialEq, Eq)]
pub struct UserId(pub u32);

#[derive(Serialize, Debug, Constructor, Deref, PartialEq, Eq)]
pub struct StoreId {
  store_id: u32,
}

#[derive(Debug, Deref, PartialEq, Eq)]
pub struct AisleId(pub u32);

#[derive(Debug, Deref, PartialEq, Eq)]
pub struct ProductId(pub u32);
