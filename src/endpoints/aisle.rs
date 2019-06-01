use crate::db;
use crate::error::*;
use crate::types::*;

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub fn create_aisle(auth: String, store_id: u32, data: &NameData, c: &Connection) -> Result<Aisle> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    db::aisles::save_aisle(&c, &auth, &StoreId::new(store_id), &data.name)
}

pub fn rename_aisle(auth: String, aisle_id: u32, data: &NameData, c: &Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    db::aisles::edit_aisle(&c, &auth, &AisleId(aisle_id), &data.name)
}

pub fn delete_aisle(auth: String, aisle_id: u32, c: &Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    db::aisles::delete_aisle(&c, &auth, &AisleId(aisle_id))
}
