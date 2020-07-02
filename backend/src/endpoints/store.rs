use crate::{db, error::Result, types::*};

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub async fn create_store(auth: String, data: &NameData, c: &mut Connection) -> Result<StoreId> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::stores::save_store(c, &auth, &data.name)
}

pub async fn edit_store(
    auth: String,
    id: String,
    data: &NameData,
    c: &mut Connection,
) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::stores::edit_store(c, &auth, &StoreId::new(id), &data.name)
}

pub async fn list_stores(auth: String, c: &mut Connection) -> Result<StoreLightList> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    Ok(StoreLightList::new(db::stores::get_all_stores(c, &auth)?))
}

pub async fn list_store(auth: String, store_id: String, c: &mut Connection) -> Result<Store> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::stores::list_store(c, &auth, &StoreId::new(store_id))
}

pub async fn delete_store(auth: String, store_id: String, c: &mut Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::stores::delete_store(c, &auth, &StoreId::new(store_id))
}
