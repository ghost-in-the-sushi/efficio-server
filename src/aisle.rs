use std::collections::HashMap;

use crate::db;
use crate::error::*;
use crate::helpers::*;
use crate::types::*;

pub fn create_aisle(auth: String, store_id: u32, obj: HashMap<String, String>) -> Result<Aisle> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    let name = extract_value(&obj, "name", "Missing `name` field.")?;
    db::aisles::save_aisle(&auth, &StoreId::new(store_id), &name)
}

pub fn rename_aisle(auth: String, aisle_id: u32, obj: HashMap<String, String>) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    let new_name = extract_value(&obj, "name", "Missing `name` field.")?;
    db::aisles::edit_aisle(&auth, &AisleId(aisle_id), &new_name)
}

pub fn delete_aisle(auth: String, aisle_id: u32) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::aisles::delete_aisle(&auth, &AisleId(aisle_id))
}
