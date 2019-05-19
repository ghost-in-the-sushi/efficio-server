#[cfg(not(test))]
use redis::{transaction, Commands, Connection, PipelineCommands};

#[cfg(test)]
use fake_redis::{transaction, FakeConnection as Connection};

use crate::db;
use crate::error::*;
use crate::types::*;

const NEXT_STORE_ID: &str = "next_store_id";
const STORE_NAME: &str = "name";
const STORE_OWNER: &str = "owner_id";

fn store_key(id: &StoreId) -> String {
    format!("store:{}", **id)
}

fn user_stores_list_key(user_id: &UserId) -> String {
    format!("stores:{}", **user_id)
}

pub fn get_store_owner(c: &Connection, store_id: &StoreId) -> Result<UserId> {
    Ok(UserId(c.hget(&store_key(&store_id), STORE_OWNER)?))
}

pub fn list_store(c: &Connection, auth: &Auth, store_id: &StoreId) -> Result<Store> {
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let store_key = store_key(&store_id);
    db::verify_permission(&user_id, &get_store_owner(&c, &store_id)?)?;
    Ok(Store::new(
        **store_id,
        c.hget(&store_key, STORE_NAME)?,
        db::aisles::get_aisles_in_store(&c, &store_id)?,
    ))
}

pub fn save_store(c: &Connection, auth: &Auth, name: &str) -> Result<StoreId> {
    let store_id = StoreId::new(c.incr(NEXT_STORE_ID, 1)?);
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let store_key = store_key(&store_id);
    let user_stores_key = user_stores_list_key(&user_id);
    transaction(c, &[&store_key, &user_stores_key], |pipe| {
        pipe.hset(&store_key, STORE_NAME, name)
            .ignore()
            .hset(&store_key, STORE_OWNER, *user_id)
            .ignore()
            .sadd(&user_stores_key, *store_id)
            .query(c)
    })?;

    Ok(store_id)
}

pub fn edit_store(c: &Connection, auth: &Auth, store_id: &StoreId, new_name: &str) -> Result<()> {
    let owner_id = get_store_owner(&c, &store_id)?;
    db::verify_permission_auth(&c, &auth, &owner_id)?;
    Ok(c.hset(&store_key(&store_id), STORE_NAME, new_name)?)
}

pub fn get_all_stores(c: &Connection, auth: &Auth) -> Result<Vec<StoreLight>> {
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let all_store_ids: Vec<u32> = c.smembers(&user_stores_list_key(&user_id))?;
    Ok(all_store_ids
        .into_iter()
        .map(|id| {
            let name: String = c
                .hget(&store_key(&StoreId::new(id)), STORE_NAME)
                .expect("Db is corrupted? Should have a store name.");
            StoreLight::new(name, id)
        })
        .collect())
}

pub fn delete_store(c: &Connection, auth: &Auth, store_id: &StoreId) -> Result<()> {
    let owner_id = get_store_owner(&c, &store_id)?;
    db::verify_permission_auth(&c, &auth, &owner_id)?;
    let store_key = store_key(&store_id);
    let user_stores_key = user_stores_list_key(&owner_id);
    transaction(c, &[&store_key, &user_stores_key], |mut pipe| {
        db::aisles::transaction_purge_aisles_in_store(&c, &mut pipe, &store_id)?;
        pipe.srem(&user_stores_key, **store_id)
            .ignore()
            .del(&store_key)
            .query(c)
    })?;
    Ok(())
}

