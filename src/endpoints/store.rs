use crate::db;
use crate::error::*;
use crate::types::*;

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub fn create_store(auth: String, data: &NameData, c: &Connection) -> Result<StoreId> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    db::stores::save_store(&c, &auth, &data.name)
}

pub fn edit_store(auth: String, id: u32, data: &NameData, c: &Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    db::stores::edit_store(&c, &auth, &StoreId::new(id), &data.name)
}

pub fn list_stores(auth: String, c: &Connection) -> Result<StoreLightList> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    Ok(StoreLightList::new(db::stores::get_all_stores(&c, &auth)?))
}

pub fn list_store(auth: String, store_id: u32, c: &Connection) -> Result<Store> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    db::stores::list_store(&c, &auth, &StoreId::new(store_id))
}

pub fn delete_store(auth: String, store_id: u32, c: &Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&c, &auth)?;
    db::stores::delete_store(&c, &auth, &StoreId::new(store_id))
}
