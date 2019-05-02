use std::cmp::Ordering;

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

#[derive(Debug, Constructor, Serialize)]
pub struct Aisle {
    aisle_id: u32,
    name: String,
    pub sort_weight: f32,
    products: Vec<Product>,
}

impl PartialEq for Aisle {
    fn eq(&self, other: &Aisle) -> bool {
        self.aisle_id == other.aisle_id && self.name == other.name
    }
}

impl Eq for Aisle {}

impl PartialOrd for Aisle {
    fn partial_cmp(&self, other: &Aisle) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Aisle {
    fn cmp(&self, other: &Aisle) -> Ordering {
        if (self.sort_weight - other.sort_weight).abs() < std::f32::EPSILON {
            self.name.cmp(&other.name)
        } else {
            if self.sort_weight < other.sort_weight {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    }
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

#[derive(Debug, Serialize, Constructor)]
pub struct Product {
    product_id: u32,
    name: String,
    quantity: u32,
    is_done: bool,
    unit: Unit,
    pub sort_weight: f32,
}

impl PartialEq for Product {
    fn eq(&self, other: &Product) -> bool {
        self.product_id == other.product_id && self.name == other.name
    }
}

impl Eq for Product {}

impl PartialOrd for Product {
    fn partial_cmp(&self, other: &Product) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Product {
    fn cmp(&self, other: &Product) -> Ordering {
        if (self.sort_weight - other.sort_weight).abs() < std::f32::EPSILON {
            self.name.cmp(&other.name)
        } else {
            if self.sort_weight < other.sort_weight {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
    }
}

#[derive(Debug, Constructor, Deserialize)]
pub struct ProductItemWeight {
    pub id: u32,
    pub sort_weight: f32,
}

#[derive(Debug, Constructor, Deserialize)]
pub struct AisleItemWeight {
    pub id: u32,
    pub sort_weight: f32,
}

#[derive(Debug, Constructor, Deserialize)]
pub struct EditWeight {
    pub aisles: Option<Vec<AisleItemWeight>>,
    pub products: Option<Vec<ProductItemWeight>>,
}

impl EditWeight {
    pub fn has_at_least_a_field(&self) -> bool {
        self.aisles.is_some() || self.products.is_some()
    }
}
