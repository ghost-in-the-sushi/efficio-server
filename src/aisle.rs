use crate::db;
use crate::error::*;
use crate::types::*;

pub fn create_aisle(auth: String, store_id: u32, data: &NameData) -> Result<Aisle> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::aisles::save_aisle(&auth, &StoreId::new(store_id), &data.name)
}

pub fn rename_aisle(auth: String, aisle_id: u32, data: &NameData) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::aisles::edit_aisle(&auth, &AisleId(aisle_id), &data.name)
}

pub fn delete_aisle(auth: String, aisle_id: u32) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::aisles::delete_aisle(&auth, &AisleId(aisle_id))
}
