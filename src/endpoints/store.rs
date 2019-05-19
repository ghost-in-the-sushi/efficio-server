use crate::db;
use crate::error::*;
use crate::types::*;

#[cfg(not(test))]
use redis::Client;

#[cfg(test)]
use fake_redis::FakeCient as Client;

pub fn create_store(auth: String, data: &NameData, db_client: &Client) -> Result<StoreId> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::stores::save_store(&c, &auth, &data.name)
}

pub fn edit_store(auth: String, id: u32, data: &NameData, db_client: &Client) -> Result<()> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::stores::edit_store(&c, &auth, &StoreId::new(id), &data.name)
}

pub fn list_stores(auth: String, db_client: &Client) -> Result<StoreLightList> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    Ok(StoreLightList::new(db::stores::get_all_stores(&c, &auth)?))
}

pub fn list_store(auth: String, store_id: u32, db_client: &Client) -> Result<Store> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::stores::list_store(&c, &auth, &StoreId::new(store_id))
}

pub fn delete_store(auth: String, store_id: u32, db_client: &Client) -> Result<()> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::stores::delete_store(&c, &auth, &StoreId::new(store_id))
}
