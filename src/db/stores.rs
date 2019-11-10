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

pub fn get_store_owner(c: &mut Connection, store_id: &StoreId) -> Result<UserId> {
    Ok(UserId(c.hget(&store_key(&store_id), STORE_OWNER)?))
}

pub fn list_store(c: &mut Connection, auth: &Auth, store_id: &StoreId) -> Result<Store> {
    let user_id = db::sessions::get_user_id(c, &auth)?;
    let store_key = store_key(&store_id);
    db::verify_permission(&user_id, &get_store_owner(c, &store_id)?)?;
    Ok(Store::new(
        store_id.to_string(),
        c.hget(&store_key, STORE_NAME)?,
        db::aisles::get_aisles_in_store(c, &store_id)?,
    ))
}

pub fn save_store(c: &mut Connection, auth: &Auth, name: &str) -> Result<StoreId> {
    let store_id = db::ids::get_next_store_id();
    let user_id = db::sessions::get_user_id(c, &auth)?;
    let store_key = store_key(&store_id);
    let user_stores_key = user_stores_list_key(&user_id);
    transaction(c, &[&store_key, &user_stores_key], |c, pipe| {
        pipe.hset(&store_key, STORE_NAME, name)
            .ignore()
            .hset(&store_key, STORE_OWNER, user_id.to_string())
            .ignore()
            .sadd(&user_stores_key, store_id.to_string())
            .query(c)
    })?;

    Ok(store_id)
}

pub fn edit_store(
    c: &mut Connection,
    auth: &Auth,
    store_id: &StoreId,
    new_name: &str,
) -> Result<()> {
    let owner_id = get_store_owner(c, &store_id)?;
    db::verify_permission_auth(c, &auth, &owner_id)?;
    Ok(c.hset(&store_key(&store_id), STORE_NAME, new_name)?)
}

pub fn get_all_stores(c: &mut Connection, auth: &Auth) -> Result<Vec<StoreLight>> {
    let user_id = db::sessions::get_user_id(c, &auth)?;
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

pub fn delete_store(c: &mut Connection, auth: &Auth, store_id: &StoreId) -> Result<()> {
    let owner_id = get_store_owner(c, &store_id)?;
    db::verify_permission_auth(c, &auth, &owner_id)?;
    let store_key = store_key(&store_id);
    let user_stores_key = user_stores_list_key(&owner_id);
    transaction(c, &[&store_key, &user_stores_key], |c, mut pipe| {
        db::aisles::transaction_purge_aisles_in_store(c, &mut pipe, &store_id)?;
        pipe.srem(&user_stores_key, store_id.to_string())
            .ignore()
            .del(&store_key)
            .query(c)
    })?;
    Ok(())
}

pub fn delete_all_user_stores(c: &mut Connection, auth: &Auth) -> Result<()> {
    let user_id = db::sessions::get_user_id(c, &auth)?;
    let user_stores_key = user_stores_list_key(&user_id);
    let stores: Option<Vec<String>> = c.smembers(&user_stores_key)?;
    if let Some(stores) = stores {
        for store_id in stores {
            delete_store(c, &auth, &StoreId::new(store_id))?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use db::{ids::tests::*, sessions::tests::*, tests::*, users::tests::*};

    use fake_redis::FakeCient as Client;

    pub const STORE_TEST_NAME: &str = "storetest";
    const NEW_STORE_NAME: &str = "new_store_name";

    pub fn save_store_for_test(c: &mut Connection) -> StoreId {
        store_user_for_test(c);
        store_session_for_test(c, &AUTH);
        let res = save_store(c, &AUTH, STORE_TEST_NAME);
        assert!(res.is_ok());
        res.unwrap()
    }

    #[test]
    fn save_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let store_id = save_store_for_test(&mut c);
        let store_key = store_key(&store_id);
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
            c.sismember(&user_stores_list_key, store_id.to_string())
        );
    }

    #[test]
    fn edit_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let store_id = save_store_for_test(&mut c);
        assert_eq!(Ok(()), edit_store(&mut c, &AUTH, &store_id, NEW_STORE_NAME));
        let store_key = store_key(&store_id);
        assert_eq!(
            Ok(NEW_STORE_NAME.to_owned()),
            c.hget(&store_key, STORE_NAME)
        );
    }

    #[test]
    fn get_all_stores_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let store_id = save_store_for_test(&mut c);
        let store_id2 = save_store(&mut c, &AUTH, NEW_STORE_NAME).unwrap();

        let expected_stores = vec![
            StoreLight::new(STORE_TEST_NAME.to_owned(), store_id.to_string()),
            StoreLight::new(NEW_STORE_NAME.to_owned(), store_id2.to_string()),
        ];
        assert_eq!(Ok(expected_stores), get_all_stores(&mut c, &AUTH));
    }

    #[test]
    fn list_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let store_id = db::aisles::tests::get_aisles_in_store_for_test(&mut c);
        let expected = Store::new(
            "".to_owned(),
            STORE_TEST_NAME.to_owned(),
            vec![
                Aisle::new(
                    "".to_owned(),
                    "Aisle1".to_owned(),
                    0f32,
                    vec![
                        Product::new(
                            "".to_owned(),
                            "product1".to_owned(),
                            1,
                            false,
                            Unit::Unit,
                            0f32,
                        ),
                        Product::new(
                            "".to_owned(),
                            "product2".to_owned(),
                            1,
                            false,
                            Unit::Unit,
                            0f32,
                        ),
                    ],
                ),
                Aisle::new(
                    "".to_owned(),
                    "AisleRenamed".to_owned(),
                    0f32,
                    vec![Product::new(
                        "".to_owned(),
                        "product3".to_owned(),
                        1,
                        false,
                        Unit::Unit,
                        0f32,
                    )],
                ),
            ],
        );
        assert_eq!(Ok(expected), list_store(&mut c, &AUTH, &store_id));
    }

    #[test]
    fn delete_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        let (store_id, aisle_id) = db::aisles::tests::save_aisle_for_test(&mut c);
        let aid2 = db::aisles::tests::add_2nd_aisle(&mut c, &store_id);
        let (p1, p2, p3) = db::aisles::tests::fill_aisles(&mut c, &aisle_id, &aid2);

        assert_eq!(Ok(()), delete_store(&mut c, &AUTH, &store_id));
        assert_eq!(
            Ok(false),
            c.sismember(&user_stores_list_key(&UserId(HASH_1.to_owned())), 1u32)
        );

        assert_eq!(Ok(false), c.exists(&store_key(&store_id)));
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisles_in_store_key(&store_id))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(p1.to_string())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(p2.to_string())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(p3.to_string())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&aisle_id))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&aid2))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::aisles::tests::aisle_key(&aisle_id))
        );
        assert_eq!(Ok(false), c.exists(&db::aisles::tests::aisle_key(&aid2)));
    }
}
