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

#[derive(Debug, Serialize, Constructor, PartialEq, Eq)]
pub struct StoreLight {
    name: String,
    store_id: u32,
}

#[derive(Debug, Serialize, Constructor, PartialEq, Eq)]
pub struct StoreLightList {
    stores: Vec<StoreLight>,
}

#[derive(Debug, Constructor, Serialize, PartialEq)]
pub struct Store {
    store_id: u32,
    name: String,
    aisles: Vec<Aisle>,
}

#[derive(Debug, Constructor, Serialize, PartialEq)]
pub struct Aisle {
    aisle_id: u32,
    name: String,
    sort_weight: f32,
    products: Vec<Product>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum Unit {
    Unit,
    Gram,
    Ml,
}

impl From<Unit> for u32 {
    fn from(o: Unit) -> u32 {
        match o {
            Unit::Unit => 0,
            Unit::Gram => 1,
            Unit::Ml => 2,
        }
    }
}

impl From<u32> for Unit {
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

#[derive(Debug, Serialize, Constructor, PartialEq)]
pub struct Product {
    product_id: u32,
    name: String,
    quantity: u32,
    is_done: bool,
    unit: Unit,
    sort_weight: f32,
}
