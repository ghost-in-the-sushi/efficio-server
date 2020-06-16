#[cfg(not(test))]
use redis::{self, transaction, Commands, Connection, Pipeline};

#[cfg(test)]
use fake_redis::{transaction, FakeConnection as Connection, FakePipeline as Pipeline};

use crate::{db, error::Result, types::*};

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

pub fn get_aisle_owner(c: &mut Connection, aisle_id: &AisleId) -> Result<UserId> {
    Ok(UserId(c.hget(&aisle_key(&aisle_id), AISLE_OWNER)?))
}

pub fn get_aisles_in_store(c: &mut Connection, store_id: &StoreId) -> Result<Vec<Aisle>> {
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
                db::products::get_products_in_aisle(c, &aisle_id)?,
            ))
        })
        .collect()
}

fn find_max_weight_in_store(c: &mut Connection, store_id: &StoreId) -> Result<f32> {
    let aisles = get_aisles_in_store(c, &store_id)?;
    Ok(aisles.iter().max().map_or(0f32, |a| a.sort_weight))
}

pub fn save_aisle(
    c: &mut Connection,
    auth: &Auth,
    store_id: &StoreId,
    name: &str,
) -> Result<Aisle> {
    let aisle_id = db::ids::get_next_aisle_id();
    let aisle_key = aisle_key(&aisle_id);
    let aisle_in_store_key = aisles_in_store_key(&store_id);
    let user_id = db::sessions::get_user_id(c, &auth)?;
    let store_owner = db::stores::get_store_owner(c, &store_id)?;
    db::verify_permission(&user_id, &store_owner)?;
    let new_sort_weight = find_max_weight_in_store(c, &store_id)? + 1f32;
    transaction(c, &[&aisle_key, &aisle_in_store_key], |c, pipe| {
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

pub fn edit_aisle(
    c: &mut Connection,
    auth: &Auth,
    aisle_id: &AisleId,
    new_name: &str,
) -> Result<()> {
    let aisle_key = aisle_key(&aisle_id);
    let aisle_owner = get_aisle_owner(c, &aisle_id)?;
    db::verify_permission_auth(c, &auth, &aisle_owner)?;
    Ok(c.hset(&aisle_key, AISLE_NAME, new_name)?)
}

pub fn delete_aisle(c: &mut Connection, auth: &Auth, aisle_id: &AisleId) -> Result<()> {
    let aisle_key = aisle_key(&aisle_id);
    let aisle_owner = get_aisle_owner(c, &aisle_id)?;
    db::verify_permission_auth(c, &auth, &aisle_owner)?;
    let store_id = StoreId::new(c.hget(&aisle_key, AISLE_STORE)?);
    let aisle_in_store_key = aisles_in_store_key(&store_id);
    transaction(c, &[&aisle_key, &aisle_in_store_key], |c, mut pipe| {
        db::products::transaction_purge_products_in_aisle(c, &mut pipe, &aisle_id)?;
        pipe.srem(&aisle_in_store_key, &**aisle_id)
            .ignore()
            .del(&aisle_key)
            .query(c)
    })?;
    Ok(())
}

pub fn transaction_purge_aisles_in_store(
    c: &mut Connection,
    mut pipe: &mut Pipeline,
    store_id: &StoreId,
) -> Result<()> {
    let aisles_in_store_key = aisles_in_store_key(&store_id);
    let aisles: Option<Vec<String>> = c.smembers(&aisles_in_store_key)?;
    if let Some(aisles) = aisles {
        for aisle_id in aisles {
            let aisle_id = AisleId(aisle_id);
            db::products::transaction_purge_products_in_aisle(c, &mut pipe, &aisle_id)?;
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
    c: &mut Connection,
    pipe: &mut Pipeline,
    auth: &Auth,
    data: &AisleItemWeight,
) -> Result<()> {
    let aisle_id = AisleId(data.id.clone());
    let aisle_owner = get_aisle_owner(c, &aisle_id)?;
    db::verify_permission_auth(c, &auth, &aisle_owner)?;
    let aisle_key = aisle_key(&aisle_id);
    pipe.hset(&aisle_key, AISLE_WEIGHT, data.sort_weight)
        .ignore();
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db::{self, sessions::tests::*, stores::tests::*, tests::*};
    use fake_redis::FakeCient as Client;

    pub const NAME: &str = "Aisle1";
    const RENAMED: &str = "AisleRenamed";

    pub fn aisles_in_store_key(store_id: &StoreId) -> String {
        super::aisles_in_store_key(&store_id)
    }

    pub fn aisle_key(aisle_id: &AisleId) -> String {
        super::aisle_key(&aisle_id)
    }

    pub fn save_aisle_for_test(c: &mut Connection) -> (StoreId, AisleId) {
        let store_id = save_store_for_test(c);
        let expected = Aisle::new("".to_owned(), NAME.to_owned(), 0f32, vec![]);
        let res = save_aisle(c, &AUTH, &store_id, NAME);
        assert_eq!(Ok(expected), res);
        (store_id, AisleId(res.unwrap().id().to_string()))
    }

    // create a user, a session with AUTH as token, a store and an aisle
    #[test]
    fn save_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let (store_id, aisle_id) = save_aisle_for_test(&mut c);

        // check DB
        let key = aisle_key(&aisle_id);
        assert_eq!(Ok(true), c.exists(&key));
        assert_eq!(Ok(true), c.exists(&aisles_in_store_key(&store_id)));
        assert_eq!(Ok(NAME.to_string()), c.hget(&key, AISLE_NAME));
        let weight: f32 = c.hget(&key, AISLE_WEIGHT).unwrap();
        assert!(weight - 1.0f32 < std::f32::EPSILON);
        assert_eq!(
            Ok(true),
            c.sismember(&aisles_in_store_key(&store_id), aisle_id.to_string())
        );
    }

    #[test]
    fn edit_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let (_, aid) = save_aisle_for_test(&mut c);
        assert_eq!(Ok(()), edit_aisle(&mut c, &AUTH, &aid, RENAMED));

        let name: String = c.hget(&aisle_key(&aid), AISLE_NAME).unwrap();
        assert_eq!(RENAMED, name.as_str());
    }

    pub fn add_2nd_aisle(c: &mut Connection, store_id: &StoreId) -> AisleId {
        let expected = Aisle::new("".to_owned(), RENAMED.to_owned(), 0f32, vec![]);
        let res = save_aisle(c, &AUTH, &store_id, RENAMED);
        assert_eq!(Ok(expected), res);
        let aid = AisleId(res.unwrap().id().to_string());
        assert_eq!(Ok(true), c.exists(&aisle_key(&aid)));
        aid
    }

    pub fn fill_aisles(
        c: &mut Connection,
        aisle1: &AisleId,
        aisle2: &AisleId,
    ) -> (ProductId, ProductId, ProductId) {
        let p1 = db::products::save_product(c, &AUTH, "product1", &aisle1).unwrap();
        let p2 = db::products::save_product(c, &AUTH, "product2", &aisle1).unwrap();
        let p3 = db::products::save_product(c, &AUTH, "product3", &aisle2).unwrap();

        assert_eq!(Ok(true), c.exists(&db::products::product_key(&p1.id())));
        assert_eq!(Ok(true), c.exists(&db::products::product_key(&p2.id())));
        assert_eq!(Ok(true), c.exists(&db::products::product_key(&p3.id())));
        assert_eq!(
            Ok(true),
            c.sismember(
                &db::products::products_in_aisle_key(&aisle1),
                p1.id().to_string(),
            )
        );
        assert_eq!(
            Ok(true),
            c.sismember(
                &db::products::products_in_aisle_key(&aisle1),
                p2.id().to_string(),
            )
        );
        assert_eq!(
            Ok(true),
            c.sismember(
                &db::products::products_in_aisle_key(&aisle2),
                p3.id().to_string()
            )
        );
        (p1.id(), p2.id(), p3.id())
    }

    pub fn get_aisles_in_store_for_test(c: &mut Connection) -> StoreId {
        let (store_id, aisle_id) = save_aisle_for_test(c);
        let aisle_id2 = add_2nd_aisle(c, &store_id);
        fill_aisles(c, &aisle_id, &aisle_id2);

        let expected = vec![
            Aisle::new(
                "".to_owned(),
                NAME.to_owned(),
                0f32,
                vec![
                    Product::new(
                        "".to_string(),
                        "product1".to_owned(),
                        1,
                        false,
                        Unit::Unit,
                        0f32,
                    ),
                    Product::new(
                        "".to_string(),
                        "product2".to_owned(),
                        1,
                        false,
                        Unit::Unit,
                        0f32,
                    ),
                ],
            ),
            Aisle::new(
                "".to_string(),
                RENAMED.to_owned(),
                0f32,
                vec![Product::new(
                    "".to_string(),
                    "product3".to_owned(),
                    1,
                    false,
                    Unit::Unit,
                    0f32,
                )],
            ),
        ];
        assert_eq!(Ok(expected), get_aisles_in_store(c, &store_id));
        store_id
    }

    #[test]
    fn get_aisles_in_store_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        get_aisles_in_store_for_test(&mut c);
    }

    #[test]
    fn delete_aisle_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        // this create a store, an aisle and put a product in it
        let (aid, pid1) = db::products::tests::save_product_for_test(&mut c);
        // add another product
        let expected = Product::new(
            "".to_string(),
            "product2".to_owned(),
            1,
            false,
            Unit::Unit,
            1f32,
        );
        let res = db::products::save_product(&mut c, &AUTH, "product2", &aid);
        assert_eq!(Ok(expected), res);
        let pid2 = res.unwrap().id();
        assert_eq!(Ok(()), delete_aisle(&mut c, &AUTH, &aid));
        assert_eq!(Ok(false), c.exists(&aisle_key(&aid)));
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&aid))
        );
        assert_eq!(Ok(false), c.exists(&db::products::product_key(&pid1)));
        assert_eq!(Ok(false), c.exists(&db::products::product_key(&pid2)));
    }

    #[test]
    fn transaction_purge_aisles_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        let (store_id, aisle_id1) = save_aisle_for_test(&mut c);
        let aid2 = add_2nd_aisle(&mut c, &store_id);
        let (p1, p2, p3) = fill_aisles(&mut c, &aisle_id1, &aid2);
        let aisle_in_store_key = aisles_in_store_key(&store_id);
        let mut pipe = Pipeline::new(c.db);
        pipe.atomic();
        assert_eq!(
            Ok(()),
            transaction_purge_aisles_in_store(&mut c, &mut pipe, &store_id)
        );
        assert_eq!(Ok(()), pipe.query(&mut c));
        assert_eq!(Ok(false), c.exists(&aisle_in_store_key));
        assert_eq!(Ok(false), c.exists(&db::products::product_key(&p1)));
        assert_eq!(Ok(false), c.exists(&db::products::product_key(&p2)));
        assert_eq!(Ok(false), c.exists(&db::products::product_key(&p3)));
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&aisle_id1))
        );
        assert_eq!(
            Ok(false),
            c.exists(&db::products::products_in_aisle_key(&aid2))
        );
        assert_eq!(Ok(false), c.exists(&aisle_key(&aisle_id1)));
        assert_eq!(Ok(false), c.exists(&aisle_key(&aid2)));
    }

    #[test]
    fn edit_aisle_sort_weight_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();

        let (_, aisle_id) = save_aisle_for_test(&mut c);
        let mut pipe = Pipeline::new(c.db);
        pipe.atomic();
        assert_eq!(
            Ok(()),
            edit_aisle_sort_weight(
                &mut c,
                &mut pipe,
                &AUTH,
                &AisleItemWeight::new(aisle_id.to_string(), 2.0f32)
            )
        );
        assert_eq!(Ok(()), pipe.query(&mut c));
        assert_eq!(Ok(2.0f32), c.hget(&aisle_key(&aisle_id), AISLE_WEIGHT));
    }
}
