#[cfg(not(test))]
use redis::{self, transaction, Commands, Connection, Pipeline, PipelineCommands};

#[cfg(test)]
use fake_redis::{transaction, FakeConnection as Connection, FakePipeline as Pipeline};

use crate::db;
use crate::error::*;
use crate::types::*;

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

pub fn get_aisle_owner(c: &Connection, aisle_id: &AisleId) -> Result<UserId> {
    Ok(UserId(c.hget(&aisle_key(&aisle_id), AISLE_OWNER)?))
}

pub fn get_aisles_in_store(c: &Connection, store_id: &StoreId) -> Result<Vec<Aisle>> {
    let aisles: Vec<String> = c.smembers(&aisles_in_store_key(&store_id))?;
    aisles
        .into_iter()
        .map(|i| {
            let aisle_id = AisleId(i.clone());
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

fn find_max_weight_in_store(c: &Connection, store_id: &StoreId) -> Result<f32> {
    let aisles = get_aisles_in_store(&c, &store_id)?;
    Ok(aisles.iter().max().map_or(0f32, |a| a.sort_weight))
}

pub fn save_aisle(c: &Connection, auth: &Auth, store_id: &StoreId, name: &str) -> Result<Aisle> {
    let aisle_id = db::salts::get_next_aisle_id(&c)?;
    let aisle_key = aisle_key(&aisle_id);
    let aisle_in_store_key = aisles_in_store_key(&store_id);
    let user_id = db::sessions::get_user_id(&c, &auth)?;
    let store_owner = db::stores::get_store_owner(&c, &store_id)?;
    db::verify_permission(&user_id, &store_owner)?;
    let new_sort_weight = find_max_weight_in_store(&c, &store_id)? + 1f32;
    transaction(c, &[&aisle_key, &aisle_in_store_key], |pipe| {
        pipe.hset(&aisle_key, AISLE_NAME, name)
            .ignore()
            .hset(&aisle_key, AISLE_WEIGHT, new_sort_weight)
            .ignore()
            .hset(&aisle_key, AISLE_OWNER, &*user_id)
            .ignore()
            .hset(&aisle_key, AISLE_STORE, &**store_id)
            .ignore()
            .sadd(&aisle_in_store_key, &*aisle_id)
            .query(c)
    })?;

    Ok(Aisle::new(
        aisle_id.to_string(),
        name.to_owned(),
        new_sort_weight,
        vec![],
    ))
}

pub fn edit_aisle(c: &Connection, auth: &Auth, aisle_id: &AisleId, new_name: &str) -> Result<()> {
    let aisle_key = aisle_key(&aisle_id);
    let aisle_owner = get_aisle_owner(&c, &aisle_id)?;
    db::verify_permission_auth(&c, &auth, &aisle_owner)?;
    Ok(c.hset(&aisle_key, AISLE_NAME, new_name)?)
}

pub fn delete_aisle(c: &Connection, auth: &Auth, aisle_id: &AisleId) -> Result<()> {
    let aisle_key = aisle_key(&aisle_id);
    let aisle_owner = get_aisle_owner(&c, &aisle_id)?;
    db::verify_permission_auth(&c, &auth, &aisle_owner)?;
    let store_id = StoreId::new(c.hget(&aisle_key, AISLE_STORE)?);
    let aisle_in_store_key = aisles_in_store_key(&store_id);
    transaction(c, &[&aisle_key, &aisle_in_store_key], |mut pipe| {
        db::products::transaction_purge_products_in_aisle(&c, &mut pipe, &aisle_id)?;
        pipe.srem(&aisle_in_store_key, &**aisle_id)
            .ignore()
            .del(&aisle_key)
            .query(c)
    })?;
    Ok(())
}

pub fn transaction_purge_aisles_in_store(
    c: &Connection,
    mut pipe: &mut Pipeline,
    store_id: &StoreId,
) -> Result<()> {
    let aisles_in_store_key = aisles_in_store_key(&store_id);
    let aisles: Option<Vec<String>> = c.smembers(&aisles_in_store_key)?;
    if let Some(aisles) = aisles {
        for aisle_id in aisles {
            let aisle_id = AisleId(aisle_id);
            db::products::transaction_purge_products_in_aisle(&c, &mut pipe, &aisle_id)?;
            pipe.del(&aisle_key(&aisle_id))
                .ignore()
                .del(&db::products::products_in_aisle_key(&aisle_id))
                .ignore();
        }
        pipe.del(&aisles_in_store_key).ignore();
    }
    Ok(())
}

pub fn edit_aisle_sort_weight(
    c: &Connection,
    pipe: &mut Pipeline,
    auth: &Auth,
    data: &AisleItemWeight,
) -> Result<()> {
    let aisle_id = AisleId(data.id.clone());
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
    use crate::db::{self, salts::tests::*, sessions::tests::*, stores::tests::*, tests::*};
    use fake_redis::FakeCient as Client;

    pub const NAME: &str = "Aisle1";
    const RENAMED: &str = "AisleRenamed";

    pub fn aisles_in_store_key(store_id: &StoreId) -> String {
        super::aisles_in_store_key(&store_id)
    }

    pub fn aisle_key(aisle_id: &AisleId) -> String {
        super::aisle_key(&aisle_id)
    }

    pub fn save_aisle_for_test(c: &Connection) -> StoreId {
        let store_id = save_store_for_test(&c);
        let expected = Aisle::new(HASH_1.to_owned(), NAME.to_owned(), 0f32, vec![]);
        assert_eq!(Ok(expected), save_aisle(&c, &AUTH, &store_id, NAME));
        store_id
    }

    // create a user, a session with AUTH as token, a store and an aisle
    #[test]
    fn save_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();
        let store_id = save_aisle_for_test(&c);

        // check DB
        let key = aisle_key(&AisleId(HASH_1.to_owned()));
        assert_eq!(Ok(true), c.exists(&key));
        assert_eq!(Ok(true), c.exists(&aisles_in_store_key(&store_id)));
        assert_eq!(Ok(NAME.to_string()), c.hget(&key, AISLE_NAME));
        let weight: f32 = c.hget(&key, AISLE_WEIGHT).unwrap();
        assert!(weight - 1.0f32 < std::f32::EPSILON);
        assert_eq!(Ok(HASH_1.to_owned()), c.hget(&key, AISLE_STORE));
        assert_eq!(
            Ok(true),
            c.sismember(&aisles_in_store_key(&store_id), HASH_1.to_owned())
        );
    }

    #[test]
    fn edit_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();
        let aid = AisleId(HASH_1.to_owned());
        save_aisle_for_test(&c);
        assert_eq!(Ok(()), edit_aisle(&c, &AUTH, &aid, RENAMED));

        let name: String = c.hget(&aisle_key(&aid), AISLE_NAME).unwrap();
        assert_eq!(RENAMED, name.as_str());
    }

    pub fn add_2nd_aisle(c: &Connection) {
        let aid = AisleId(HASH_2.to_owned());
        assert_eq!(Ok(false), c.exists(&aisle_key(&aid)));
        let expected = Aisle::new(HASH_2.to_owned(), RENAMED.to_owned(), 0f32, vec![]);
        assert_eq!(
            Ok(expected),
            save_aisle(&c, &AUTH, &StoreId::new(HASH_1.to_owned()), RENAMED)
        );
        assert_eq!(Ok(true), c.exists(&aisle_key(&aid)));
    }

    pub fn fill_aisles(c: &Connection) {
        db::products::save_product(&c, &AUTH, "product1", &AisleId(HASH_1.to_owned())).unwrap();
        db::products::save_product(&c, &AUTH, "product2", &AisleId(HASH_1.to_owned())).unwrap();
        db::products::save_product(&c, &AUTH, "product3", &AisleId(HASH_2.to_owned())).unwrap();

        assert_eq!(
            Ok(true),
            c.exists(&db::products::product_key(&ProductId(HASH_1.to_owned())))
        );
        assert_eq!(
            Ok(true),
            c.exists(&db::products::product_key(&ProductId(HASH_2.to_owned())))
        );
        assert_eq!(
            Ok(true),
            c.exists(&db::products::product_key(&ProductId(HASH_3.to_owned())))
        );
        assert_eq!(
            Ok(true),
            c.sismember(
                &db::products::products_in_aisle_key(&AisleId(HASH_1.to_owned())),
                HASH_1.to_owned()
            )
        );
        assert_eq!(
            Ok(true),
            c.sismember(
                &db::products::products_in_aisle_key(&AisleId(HASH_1.to_owned())),
                HASH_2.to_owned()
            )
        );
        assert_eq!(
            Ok(true),
            c.sismember(
                &db::products::products_in_aisle_key(&AisleId(HASH_2.to_owned())),
                HASH_3.to_owned()
            )
        );
    }

    pub fn get_aisles_in_store_for_test(c: &Connection) {
        save_aisle_for_test(&c);
        add_2nd_aisle(&c);
        fill_aisles(&c);

        let expected = vec![
            Aisle::new(
                HASH_1.to_owned(),
                NAME.to_owned(),
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
                RENAMED.to_owned(),
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
        ];
        assert_eq!(
            Ok(expected),
            get_aisles_in_store(&c, &StoreId::new(HASH_1.to_owned()))
        );
    }

    #[test]
    fn get_aisles_in_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();

        get_aisles_in_store_for_test(&c)
    }

    #[test]
    fn delete_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();

        // this create a store, an aisle and put a product in it
        db::products::tests::save_product_for_test(&c);
        let aid = AisleId(HASH_1.to_owned());
        // add another product
        let expected = Product::new(
            HASH_2.to_owned(),
            "product2".to_owned(),
            1,
            false,
            Unit::Unit,
            1f32,
        );
        assert_eq!(
            Ok(expected),
            db::products::save_product(&c, &AUTH, "product2", &aid)
        );

        assert_eq!(Ok(()), delete_aisle(&c, &AUTH, &aid));
        assert_eq!(Ok(false), c.exists(&aisle_key(&aid)));
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&aid))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(HASH_1.to_owned())))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(HASH_2.to_owned())))
        );
    }

    #[test]
    fn transaction_purge_aisles_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();

        save_aisle_for_test(&c);
        add_2nd_aisle(&c);
        fill_aisles(&c);
        let aisle_in_store_key = aisles_in_store_key(&StoreId::new(HASH_1.to_owned()));
        let mut pipe = Pipeline::new(c.db);
        pipe.atomic();
        assert_eq!(
            Ok(()),
            transaction_purge_aisles_in_store(&c, &mut pipe, &StoreId::new(HASH_1.to_owned()))
        );
        assert_eq!(Ok(()), pipe.query(&c));
        assert_eq!(Ok(false), c.exists(&aisle_in_store_key));
        assert_eq!(
            Ok(false),
            c.exists(&db::products::product_key(&ProductId(HASH_1.to_owned())))
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
        assert_eq!(Ok(false), c.exists(&aisle_key(&AisleId(HASH_1.to_owned()))));
        assert_eq!(Ok(false), c.exists(&aisle_key(&AisleId(HASH_2.to_owned()))));
    }

    #[test]
    fn edit_aisle_sort_weight_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let c = client.get_connection().unwrap();

        save_aisle_for_test(&c);
        let mut pipe = Pipeline::new(c.db);
        pipe.atomic();
        assert_eq!(
            Ok(()),
            edit_aisle_sort_weight(
                &c,
                &mut pipe,
                &AUTH,
                &AisleItemWeight::new(HASH_1.to_owned(), 2.0f32)
            )
        );
        assert_eq!(Ok(()), pipe.query(&c));
        assert_eq!(
            Ok(2.0f32),
            c.hget(&aisle_key(&AisleId(HASH_1.to_owned())), AISLE_WEIGHT)
        );
    }
}
