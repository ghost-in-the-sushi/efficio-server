use crate::db;
use crate::error::*;
use crate::types::*;

pub fn create_product(auth: String, aisle_id: u32, data: &NameData) -> Result<Product> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::products::save_product(&auth, &data.name, &AisleId(aisle_id))
}

pub fn edit_product(auth: String, product_id: u32, data: &db::products::EditProduct) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    // let data = db::products::EditProduct::new(
    //     obj.get("name").and_then(|s| Some(s.as_str())),
    //     obj.get("quantity").and_then(|s| s.parse::<u32>().ok()),
    //     obj.get("unit").and_then(|s| {
    //         if let Some(u) = s.parse::<u32>().ok() {
    //             Some(Unit::from(u))
    //         } else {
    //             None
    //         }
    //     }),
    //     obj.get("is_done").and_then(|s| bool::from_str(s).ok()),
    // );

    if !data.has_at_last_a_field() {
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
