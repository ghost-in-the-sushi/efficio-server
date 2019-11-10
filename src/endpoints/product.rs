use crate::db;
use crate::endpoints::INVALID_PARAMS;
use crate::error::*;
use crate::types::*;

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub fn create_product(
    auth: String,
    aisle_id: String,
    data: &NameData,
    c: &mut Connection,
) -> Result<Product> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::products::save_product(c, &auth, &data.name, &AisleId(aisle_id))
}

pub fn edit_product(
    auth: String,
    product_id: String,
    data: &EditProduct,
    c: &mut Connection,
) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    if !data.has_at_least_a_field() {
        Err(ServerError::new(
            INVALID_PARAMS,
            "At least a field must be present",
        ))
    } else {
        db::products::modify_product(c, &auth, &data, &ProductId(product_id))
    }
}

pub fn delete_product(auth: String, product_id: String, c: &mut Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::products::delete_product(c, &auth, &ProductId(product_id))
}
