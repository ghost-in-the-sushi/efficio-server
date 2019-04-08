use std::collections::HashMap;

use crate::db;
use crate::error::*;
use crate::helpers;
use crate::types::*;
use crate::error::*;

pub fn create_store(auth: String, obj: HashMap<String, String>) -> Result<StoreId> {
  db::sessions::validate_session(&auth)?;
  let name = helpers::extract_value(&obj, "name", "Missing name")?;
  if db::stores::store_exists(&name)? {
    Err(ServerError::new(
      error::STORENAME_TAKEN,
      "Store name exists for this user",
    ))
  } else {
    db::stores::save_store(&auth, &name)
  }
}