pub fn delete_all_user_stores(c: &Connection, auth: &Auth) -> Result<()> {
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let user_stores_key = user_stores_list_key(&user_id);
    let stores: Option<Vec<u32>> = c.smembers(&user_stores_key)?;
    if let Some(stores) = stores {
        for store_id in stores {
            delete_store(&c, &auth, &StoreId::new(store_id))?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use db::sessions::tests::*;
    use db::tests::*;
    use db::users::tests::*;

    pub const STORE_TEST_NAME: &str = "storetest";
    const NEW_STORE_NAME: &str = "new_store_name";

    pub fn save_store_for_test(c: &Connection) -> StoreId {
        store_user_for_test(&c);
        store_session_for_test(&c, &AUTH);
        let res = save_store(&c, &AUTH, STORE_TEST_NAME);
        assert_eq!(Ok(StoreId::new(1)), res);
        res.unwrap()
    }

    #[test]
    fn save_store_test() {
        let client = db::get_client(&get_db_addr());
        let c = client.get_connection().unwrap();
        save_store_for_test(&c);
        let store_key = store_key(&StoreId::new(1));
        let res: bool = c.exists(&store_key).unwrap();
        assert_eq!(true, res);
        let store_name: String = c.hget(&store_key, STORE_NAME).unwrap();
        assert_eq!(STORE_TEST_NAME, &store_name);
        let store_owner: u32 = c.hget(&store_key, STORE_OWNER).unwrap();
        assert_eq!(1, store_owner);
        let user_stores_list_key = user_stores_list_key(&UserId(1));
        let res: bool = c.exists(&user_stores_list_key).unwrap();
        assert_eq!(true, res);
        let res: bool = c.sismember(&user_stores_list_key, 1).unwrap();
        assert_eq!(true, res);
    }

    #[test]
    fn edit_store_test() {
        let client = db::get_client(&get_db_addr());
        let c = client.get_connection().unwrap();
        save_store_for_test(&c);
        assert_eq!(
            Ok(()),
            edit_store(&c, &AUTH, &StoreId::new(1), NEW_STORE_NAME)
        );
        let store_key = store_key(&StoreId::new(1));
        let store_name: String = c.hget(&store_key, STORE_NAME).unwrap();
        assert_eq!(NEW_STORE_NAME, &store_name);
    }

    #[test]
    fn get_all_stores_test() {
        let client = db::get_client(&get_db_addr());
        let c = client.get_connection().unwrap();
        save_store_for_test(&c);
        assert_eq!(Ok(StoreId::new(2)), save_store(&c, &AUTH, NEW_STORE_NAME));
        let expected_stores = vec![
            StoreLight::new(STORE_TEST_NAME.to_owned(), 1),
            StoreLight::new(NEW_STORE_NAME.to_owned(), 2),
        ];
        assert_eq!(Ok(expected_stores), get_all_stores(&c, &AUTH));
    }

    #[test]
    fn list_store_test() {
        let client = db::get_client(&get_db_addr());
        let c = client.get_connection().unwrap();
        db::aisles::tests::get_aisles_in_store_for_test(&c);
        let expected = Store::new(
            1,
            STORE_TEST_NAME.to_owned(),
            vec![
                Aisle::new(
                    1,
                    "Aisle1".to_owned(),
                    0f32,
                    vec![
                        Product::new(1, "product1".to_owned(), 1, false, Unit::Unit, 0f32),
                        Product::new(2, "product2".to_owned(), 1, false, Unit::Unit, 0f32),
                    ],
                ),
                Aisle::new(
                    2,
                    "AisleRenamed".to_owned(),
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
            ],
        );
        assert_eq!(Ok(expected), list_store(&c, &AUTH, &StoreId::new(1)));
    }

    #[test]
    fn delete_store_test() {
        let client = db::get_client(&get_db_addr());
        let c = client.get_connection().unwrap();

        db::aisles::tests::save_aisle_for_test(&c);
        db::aisles::tests::add_2nd_aisle(&c);
        db::aisles::tests::fill_aisles(&c);
        assert_eq!(Ok(()), delete_store(&c, &AUTH, &StoreId::new(1)));
        assert_eq!(
            Ok(false),
            c.sismember(&user_stores_list_key(&UserId(1)), 1u32)
        );
        assert_eq!(Ok(false), c.exists(&store_key(&StoreId::new(1))));
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisles_in_store_key(&StoreId::new(1)))
        );
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
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisle_key(&AisleId(1)))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisle_key(&AisleId(2)))
        );
    }
}
