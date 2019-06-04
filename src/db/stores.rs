#[cfg(not(test))]
use redis::{transaction, Commands, Connection, PipelineCommands};

#[cfg(test)]
use fake_redis::{transaction, FakeConnection as Connection};

use crate::db;
use crate::error::*;
use crate::types::*;

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
        store_id.to_string(),
        c.hget(&store_key, STORE_NAME)?,
        db::aisles::get_aisles_in_store(&c, &store_id)?,
    ))
}

pub fn save_store(c: &Connection, auth: &Auth, name: &str) -> Result<StoreId> {
    let store_id = db::salts::get_next_store_id(&c)?;
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let store_key = store_key(&store_id);
    let user_stores_key = user_stores_list_key(&user_id);
    transaction(c, &[&store_key, &user_stores_key], |pipe| {
        pipe.hset(&store_key, STORE_NAME, name)
            .ignore()
            .hset(&store_key, STORE_OWNER, user_id.to_string())
            .ignore()
            .sadd(&user_stores_key, store_id.to_string())
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
    let all_store_ids: Vec<String> = c.smembers(&user_stores_list_key(&user_id))?;
    Ok(all_store_ids
        .into_iter()
        .map(|id| {
            let name: String = c
                .hget(&store_key(&StoreId::new(id.to_owned())), STORE_NAME)
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
        pipe.srem(&user_stores_key, store_id.to_string())
            .ignore()
            .del(&store_key)
            .query(c)
    })?;
    Ok(())
}

pub fn delete_all_user_stores(c: &Connection, auth: &Auth) -> Result<()> {
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let user_stores_key = user_stores_list_key(&user_id);
    let stores: Option<Vec<String>> = c.smembers(&user_stores_key)?;
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
    use db::{salts::tests::*, sessions::tests::*, tests::*, users::tests::*};

    use fake_redis::FakeCient as Client;

    pub const STORE_TEST_NAME: &str = "storetest";
    const NEW_STORE_NAME: &str = "new_store_name";

    pub fn save_store_for_test(c: &Connection) -> StoreId {
        store_user_for_test(&c);
        store_session_for_test(&c, &AUTH);
        let res = save_store(&c, &AUTH, STORE_TEST_NAME);
        assert_eq!(Ok(StoreId::new(HASH_1.to_owned())), res);
        res.unwrap()
    }

    #[test]
    fn save_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();
        save_store_for_test(&c);
        let store_key = store_key(&StoreId::new(HASH_1.to_owned()));
        assert_eq!(Ok(true), c.exists(&store_key));
        assert_eq!(
            Ok(STORE_TEST_NAME.to_owned()),
            c.hget(&store_key, STORE_NAME)
        );
        assert_eq!(Ok(HASH_1.to_owned()), c.hget(&store_key, STORE_OWNER));
        let user_stores_list_key = user_stores_list_key(&UserId(HASH_1.to_owned()));
        assert_eq!(Ok(true), c.exists(&user_stores_list_key));
        assert_eq!(
            Ok(true),
            c.sismember(&user_stores_list_key, HASH_1.to_owned())
        );
    }

    #[test]
    fn edit_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();
        save_store_for_test(&c);
        assert_eq!(
            Ok(()),
            edit_store(&c, &AUTH, &StoreId::new(HASH_1.to_owned()), NEW_STORE_NAME)
        );
        let store_key = store_key(&StoreId::new(HASH_1.to_owned()));
        assert_eq!(
            Ok(NEW_STORE_NAME.to_owned()),
            c.hget(&store_key, STORE_NAME)
        );
    }

    #[test]
    fn get_all_stores_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();
        save_store_for_test(&c);
        assert_eq!(
            Ok(StoreId::new(HASH_2.to_owned())),
            save_store(&c, &AUTH, NEW_STORE_NAME)
        );
        let expected_stores = vec![
            StoreLight::new(STORE_TEST_NAME.to_owned(), HASH_1.to_owned()),
            StoreLight::new(NEW_STORE_NAME.to_owned(), HASH_2.to_owned()),
        ];
        assert_eq!(Ok(expected_stores), get_all_stores(&c, &AUTH));
    }

    #[test]
    fn list_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();
        db::aisles::tests::get_aisles_in_store_for_test(&c);
        let expected = Store::new(
            HASH_1.to_owned(),
            STORE_TEST_NAME.to_owned(),
            vec![
                Aisle::new(
                    HASH_1.to_owned(),
                    "Aisle1".to_owned(),
                    0f32,
                    vec![
                        Product::new(
                            HASH_1.to_owned(),
                            "product1".to_owned(),
                            1,
                            false,
                            Unit::Unit,
                            0f32,
                        ),
                        Product::new(
                            HASH_2.to_owned(),
                            "product2".to_owned(),
                            1,
                            false,
                            Unit::Unit,
                            0f32,
                        ),
                    ],
                ),
                Aisle::new(
                    HASH_2.to_owned(),
                    "AisleRenamed".to_owned(),
                    0f32,
                    vec![Product::new(
                        HASH_3.to_owned(),
                        "product3".to_owned(),
                        1,
                        false,
                        Unit::Unit,
                        0f32,
                    )],
                ),
            ],
        );
        assert_eq!(
            Ok(expected),
            list_store(&c, &AUTH, &StoreId::new(HASH_1.to_owned()))
        );
    }

    #[test]
    fn delete_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();

        db::aisles::tests::save_aisle_for_test(&c);
        db::aisles::tests::add_2nd_aisle(&c);
        db::aisles::tests::fill_aisles(&c);

        assert_eq!(
            Ok(()),
            delete_store(&c, &AUTH, &StoreId::new(HASH_1.to_owned()))
        );
        assert_eq!(
            Ok(false),
            c.sismember(&user_stores_list_key(&UserId(HASH_1.to_owned())), 1u32)
        );

        assert_eq!(
            Ok(false),
            c.exists(&store_key(&StoreId::new(HASH_1.to_owned())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisles_in_store_key(&StoreId::new(
                HASH_1.to_owned()
            )))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(HASH_2.to_owned())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(HASH_3.to_owned())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&AisleId(
                HASH_1.to_owned()
            )))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&AisleId(
                HASH_2.to_owned()
            )))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisle_key(&AisleId(HASH_1.to_owned())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisle_key(&AisleId(HASH_2.to_owned())))
        );
    }
}
