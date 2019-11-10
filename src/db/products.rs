use std::convert::From;

#[cfg(not(test))]
use redis::{self, transaction, Commands, Connection, Pipeline, PipelineCommands};

#[cfg(test)]
use fake_redis::{transaction, FakeConnection as Connection, FakePipeline as Pipeline};

use crate::db;
use crate::error::*;
use crate::types::*;

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

fn get_product_owner(c: &mut Connection, id: &ProductId) -> Result<UserId> {
    Ok(UserId(c.hget(&product_key(&id), PROD_OWNER)?))
}

pub fn get_products_in_aisle(c: &mut Connection, aisle_id: &AisleId) -> Result<Vec<Product>> {
    let products: Vec<String> = c.smembers(&products_in_aisle_key(&aisle_id))?;
    products
        .into_iter()
        .map(|p| {
            let product_key = product_key(&ProductId(p.clone()));
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

fn find_max_weight_in_aisle(c: &mut Connection, aisle_id: &AisleId) -> Result<f32> {
    let products = get_products_in_aisle(c, &aisle_id)?;
    Ok(products.iter().max().map_or(0f32, |p| p.sort_weight))
}

pub fn save_product(
    c: &mut Connection,
    auth: &Auth,
    name: &str,
    aisle_id: &AisleId,
) -> Result<Product> {
    let aisle_owner = db::aisles::get_aisle_owner(c, &aisle_id)?;
    let user_id = db::sessions::get_user_id(c, &auth)?;
    db::verify_permission(&user_id, &aisle_owner)?;
    let prod_id = db::ids::get_next_product_id();
    let prod_key = product_key(&prod_id);
    let prod_in_aisle_key = products_in_aisle_key(&aisle_id);
    let new_sort_weight = find_max_weight_in_aisle(c, &aisle_id)? + 1f32;
    transaction(c, &[&prod_key, &prod_in_aisle_key], |c, pipe| {
        pipe.hset(&prod_key, PROD_NAME, name)
            .ignore()
            .hset(&prod_key, PROD_QTY, 1)
            .ignore()
            .hset(&prod_key, PROD_SORT_WEIGHT, new_sort_weight)
            .ignore()
            .hset(&prod_key, PROD_STATE, false as i32)
            .ignore()
            .hset(&prod_key, PROD_OWNER, &*user_id)
            .ignore()
            .hset(&prod_key, PROD_UNIT, u32::from(Unit::Unit))
            .ignore()
            .hset(&prod_key, PROD_AISLE, &**aisle_id)
            .ignore()
            .sadd(&prod_in_aisle_key, &*prod_id)
            .query(c)
    })?;
    Ok(Product::new(
        prod_id.to_string(),
        name.to_owned(),
        1,
        false,
        Unit::Unit,
        new_sort_weight,
    ))
}

pub fn modify_product(
    c: &mut Connection,
    auth: &Auth,
    edit_data: &EditProduct,
    product_id: &ProductId,
) -> Result<()> {
    let product_owner = get_product_owner(c, &product_id)?;
    db::verify_permission_auth(c, &auth, &product_owner)?;
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

pub fn delete_product(c: &mut Connection, auth: &Auth, product_id: &ProductId) -> Result<()> {
    let product_owner = get_product_owner(c, &product_id)?;
    db::verify_permission_auth(c, &auth, &product_owner)?;
    let product_key = product_key(&product_id);
    let aisle_id = AisleId(c.hget(&product_key, PROD_AISLE)?);
    let prod_in_aisle_key = products_in_aisle_key(&aisle_id);
    transaction(c, &[&product_key, &prod_in_aisle_key], |c, pipe| {
        pipe.srem(&prod_in_aisle_key, &**product_id)
            .ignore()
            .del(&product_key)
            .query(c)
    })?;
    Ok(())
}

// purge all products contained in aisle
// to be used only in a transaction, doesn't execute the `pipe`
pub fn transaction_purge_products_in_aisle(
    c: &mut Connection,
    pipe: &mut Pipeline,
    aisle_id: &AisleId,
) -> Result<()> {
    let products_in_aisle_key = products_in_aisle_key(&aisle_id);
    let products: Option<Vec<String>> = c.smembers(&products_in_aisle_key)?;
    if let Some(products) = products {
        products.into_iter().for_each(|p| {
            pipe.del(&product_key(&ProductId(p))).ignore();
        });
        pipe.del(&products_in_aisle_key).ignore();
    }
    Ok(())
}

pub fn edit_product_sort_weight(
    c: &mut Connection,
    pipe: &mut Pipeline,
    auth: &Auth,
    data: &ProductItemWeight,
) -> Result<()> {
    let product_id = ProductId(data.id.clone());
    let product_owner = get_product_owner(c, &product_id)?;
    db::verify_permission_auth(c, &auth, &product_owner)?;
    let product_key = product_key(&product_id);
    pipe.hset(&product_key, PROD_SORT_WEIGHT, data.sort_weight)
        .ignore();
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db::sessions::tests::*;
    use crate::db::{self, ids::tests::HASH_1, tests::*};
    use fake_redis::FakeCient as Client;

    const NAME: &str = "product1";
    pub const RENAME: &str = "product2";

    pub fn save_product_for_test(c: &mut Connection) -> (AisleId, ProductId) {
        db::users::tests::store_user_for_test(c);
        db::sessions::tests::store_session_for_test(c, &AUTH);
        let store_id = db::stores::save_store(c, &AUTH, "MyStore").unwrap();
        let aisle = db::aisles::save_aisle(c, &AUTH, &store_id, db::aisles::tests::NAME).unwrap();
        let expected = Product::new(
            "".to_owned(), // ignored in tests
            "product1".to_owned(),
            1,
            false,
            Unit::Unit,
            1f32,
        );
        let res = save_product(c, &AUTH, NAME, &AisleId(aisle.id().to_owned()));
        assert_eq!(Ok(expected), res);
        (aisle.id(), res.unwrap().id())
    }

    // create a store, a session with AUTH token, an aisle and put a product in it
    #[test]
    fn save_product_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let (aisle_id, product_id) = save_product_for_test(&mut c);

        // check DB
        let prod_key = product_key(&product_id);
        assert_eq!(Ok(NAME.to_string()), c.hget(&prod_key, PROD_NAME));
        assert_eq!(Ok(1), c.hget(&prod_key, PROD_QTY));
        let sort: f32 = c.hget(&prod_key, PROD_SORT_WEIGHT).unwrap();
        assert!(sort - 1f32 < std::f32::EPSILON);
        let is_done: i32 = c.hget(&prod_key, PROD_STATE).unwrap();
        assert_eq!(false, is_done != 0);
        assert_eq!(Ok(HASH_1.to_owned()), c.hget(&prod_key, PROD_OWNER));
        assert_eq!(
            Ok(true),
            c.sismember(&products_in_aisle_key(&aisle_id), product_id.to_string(),)
        );
    }

    fn add_2nd_product(c: &mut Connection, aisle_id: &AisleId) -> ProductId {
        let expected = Product::new("".to_owned(), RENAME.to_owned(), 1, false, Unit::Unit, 0f32);
        let res = save_product(c, &AUTH, RENAME, &aisle_id);
        assert_eq!(Ok(expected), res);
        res.unwrap().id()
    }

    #[test]
    fn modify_product_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let (_, product_id) = save_product_for_test(&mut c);
        let data = EditProduct::new(Some(RENAME.to_owned()), Some(2), None, Some(true));
        assert_eq!(Ok(()), modify_product(&mut c, &AUTH, &data, &product_id));

        // check DB
        let product_key = product_key(&product_id);
        let name: String = c.hget(&product_key, PROD_NAME).unwrap();
        assert_eq!(RENAME, &name);
        assert_eq!(Ok(2), c.hget(&product_key, PROD_QTY));
        let unit: u32 = c.hget(&product_key, PROD_UNIT).unwrap();
        let unit = Unit::from(unit);
        assert_eq!(Unit::Unit, unit);
        let state: i32 = c.hget(&product_key, PROD_STATE).unwrap();
        assert_eq!(true, state != 0);
    }

    #[test]
    fn get_products_in_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        let (aisle_id, _) = save_product_for_test(&mut c);
        add_2nd_product(&mut c, &aisle_id);
        let res = get_products_in_aisle(&mut c, &aisle_id);
        let expected = vec![
            Product::new("".to_owned(), NAME.to_owned(), 1, false, Unit::Unit, 0f32),
            Product::new("".to_owned(), RENAME.to_owned(), 1, false, Unit::Unit, 0f32),
        ];
        assert_eq!(Ok(expected), res);
    }

    #[test]
    fn delete_product_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        let (_, p) = save_product_for_test(&mut c);
        assert_eq!(Ok(()), delete_product(&mut c, &AUTH, &p));
        assert_eq!(Ok(false), c.exists(&product_key(&p)));
    }

    #[test]
    fn transaction_purge_products_in_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        let (aisle_id, product_id) = save_product_for_test(&mut c);
        let p2 = add_2nd_product(&mut c, &aisle_id);
        let mut pipe = Pipeline::new(c.db);
        pipe.atomic();
        assert_eq!(
            Ok(()),
            transaction_purge_products_in_aisle(&mut c, &mut pipe, &aisle_id)
        );
        assert_eq!(Ok(()), pipe.query(&c));
        assert_eq!(Ok(false), c.exists(&product_key(&product_id)));
        assert_eq!(Ok(false), c.exists(&product_key(&p2)));
        assert_eq!(Ok(false), c.exists(&products_in_aisle_key(&aisle_id)));
    }

    #[test]
    fn edit_product_sort_weight_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let (_, product_id) = save_product_for_test(&mut c);
        let mut pipe = Pipeline::new(c.db);
        pipe.atomic();
        assert_eq!(
            Ok(()),
            edit_product_sort_weight(
                &mut c,
                &mut pipe,
                &AUTH,
                &ProductItemWeight::new(product_id.to_string(), 2.0f32)
            )
        );
        assert_eq!(Ok(()), pipe.query(&c));
        assert_eq!(
            Ok(2.0f32),
            c.hget(&product_key(&product_id), PROD_SORT_WEIGHT)
        );
    }
}
