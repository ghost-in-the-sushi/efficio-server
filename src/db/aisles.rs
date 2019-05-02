use redis::{self, Commands, PipelineCommands};

use crate::db::{self, get_connection};
use crate::error::*;
use crate::types::*;

const NEXT_AISLE_ID: &str = "next_aisle_id";
const AISLE_NAME: &str = "name";
const AISLE_WEIGHT: &str = "sort_weight";
const AISLE_OWNER: &str = "owner_id";
const AISLE_STORE: &str = "store_id";

fn aisle_key(id: &AisleId) -> String {
    format!("aisle:{}", **id)
}

fn aisles_in_store_key(id: &StoreId) -> String {
    format!("aisles_in_store:{}", **id)
}

pub fn get_aisle_owner(c: &redis::Connection, aisle_id: &AisleId) -> Result<UserId> {
    Ok(UserId(c.hget(&aisle_key(&aisle_id), AISLE_OWNER)?))
}

pub fn get_aisles_in_store(c: &redis::Connection, store_id: &StoreId) -> Result<Vec<Aisle>> {
    let aisles: Vec<u32> = c.smembers(&aisles_in_store_key(&store_id))?;
    aisles
        .into_iter()
        .map(|i| {
            let aisle_id = AisleId(i);
            let aisle_key = aisle_key(&aisle_id);
            Ok(Aisle::new(
                i,
                c.hget(&aisle_key, AISLE_NAME)?,
                c.hget(&aisle_key, AISLE_WEIGHT)?,
                db::products::get_products_in_aisle(&c, &aisle_id)?,
            ))
        })
        .collect()
}

fn find_max_weight_in_store(c: &redis::Connection, store_id: &StoreId) -> Result<f32> {
    let aisles = get_aisles_in_store(&c, &store_id)?;
    Ok(aisles.iter().max().map_or(0f32, |a| a.sort_weight))
}

pub fn save_aisle(auth: &Auth, store_id: &StoreId, name: &str) -> Result<Aisle> {
    let c = get_connection()?;
    let aisle_id = AisleId(c.incr(NEXT_AISLE_ID, 1)?);
    let aisle_key = aisle_key(&aisle_id);
    let aisle_in_store_key = aisles_in_store_key(&store_id);
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let store_owner = db::stores::get_store_owner(&c, &store_id)?;
    db::verify_permission(&user_id, &store_owner)?;
    let new_sort_weight = find_max_weight_in_store(&c, &store_id)? + 1f32;
    redis::transaction(&c, &[&aisle_key, &aisle_in_store_key], |pipe| {
        pipe.hset(&aisle_key, AISLE_NAME, name)
            .ignore()
            .hset(&aisle_key, AISLE_WEIGHT, new_sort_weight)
            .ignore()
            .hset(&aisle_key, AISLE_OWNER, *user_id)
            .ignore()
            .hset(&aisle_key, AISLE_STORE, **store_id)
            .ignore()
            .sadd(&aisle_in_store_key, *aisle_id)
            .query(&c)
    })?;

    Ok(Aisle::new(*aisle_id, name.to_owned(), 0.0, vec![]))
}

pub fn edit_aisle(auth: &Auth, aisle_id: &AisleId, new_name: &str) -> Result<()> {
    let c = get_connection()?;
    let aisle_key = aisle_key(&aisle_id);
    let aisle_owner = get_aisle_owner(&c, &aisle_id)?;
    db::verify_permission_auth(&c, &auth, &aisle_owner)?;
    Ok(c.hset(&aisle_key, AISLE_NAME, new_name)?)
}

pub fn delete_aisle(auth: &Auth, aisle_id: &AisleId) -> Result<()> {
    let c = get_connection()?;
    let aisle_key = aisle_key(&aisle_id);
    let aisle_owner = get_aisle_owner(&c, &aisle_id)?;
    db::verify_permission_auth(&c, &auth, &aisle_owner)?;
    let store_id = StoreId::new(c.hget(&aisle_key, AISLE_STORE)?);
    let aisle_in_store_key = aisles_in_store_key(&store_id);
    redis::transaction(&c, &[&aisle_key, &aisle_in_store_key], |mut pipe| {
        db::products::transaction_purge_products_in_aisle(&c, &mut pipe, &aisle_id)?;
        pipe.srem(&aisle_in_store_key, **aisle_id)
            .ignore()
            .del(&aisle_key)
            .query(&c)
    })?;
    Ok(())
}

