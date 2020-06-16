use crate::{error::Result, types::*, db};

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub async fn create_aisle(
    auth: String,
    store_id: String,
    data: &NameData,
    c: &mut Connection,
) -> Result<Aisle> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::aisles::save_aisle(c, &auth, &StoreId::new(store_id), &data.name)
}

pub async fn rename_aisle(
    auth: String,
    aisle_id: String,
    data: &NameData,
    c: &mut Connection,
) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::aisles::edit_aisle(c, &auth, &AisleId(aisle_id), &data.name)
}

pub async fn delete_aisle(auth: String, aisle_id: String, c: &mut Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::aisles::delete_aisle(c, &auth, &AisleId(aisle_id))
}
