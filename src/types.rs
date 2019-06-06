use std::cmp::Ordering;
use std::str::FromStr;
use std::string::ToString;

use derive_deref::Deref;
use derive_new::new;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::error;

#[derive(Deref, PartialEq, Eq)]
pub struct Auth<'a>(pub &'a str);

#[derive(Deserialize, Debug)]
pub struct AuthInfo {
    pub username: String,
    pub password: String,
}

impl Drop for AuthInfo {
    fn drop(&mut self) {
        self.password.replace_range(..self.password.len(), "0");
    }
}

#[derive(Debug, Serialize, Deserialize, new)]
pub struct ConnectionToken {
    pub session_token: String,
    pub user_id: String,
}

#[derive(Default, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub email: String,
    pub password: String,
}

impl Drop for User {
    fn drop(&mut self) {
        self.password.replace_range(..self.password.len(), "0");
        self.email.replace_range(..self.email.len(), "0");
    }
}

#[derive(Debug, Deref, PartialEq, Eq)]
pub struct UserId(pub String);

impl ToString for UserId {
    fn to_string(&self) -> String {
        self.0.to_owned()
    }
}

impl FromStr for UserId {
    type Err = error::ServerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(UserId(s.to_owned()))
    }
}

#[derive(Serialize, Debug, new, Deref, PartialEq, Eq)]
pub struct StoreId {
    store_id: String,
}

impl ToString for StoreId {
    fn to_string(&self) -> String {
        self.store_id.to_owned()
    }
}
impl FromStr for StoreId {
    type Err = error::ServerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(StoreId::new(s.to_owned()))
    }
}

#[derive(Debug, Deref, PartialEq, Eq)]
pub struct AisleId(pub String);

impl ToString for AisleId {
    fn to_string(&self) -> String {
        self.0.to_owned()
    }
}

impl FromStr for AisleId {
    type Err = error::ServerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(AisleId(s.to_owned()))
    }
}

#[derive(Debug, Deref, PartialEq, Eq)]
pub struct ProductId(pub String);

impl ToString for ProductId {
    fn to_string(&self) -> String {
        self.0.to_owned()
    }
}

impl FromStr for ProductId {
    type Err = error::ServerError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ProductId(s.to_owned()))
    }
}

#[derive(Debug, Serialize, new, PartialEq, Eq)]
pub struct StoreLight {
    name: String,
    store_id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NameData {
    pub name: String,
}

#[derive(Debug, Serialize, new, PartialEq, Eq)]
pub struct StoreLightList {
    stores: Vec<StoreLight>,
}

#[derive(Debug, new, Serialize, PartialEq)]
pub struct Store {
    store_id: String,
    name: String,
    aisles: Vec<Aisle>,
}

#[derive(Debug, new, Serialize)]
pub struct Aisle {
    aisle_id: String,
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
        } else if self.sort_weight < other.sort_weight {
            Ordering::Less
        } else {
            Ordering::Greater
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

#[derive(Debug, Serialize, new)]
pub struct Product {
    product_id: String,
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
        } else if self.sort_weight < other.sort_weight {
            Ordering::Less
        } else {
            Ordering::Greater
        }
    }
}

#[derive(Debug, new, Deserialize)]
pub struct ProductItemWeight {
    pub id: String,
    pub sort_weight: f32,
}

#[derive(Debug, new, Deserialize)]
pub struct AisleItemWeight {
    pub id: String,
    pub sort_weight: f32,
}

#[derive(Debug, new, Deserialize)]
pub struct EditWeight {
    pub aisles: Option<Vec<AisleItemWeight>>,
    pub products: Option<Vec<ProductItemWeight>>,
}

impl EditWeight {
    pub fn has_at_least_a_field(&self) -> bool {
        match (&self.aisles, &self.products) {
            (None, None) => false,
            (Some(aisles), None) => !aisles.is_empty(),
            (None, Some(products)) => !products.is_empty(),
            (Some(aisles), Some(products)) => !aisles.is_empty() || !products.is_empty(),
        }
    }
}

#[derive(new, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EditProduct {
    pub name: Option<String>,
    pub quantity: Option<u32>,
    pub unit: Option<Unit>,
    pub is_done: Option<bool>,
}

impl EditProduct {
    pub fn has_at_least_a_field(&self) -> bool {
        self.name.is_some()
            || self.quantity.is_some()
            || self.unit.is_some()
            || self.is_done.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::salts::tests::*;

    #[test]
    fn test_edit_product_has_as_least_a_field() {
        let e = EditProduct::new(None, None, None, None);
        assert_eq!(false, e.has_at_least_a_field());
        let e = EditProduct::new(Some("Toto".to_owned()), None, None, None);
        assert_eq!(true, e.has_at_least_a_field());
        let e = EditProduct::new(None, Some(1), None, None);
        assert_eq!(true, e.has_at_least_a_field());
        let e = EditProduct::new(None, None, Some(Unit::Unit), None);
        assert_eq!(true, e.has_at_least_a_field());
        let e = EditProduct::new(None, None, None, Some(true));
        assert_eq!(true, e.has_at_least_a_field());
    }

    #[test]
    fn test_edit_weight_has_as_least_a_field() {
        let e = EditWeight::new(None, None);
        assert_eq!(false, e.has_at_least_a_field());
        let e = EditWeight::new(Some(vec![]), None);
        assert_eq!(false, e.has_at_least_a_field());
        let e = EditWeight::new(None, Some(vec![]));
        assert_eq!(false, e.has_at_least_a_field());
        let e = EditWeight::new(
            Some(vec![AisleItemWeight::new(HASH_1.to_owned(), 1.0)]),
            None,
        );
        assert_eq!(true, e.has_at_least_a_field());
        let e = EditWeight::new(
            None,
            Some(vec![ProductItemWeight::new(HASH_1.to_owned(), 1.0)]),
        );
        assert_eq!(true, e.has_at_least_a_field());
    }
}