pub fn transaction_purge_aisles_in_store(
    c: &redis::Connection,
    mut pipe: &mut redis::Pipeline,
    store_id: &StoreId,
) -> Result<()> {
    let aisles_in_store_key = aisles_in_store_key(&store_id);
    let aisles: Vec<u32> = c.smembers(&aisles_in_store_key)?;
    for aisle_id in aisles {
        let aisle_id = AisleId(aisle_id);
        db::products::transaction_purge_products_in_aisle(&c, &mut pipe, &aisle_id)?;
        pipe.del(&aisle_key(&aisle_id))
            .ignore()
            .del(&db::products::products_in_aisle_key(&aisle_id))
            .ignore();
    }
    pipe.del(&aisles_in_store_key).ignore();
    Ok(())
}

pub fn edit_aisle_sort_weight(
    c: &redis::Connection,
    pipe: &mut redis::Pipeline,
    auth: &Auth,
    data: &AisleItemWeight,
) -> Result<()> {
    let aisle_id = AisleId(data.id);
    let aisle_owner = get_aisle_owner(&c, &aisle_id)?;
    db::verify_permission_auth(&c, &auth, &aisle_owner)?;
    let aisle_key = aisle_key(&aisle_id);
    pipe.hset(&aisle_key, AISLE_WEIGHT, data.sort_weight)
        .ignore();
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db;
    use crate::db::stores::*;
    use crate::db::{sessions::tests::*, users::tests::*};

    pub const NAME: &str = "Aisle1";
    const RENAMED: &str = "AisleRenamed";

    pub fn aisles_in_store_key(store_id: &StoreId) -> String {
        super::aisles_in_store_key(&store_id)
    }

    pub fn aisle_key(aisle_id: &AisleId) -> String {
        super::aisle_key(&aisle_id)
    }
    // create a user, a session with AUTH as token, a store and an aisle
    pub fn save_aisle_test() {
        store_user_for_test_with_reset();
        store_session_for_test(&AUTH);
        let store_id = save_store(&AUTH, db::stores::tests::STORE_TEST_NAME).unwrap();
        let expected = Aisle::new(1, NAME.to_owned(), 0f32, vec![]);
        assert_eq!(Ok(expected), save_aisle(&AUTH, &store_id, NAME));

        // check DB
        let c = get_connection().unwrap();
        let key = aisle_key(&AisleId(1));
        let res: bool = c.exists(&key).unwrap();
        assert_eq!(true, res);
        let res: bool = c.exists(&aisles_in_store_key(&store_id)).unwrap();
        assert_eq!(true, res);
        let name: String = c.hget(&key, AISLE_NAME).unwrap();
        assert_eq!(NAME, name.as_str());
        let weight: f32 = c.hget(&key, AISLE_WEIGHT).unwrap();
        assert!(weight - 0.0f32 < std::f32::EPSILON);
        let aisle_store: u32 = c.hget(&key, AISLE_STORE).unwrap();
        assert_eq!(1, aisle_store);
        let res: bool = c.sismember(&aisles_in_store_key(&store_id), 1).unwrap();
        assert_eq!(true, res);
    }

    #[test]
    fn edit_aisle_test() {
        save_aisle_test();
        assert_eq!(Ok(()), edit_aisle(&AUTH, &AisleId(1), RENAMED));
        let c = get_connection().unwrap();
        let name: String = c.hget(&aisle_key(&AisleId(1)), AISLE_NAME).unwrap();
        assert_eq!(RENAMED, name.as_str());
    }

    pub fn add_2nd_aisle() {
        let c = db::get_connection().unwrap();
        assert_eq!(Ok(false), c.exists(&aisle_key(&AisleId(2))));
        let expected = Aisle::new(2, RENAMED.to_owned(), 0f32, vec![]);
        assert_eq!(Ok(expected), save_aisle(&AUTH, &StoreId::new(1), RENAMED));
        assert_eq!(Ok(true), c.exists(&aisle_key(&AisleId(2))));
    }

    pub fn fill_aisles() {
        db::products::save_product(&AUTH, "product1", &AisleId(1)).unwrap();
        db::products::save_product(&AUTH, "product2", &AisleId(1)).unwrap();
        db::products::save_product(&AUTH, "product3", &AisleId(2)).unwrap();
        let c = db::get_connection().unwrap();
        assert_eq!(
            Ok(true),
            c.exists(&db::products::product_key(&ProductId(1)))
        );
        assert_eq!(
            Ok(true),
            c.exists(&db::products::product_key(&ProductId(2)))
        );
        assert_eq!(
            Ok(true),
            c.exists(&db::products::product_key(&ProductId(3)))
        );
        assert_eq!(
            Ok(true),
            c.sismember(&db::products::products_in_aisle_key(&AisleId(1)), 1u32)
        );
        assert_eq!(
            Ok(true),
            c.sismember(&db::products::products_in_aisle_key(&AisleId(1)), 2u32)
        );
        assert_eq!(
            Ok(true),
            c.sismember(&db::products::products_in_aisle_key(&AisleId(2)), 3u32)
        );
    }

    pub fn get_aisles_in_store_for_test() {
        save_aisle_test();
        add_2nd_aisle();
        fill_aisles();
        let c = get_connection().unwrap();
        let res = get_aisles_in_store(&c, &StoreId::new(1));
        let expected = vec![
            Aisle::new(
                1,
                NAME.to_owned(),
                0f32,
                vec![
                    Product::new(1, "product1".to_owned(), 1, false, Unit::Unit, 0f32),
                    Product::new(2, "product2".to_owned(), 1, false, Unit::Unit, 0f32),
                ],
            ),
            Aisle::new(
                2,
                RENAMED.to_owned(),
                0f32,
                vec![Product::new(
                    3,
                    "product3".to_owned(),
                    1,
                    false,
                    Unit::Unit,
                    0f32,
                )],
            ),
        ];
        assert_eq!(Ok(expected), res);
    }

    #[test]
    fn get_aisles_in_store_test() {
        get_aisles_in_store_for_test()
    }

    #[test]
    fn delete_aisle_test() {
        // this create a store, an aisle and put a product in it
        db::products::tests::save_product_test();
        // add another product
        let expected = Product::new(2, "product2".to_owned(), 1, false, Unit::Unit, 0f32);
        assert_eq!(
            Ok(expected),
            db::products::save_product(&AUTH, "product2", &AisleId(1))
        );

        assert_eq!(Ok(()), delete_aisle(&AUTH, &(AisleId(1))));
        let c = get_connection().unwrap();
        assert_eq!(Ok(false), c.exists(&aisle_key(&AisleId(1))));
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&AisleId(1)))
        );
        assert_eq!(
            Ok(false),
            c.exists(db::products::product_key(&ProductId(1)))
        );
        assert_eq!(
            Ok(false),
            c.exists(db::products::product_key(&ProductId(2)))
        );
    }

    #[test]
    fn transaction_purge_aisles_test() {
        save_aisle_test();
        add_2nd_aisle();
        fill_aisles();
        let c = db::get_connection().unwrap();
        let aisle_in_store_key = aisles_in_store_key(&StoreId::new(1));
        let mut pipe = redis::pipe();
        pipe.atomic();
        assert_eq!(
            Ok(()),
            transaction_purge_aisles_in_store(&c, &mut pipe, &StoreId::new(1))
        );
        assert_eq!(Ok(()), pipe.query(&c));
        assert_eq!(Ok(false), c.exists(&aisle_in_store_key));
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(1)))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(2)))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(3)))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&AisleId(1)))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&AisleId(2)))
        );
        assert_eq!(Ok(false), c.exists(&aisle_key(&AisleId(1))));
        assert_eq!(Ok(false), c.exists(&aisle_key(&AisleId(2))));
    }
}
