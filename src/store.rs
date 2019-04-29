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
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    let name = helpers::extract_value(&obj, "name", "Missing name")?;
    db::stores::edit_store(&auth, &StoreId::new(id), &name)
}

pub fn list_stores(auth: String) -> Result<StoreLightList> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    Ok(StoreLightList::new(db::stores::get_all_stores(&auth)?))
}

pub fn list_store(auth: String, store_id: u32) -> Result<Store> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::stores::list_store(&auth, &StoreId::new(store_id))
}

pub fn delete_store(auth: String, store_id: u32) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::stores::delete_store(&auth, &StoreId::new(store_id))
}
