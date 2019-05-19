use crate::db;
use crate::error::*;
use crate::types::*;

#[cfg(not(test))]
use redis::Client;

#[cfg(test)]
use fake_redis::FakeCient as Client;

pub fn create_aisle(
    auth: String,
    store_id: u32,
    data: &NameData,
    db_client: &Client,
) -> Result<Aisle> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::aisles::save_aisle(&c, &auth, &StoreId::new(store_id), &data.name)
}

pub fn rename_aisle(
    auth: String,
    aisle_id: u32,
    data: &NameData,
    db_client: &Client,
) -> Result<()> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::aisles::edit_aisle(&c, &auth, &AisleId(aisle_id), &data.name)
}

pub fn delete_aisle(auth: String, aisle_id: u32, db_client: &Client) -> Result<()> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::aisles::delete_aisle(&c, &auth, &AisleId(aisle_id))
}