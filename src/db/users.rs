use hex_view::HexView;
use rand::{self, Rng};

#[cfg(test)]
use fake_redis::FakeConnection as Connection;
#[cfg(not(test))]
use redis::{self, Commands, Connection};

use crate::db;
use crate::error::{self, *};
use crate::types::*;

const USER_PWD: &str = "password";
const USER_MAIL: &str = "email";
const USER_SALT_M: &str = "salt_mail";
const USER_SALT_P: &str = "salt_password";
const USER_NAME: &str = "username";
const USERS_LIST: &str = "users";

fn user_key(user_id: &UserId) -> String {
    format!("user:{}", **user_id)
}

fn gen_auth(rng: &mut rand::rngs::ThreadRng) -> String {
    let mut auth = [0u8; 32];
    rng.fill(&mut auth[..]);
    format!("{:x}", HexView::from(&auth))
}

pub fn save_user(c: &mut Connection, user: &User) -> Result<ConnectionToken> {
    let norm_username = user.username.to_lowercase();
    if c.hexists(USERS_LIST, &norm_username)? {
        Err(ServerError::new(
            error::USERNAME_TAKEN,
            &format!("Username {} is not available.", &user.username),
        ))
    } else {
        let mut rng = rand::thread_rng();
        let salt_mail = rng.gen::<u64>().to_string();
        let salt_pwd = rng.gen::<u64>().to_string();
        let hashed_pwd = db::ids::hash(&user.password, &salt_pwd);
        let hashed_mail = db::ids::hash(&user.email, &salt_mail);
        let user_id = db::ids::get_next_user_id(c)?;
        c.hset_multiple(
            &user_key(&user_id),
            &[
                (USER_NAME, &user.username),
                (USER_MAIL, &hashed_mail),
                (USER_PWD, &hashed_pwd),
                (USER_SALT_M, &salt_mail),
                (USER_SALT_P, &salt_pwd),
            ],
        )?;
        c.hset(USERS_LIST, &norm_username, user_id.to_string())?;
        let auth = gen_auth(&mut rng);
        db::sessions::store_session(c, &auth, &user_id)?;
        Ok(ConnectionToken::new(auth.into(), user_id.to_string()))
    }
}

pub fn delete_user(c: &mut Connection, auth: &Auth, wanted_user_id: &UserId) -> Result<()> {
    let user_id = db::sessions::get_user_id(c, auth)?;
    if user_id == *wanted_user_id {
        let user_key = user_key(&user_id);
        let username: String = c.hget(&user_key, USER_NAME)?;
        db::stores::delete_all_user_stores(c, &auth)?;
        c.hdel(USERS_LIST, &username.to_lowercase())?;
        db::sessions::delete_all_user_sessions(c, auth)?;
        Ok(c.del(&user_key)?)
    } else {
        Err(ServerError::new(
            error::UNAUTHORISED,
            "x-auth-token does not belong to this user",
        ))
    }
}

pub fn login(c: &mut Connection, auth_info: &AuthInfo) -> Result<ConnectionToken> {
    let user_id = UserId(
        c.hget(USERS_LIST, &auth_info.username.to_lowercase())
            .or_else(|_| {
                Err(ServerError::new(
                    error::INVALID_USER_OR_PWD,
                    "Invalid usename or password",
                ))
            })?,
    );
    let user_key = user_key(&user_id);
    let salt_pwd: String = c.hget(&user_key, USER_SALT_P)?;
    let stored_pwd: String = c.hget(&user_key, USER_PWD)?;
    let hashed_pwd = db::ids::hash(&auth_info.password, &salt_pwd);
    if hashed_pwd == stored_pwd {
        let mut rng = rand::thread_rng();
        let auth = gen_auth(&mut rng);
        db::sessions::store_session(c, &auth, &user_id)?;
        Ok(ConnectionToken::new(auth, user_id.to_string()))
    } else {
        Err(ServerError::new(
            error::INVALID_USER_OR_PWD,
            "Invalid usename or password",
        ))
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db::{ids::tests::*, tests::*};
    use fake_redis::FakeCient as Client;

    pub fn gen_user() -> User {
        User {
            username: "toto".to_string(),
            password: "pwd".to_string(),
            email: "m@m.com".to_string(),
        }
    }

    pub fn store_user_for_test(c: &mut Connection) -> ConnectionToken {
        let user = gen_user();
        let res = save_user(c, &user);
        if res.is_err() {
            dbg!(&res);
        }
        assert_eq!(true, res.is_ok());
        res.unwrap()
    }

    #[test]
    fn store_user_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let token = store_user_for_test(&mut c);
        let user = gen_user();
        assert_eq!(Ok(true), c.exists(&format!("user:{}", HASH_1)));
        assert_eq!(Ok(true), c.exists(&format!("sessions:{}", HASH_1)));
        assert_eq!(
            Ok(true),
            c.sismember(&format!("sessions:{}", HASH_1), token.session_token)
        );
        assert_eq!(Ok(1), c.get("next_user_id"));
        assert_eq!(Ok(true), c.hexists("users", "toto"));
        assert_eq!(Ok(HASH_1.to_owned()), c.hget("users", "toto"));

        assert_eq!(
            Ok(true),
            c.hexists(USERS_LIST, &user.username.to_lowercase())
        );
    }

    #[test]
    fn store_user_exists_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        store_user_for_test(&mut c);
        let mut user = gen_user();
        let res = save_user(&mut c, &user);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());
        user.username = "ToTo".to_string(); // username uniqueness should be case insensitive
        let res = save_user(&mut c, &user);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());
    }

    #[test]
    fn login_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        store_user_for_test(&mut c);

        let login_data = AuthInfo {
            username: "toto".to_string(),
            password: "pwd".to_string(),
        };
        let res = login(&mut c, &login_data);
        if res.is_err() {
            dbg!(&res);
        }
        assert_eq!(true, res.is_ok());

        let login_data = AuthInfo {
            username: "toto".to_string(),
            password: "pwdb".to_string(),
        };
        let res = login(&mut c, &login_data);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());

        let login_data = AuthInfo {
            username: "tato".to_string(),
            password: "pwd".to_string(),
        };
        let res = login(&mut c, &login_data);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());
    }

    #[test]
    fn delete_user_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        let token = store_user_for_test(&mut c);
        let auth = Auth(&token.session_token);
        assert_eq!(
            Ok(()),
            delete_user(&mut c, &auth, &UserId(HASH_1.to_owned()))
        );
        assert_eq!(Ok(false), c.exists(USERS_LIST));
        assert_eq!(Ok(false), c.exists(&format!("user:{}", HASH_1)));

        store_user_for_test(&mut c); // create toto user as user:2
        let mut user = gen_user();
        user.username = "tata".to_string();
        let res = save_user(&mut c, &user); // create tata user as user:3
        if res.is_err() {
            dbg!(&res);
        }
        assert_eq!(true, res.is_ok());
        let token = res.unwrap();
        let auth = Auth(&token.session_token);
        assert_eq!(
            Ok(()),
            delete_user(&mut c, &auth, &UserId(HASH_3.to_owned()))
        ); // delete tata
        assert_eq!(Ok(false), c.hexists(USERS_LIST, "tata"));
        assert_eq!(Ok(true), c.hexists(USERS_LIST, "toto"));
        assert_eq!(Ok(false), c.exists(&format!("user:{}", HASH_1)));
        assert_eq!(Ok(true), c.exists(&format!("user:{}", HASH_2)));
        assert_eq!(Ok(false), c.exists(&format!("user:{}", HASH_3)));
    }
}
