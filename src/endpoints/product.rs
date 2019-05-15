use crate::db;
use crate::endpoints::INVALID_PARAMS;
use crate::error::*;
use crate::types::*;

pub fn create_product(auth: String, aisle_id: u32, data: &NameData) -> Result<Product> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::products::save_product(&auth, &data.name, &AisleId(aisle_id))
}

pub fn edit_product(auth: String, product_id: u32, data: &EditProduct) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    if !data.has_at_least_a_field() {
        Err(ServerError::new(
            INVALID_PARAMS,
            "At least a field must be present",
        ))
    } else {
        db::products::modify_product(&auth, &data, &ProductId(product_id))
    }
}

pub fn delete_product(auth: String, product_id: u32) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::products::delete_product(&auth, &ProductId(product_id))
}
