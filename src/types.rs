use derive_deref::Deref;
use derive_more::Constructor;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameData {
    pub name: String,
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

#[derive(Deserialize_repr, Serialize_repr, Debug, Clone, PartialEq)]
#[repr(u32)]
#[serde(deny_unknown_fields)]
pub enum Unit {
    Unit = 0,
    Gram = 1,
    Ml = 2,
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

#[derive(Debug, Constructor)]
pub struct ProductItemWeight {
    pub id: u32,
    pub sort_weight: f32,
}

#[derive(Debug, Constructor)]
pub struct AisleItemWeight {
    pub id: u32,
    pub sort_weight: f32,
}

#[derive(Debug, Constructor)]
pub struct EditWeight {
    sections: Vec<AisleItemWeight>,
    products: Vec<ProductItemWeight>,
}
