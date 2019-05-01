use crate::db;
use crate::error::*;
use crate::types::*;

pub fn create_store(auth: String, data: &NameData) -> Result<StoreId> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::stores::save_store(&auth, &data.name)
}

pub fn edit_store(auth: String, id: u32, data: &NameData) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::stores::edit_store(&auth, &StoreId::new(id), &data.name)
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
