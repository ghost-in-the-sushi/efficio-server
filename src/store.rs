use std::collections::HashMap;

use crate::db;
use crate::error::*;
use crate::helpers;
use crate::types::*;

pub fn create_store(auth: String, obj: HashMap<String, String>) -> Result<StoreId> {
  let auth = Auth(&auth);
  db::sessions::validate_session(&auth)?;
  let name = helpers::extract_value(&obj, "name", "Missing name")?;
  db::stores::save_store(&auth, &name)
}

pub fn edit_store(auth: String, id: u32, obj: HashMap<String, String>) -> Result<()> {
  db::sessions::validate_session(&Auth(&auth))?;
  let name = helpers::extract_value(&obj, "name", "Missing name")?;
  db::stores::edit_store(&StoreId::new(id), &name)
}
