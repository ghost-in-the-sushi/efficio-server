use std::convert::From;

#[cfg(not(test))]
use redis::{self, transaction, Commands, Connection, Pipeline, PipelineCommands};

#[cfg(test)]
use fake_redis::{transaction, FakeConnection as Connection, FakePipeline as Pipeline};

use crate::db;
use crate::error::*;
use crate::types::*;

const NEXT_PROD_ID: &str = "next_product_id";
const PROD_NAME: &str = "name";
const PROD_SORT_WEIGHT: &str = "sort_weight";
const PROD_STATE: &str = "is_done";
const PROD_OWNER: &str = "product_owner";
const PROD_QTY: &str = "quantity";
const PROD_UNIT: &str = "unit";
const PROD_AISLE: &str = "aisle";

pub fn product_key(id: &ProductId) -> String {
    format!("product:{}", **id)
}

pub fn products_in_aisle_key(id: &AisleId) -> String {
    format!("products_in_aisle:{}", **id)
}

fn get_product_owner(c: &Connection, id: &ProductId) -> Result<UserId> {
    Ok(UserId(c.hget(&product_key(&id), PROD_OWNER)?))
}

pub fn get_products_in_aisle(c: &Connection, aisle_id: &AisleId) -> Result<Vec<Product>> {
    let products: Vec<u32> = c.smembers(&products_in_aisle_key(&aisle_id))?;
    products
        .into_iter()
        .map(|p| {
            let product_key = product_key(&ProductId(p));
            let unit: u32 = c.hget(&product_key, PROD_UNIT)?;
            let state: i32 = c.hget(&product_key, PROD_STATE)?;
            let state = state != 0;
            Ok(Product::new(
                p,
                c.hget(&product_key, PROD_NAME)?,
                c.hget(&product_key, PROD_QTY)?,
                state,
                Unit::from(unit),
                c.hget(&product_key, PROD_SORT_WEIGHT)?,
            ))
        })
        .collect()
}

fn find_max_weight_in_aisle(c: &Connection, aisle_id: &AisleId) -> Result<f32> {
    let products = get_products_in_aisle(&c, &aisle_id)?;
    Ok(products.iter().max().map_or(0f32, |p| p.sort_weight))
}

pub fn save_product(auth: &Auth, name: &str, aisle_id: &AisleId) -> Result<Product> {
    let c = db::get_connection()?;
    let aisle_owner = db::aisles::get_aisle_owner(&c, &aisle_id)?;
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    db::verify_permission(&user_id, &aisle_owner)?;
    let prod_id = ProductId(c.incr(NEXT_PROD_ID, 1)?);
    let prod_key = product_key(&prod_id);
    let prod_in_aisle_key = products_in_aisle_key(&aisle_id);
    let new_sort_weight = find_max_weight_in_aisle(&c, &aisle_id)? + 1f32;
    transaction(&c, &[&prod_key, &prod_in_aisle_key], |pipe| {
        pipe.hset(&prod_key, PROD_NAME, name)
            .ignore()
            .hset(&prod_key, PROD_QTY, 1)
            .ignore()
            .hset(&prod_key, PROD_SORT_WEIGHT, new_sort_weight)
            .ignore()
            .hset(&prod_key, PROD_STATE, false as i32)
            .ignore()
            .hset(&prod_key, PROD_OWNER, *user_id)
            .ignore()
            .hset(&prod_key, PROD_UNIT, u32::from(Unit::Unit))
            .ignore()
            .hset(&prod_key, PROD_AISLE, **aisle_id)
            .ignore()
            .sadd(&prod_in_aisle_key, *prod_id)
            .query(&c)
    })?;
    Ok(Product::new(
        *prod_id,
        name.to_owned(),
        1,
        false,
        Unit::Unit,
        new_sort_weight,
    ))
}

pub fn modify_product(auth: &Auth, edit_data: &EditProduct, product_id: &ProductId) -> Result<()> {
    let c = db::get_connection()?;
    let product_owner = get_product_owner(&c, &product_id)?;
    db::verify_permission_auth(&c, &auth, &product_owner)?;
    let product_key = product_key(&product_id);
    if let Some(ref new_name) = edit_data.name {
        c.hset(&product_key, PROD_NAME, new_name)?;
    }
    if let Some(qty) = edit_data.quantity {
        c.hset(&product_key, PROD_QTY, qty)?;
    }
    if let Some(is_done) = edit_data.is_done {
        c.hset(&product_key, PROD_STATE, is_done as i32)?;
    }
    if let Some(unit) = &edit_data.unit {
        c.hset(&product_key, PROD_UNIT, u32::from(unit.clone()))?;
    }
    Ok(())
}

pub fn delete_product(auth: &Auth, product_id: &ProductId) -> Result<()> {
    let c = db::get_connection()?;
    let product_owner = get_product_owner(&c, &product_id)?;
    db::verify_permission_auth(&c, &auth, &product_owner)?;
    let product_key = product_key(&product_id);
    let aisle_id = AisleId(c.hget(&product_key, PROD_AISLE)?);
    let prod_in_aisle_key = products_in_aisle_key(&aisle_id);
    transaction(&c, &[&product_key, &prod_in_aisle_key], |pipe| {
        pipe.srem(&prod_in_aisle_key, **product_id)
            .ignore()
            .del(&product_key)
            .query(&c)
    })?;
    Ok(())
}

// purge all products contained in aisle
// to be used only in a transaction, doesn't execute the `pipe`
pub fn transaction_purge_products_in_aisle(
    c: &Connection,
    pipe: &mut Pipeline,
    aisle_id: &AisleId,
) -> Result<()> {
    let products_in_aisle_key = products_in_aisle_key(&aisle_id);
    let products: Option<Vec<u32>> = c.smembers(&products_in_aisle_key)?;
    if let Some(products) = products {
        products.into_iter().for_each(|p| {
            pipe.del(&product_key(&ProductId(p))).ignore();
        });
        pipe.del(&products_in_aisle_key).ignore();
    }
    Ok(())
}

