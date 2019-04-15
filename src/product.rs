use std::collections::HashMap;
use std::str::FromStr;

use crate::db;
use crate::error::*;
use crate::helpers::*;
use crate::types::*;

pub fn create_product(
  auth: String,
  aisle_id: u32,
  obj: HashMap<String, String>,
) -> Result<db::products::Product> {
  let auth = Auth(&auth);
  db::sessions::validate_session(&auth)?;
  let name = extract_value(&obj, "name", "Missing `name` field.")?;
  db::products::save_product(&auth, &name, &AisleId(aisle_id))
}

pub fn edit_product(auth: String, product_id: u32, obj: HashMap<String, String>) -> Result<()> {
  let auth = Auth(&auth);
  db::sessions::validate_session(&auth)?;
  let data = db::products::EditProduct::new(
    obj.get("name").and_then(|s| Some(s.as_str())),
    obj.get("quantity").and_then(|s| s.parse::<u32>().ok()),
    obj.get("unit").and_then(|s| {
      if let Some(u) = s.parse::<u32>().ok() {
        Some(db::products::Unit::from(u))
      } else {
        None
      }
    }),
    obj.get("is_done").and_then(|s| bool::from_str(s).ok()),
  );
  db::products::modify_product(&auth, &data, &ProductId(product_id))
}
