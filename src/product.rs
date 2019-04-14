use std::collections::HashMap;

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

pub fn rename_product(auth: String, product_id: u32, obj: HashMap<String, String>) -> Result<()> {
  let auth = Auth(&auth);
  db::sessions::validate_session(&auth)?;
  let name = extract_value(&obj, "name", "Missing `name` field.")?;
  db::products::rename_product(&auth, &name, &ProductId(product_id))
}