pub fn edit_product_sort_weight(
    c: &Connection,
    pipe: &mut Pipeline,
    auth: &Auth,
    data: &ProductItemWeight,
) -> Result<()> {
    let product_id = ProductId(data.id);
    let product_owner = get_product_owner(&c, &product_id)?;
    db::verify_permission_auth(&c, &auth, &product_owner)?;
    let product_key = product_key(&product_id);
    pipe.hset(&product_key, PROD_SORT_WEIGHT, data.sort_weight)
        .ignore();
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db;
    use crate::db::sessions::tests::*;

    const NAME: &str = "product1";
    pub const RENAME: &str = "product2";

    // create a store, a session with AUTH token, an aisle and put a product in it
    pub fn save_product_test() {
        db::users::tests::store_user_for_test_with_reset();
        db::sessions::tests::store_session_for_test(&AUTH);
        let store_id = db::stores::save_store(&AUTH, "MyStore").unwrap();
        db::aisles::save_aisle(&AUTH, &store_id, db::aisles::tests::NAME).unwrap();
        let expected = Product::new(1, "product1".to_owned(), 1, false, Unit::Unit, 1f32);
        assert_eq!(Ok(expected), save_product(&AUTH, NAME, &AisleId(1)));

        // check DB
        let c = db::get_connection().unwrap();
        let prod_key = product_key(&ProductId(1));
        let name: String = c.hget(&prod_key, PROD_NAME).unwrap();
        assert_eq!(NAME, &name);
        let qty: u32 = c.hget(&prod_key, PROD_QTY).unwrap();
        assert_eq!(1, qty);
        let sort: f32 = c.hget(&prod_key, PROD_SORT_WEIGHT).unwrap();
        assert!(sort - 1f32 < std::f32::EPSILON);
        let is_done: i32 = c.hget(&prod_key, PROD_STATE).unwrap();
        assert_eq!(false, is_done != 0);
        let owner: u32 = c.hget(&prod_key, PROD_OWNER).unwrap();
        assert_eq!(1, owner);
        let res: bool = c.sismember(&products_in_aisle_key(&AisleId(1)), 1).unwrap();
        assert_eq!(true, res);
    }

    fn add_2nd_product() {
        let expected = Product::new(2, RENAME.to_owned(), 1, false, Unit::Unit, 0f32);
        assert_eq!(Ok(expected), save_product(&AUTH, RENAME, &AisleId(1)));
    }

    #[test]
    fn modify_product_test() {
        save_product_test();
        let data = EditProduct::new(Some(RENAME.to_owned()), Some(2), None, Some(true));
        assert_eq!(Ok(()), modify_product(&AUTH, &data, &ProductId(1)));

        // check DB
        let c = db::get_connection().unwrap();
        let product_key = product_key(&ProductId(1));
        let name: String = c.hget(&product_key, PROD_NAME).unwrap();
        assert_eq!(RENAME, &name);
        let qty: u32 = c.hget(&product_key, PROD_QTY).unwrap();
        assert_eq!(2, qty);
        let unit: u32 = c.hget(&product_key, PROD_UNIT).unwrap();
        let unit = Unit::from(unit);
        assert_eq!(Unit::Unit, unit);
        let state: i32 = c.hget(&product_key, PROD_STATE).unwrap();
        assert_eq!(true, state != 0);
    }

    #[test]
    fn get_products_in_aisle_test() {
        save_product_test();
        add_2nd_product();
        let c = db::get_connection().unwrap();
        let res = get_products_in_aisle(&c, &AisleId(1));
        let expected = vec![
            Product::new(1, NAME.to_owned(), 1, false, Unit::Unit, 0f32),
            Product::new(2, RENAME.to_owned(), 1, false, Unit::Unit, 0f32),
        ];
        assert_eq!(Ok(expected), res);
    }

    #[test]
    fn delete_product_test() {
        save_product_test();
        assert_eq!(Ok(()), delete_product(&AUTH, &ProductId(1)));
        let c = db::get_connection().unwrap();
        assert_eq!(Ok(false), c.exists(&product_key(&ProductId(1))));
    }

    #[test]
    fn transaction_purge_products_in_aisle_test() {
        save_product_test();
        add_2nd_product();
        let c = db::get_connection().unwrap();
        let mut pipe = Pipeline::new();
        pipe.atomic();
        let aisle_id = AisleId(1);
        assert_eq!(
            Ok(()),
            transaction_purge_products_in_aisle(&c, &mut pipe, &aisle_id)
        );
        assert_eq!(Ok(()), pipe.query(&c));
        assert_eq!(Ok(false), c.exists(&product_key(&ProductId(1))));
        assert_eq!(Ok(false), c.exists(&product_key(&ProductId(2))));
        assert_eq!(Ok(false), c.exists(&products_in_aisle_key(&aisle_id)));
    }

    #[test]
    fn edit_product_sort_weight_test() {
        save_product_test();
        let c = db::get_connection().unwrap();
        let mut pipe = Pipeline::new();
        pipe.atomic();
        assert_eq!(
            Ok(()),
            edit_product_sort_weight(&c, &mut pipe, &AUTH, &ProductItemWeight::new(1, 2.0f32))
        );
        assert_eq!(Ok(()), pipe.query(&c));
        assert_eq!(
            Ok(2.0f32),
            c.hget(&product_key(&ProductId(1)), PROD_SORT_WEIGHT)
        );
    }
}
