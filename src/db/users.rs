use argon2rs;
use hex_view::HexView;
use rand::{self, Rng};

#[cfg(not(test))]
use redis::{self, Commands};

use crate::db;
use crate::error::{self, Result, ServerError};
use crate::types::*;

const NEXT_USER_ID: &str = "next_user_id";
const USER_PWD: &str = "password";
const USER_MAIL: &str = "email";
const USER_SALT_M: &str = "salt_mail";
const USER_SALT_P: &str = "salt_password";
const USER_NAME: &str = "username";
const USERS_LIST: &str = "users";

fn hash(data: &str, salt: &str) -> String {
    format!(
        "{:x}",
        HexView::from(&argon2rs::argon2i_simple(&data, &salt))
    )
}

fn user_key(user_id: &UserId) -> String {
    format!("user:{}", **user_id)
}

fn gen_auth(rng: &mut rand::rngs::ThreadRng) -> String {
    let mut auth = [0u8; 32];
    rng.fill(&mut auth[..]);
    format!("{:x}", HexView::from(&auth))
}

pub fn save_user(user: &User) -> Result<Token> {
    let c = db::get_connection()?;
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
        let hashed_pwd = hash(&user.password, &salt_pwd);
        let hashed_mail = hash(&user.email, &salt_mail);
        let user_id = UserId(c.incr(NEXT_USER_ID, 1)?);
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
        c.hset(USERS_LIST, &norm_username, *user_id)?;
        let auth = gen_auth(&mut rng);
        db::sessions::store_session(&auth, &user_id)?;
        Ok(auth.into())
    }
}

pub fn delete_user(auth: &Auth) -> Result<()> {
    let c = db::get_connection()?;
    let user_id = db::sessions::get_user_id(&c, auth)?;
    let user_key = user_key(&user_id);
    let username: String = c.hget(&user_key, USER_NAME)?;
    db::stores::delete_all_user_stores(&auth)?;
    c.hdel(USERS_LIST, &username.to_lowercase())?;
    db::sessions::delete_all_user_sessions(auth)?;
    Ok(c.del(&user_key)?)
}

pub fn login(auth_info: &AuthInfo) -> Result<(Token, UserId)> {
    let c = db::get_connection()?;
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
    let hashed_pwd = hash(&auth_info.password, &salt_pwd);
    if hashed_pwd == stored_pwd {
        let mut rng = rand::thread_rng();
        Ok((gen_auth(&mut rng).into(), user_id))
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
    use crate::db::tests::*;

    pub fn gen_user() -> User {
        User {
            username: "toto".to_string(),
            password: "pwd".to_string(),
            email: "m@m.com".to_string(),
        }
    }

    pub fn store_user_for_test() -> Token {
        let user = gen_user();
        let res = save_user(&user);
        if res.is_err() {
            dbg!(&res);
        }
        assert_eq!(true, res.is_ok());
        res.unwrap()
    }

    pub fn store_user_for_test_with_reset() -> Token {
        reset_db();
        store_user_for_test()
    }

    #[test]
    fn store_user_test() {
        let token = store_user_for_test_with_reset();
        let user = gen_user();
        let c = db::get_connection().unwrap();
        assert_eq!(Ok(true), c.exists("user:1"));
        assert_eq!(Ok(true), c.exists("sessions:1"));
        assert_eq!(Ok(true), c.sismember("sessions:1", token.session_token));
        assert_eq!(Ok(1), c.get("next_user_id"));
        assert_eq!(Ok(true), c.hexists("users", "toto"));
        assert_eq!(Ok(1), c.hget("users", "toto"));

        assert_eq!(
            Ok(true),
            c.hexists(USERS_LIST, &user.username.to_lowercase())
        );
    }

    #[test]
    fn store_user_exists_test() {
        store_user_test();
        let mut user = gen_user();
        let res = save_user(&user);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());
        user.username = "ToTo".to_string(); // username uniqueness should be case insensitive
        let res = save_user(&user);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());
    }

    #[test]
    fn login_test() {
        store_user_test();

        let login_data = AuthInfo {
            username: "toto".to_string(),
            password: "pwd".to_string(),
        };
        let res = login(&login_data);
        if res.is_err() {
            dbg!(&res);
        }
        assert_eq!(true, res.is_ok());

        let login_data = AuthInfo {
            username: "toto".to_string(),
            password: "pwdb".to_string(),
        };
        let res = login(&login_data);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());

        let login_data = AuthInfo {
            username: "tato".to_string(),
            password: "pwd".to_string(),
        };
        let res = login(&login_data);
        if res.is_ok() {
            dbg!(&res);
        }
        assert_eq!(false, res.is_ok());
    }

    // pub fn delete_user(auth: &Auth) -> Result<()> {
    //     let c = db::get_connection()?;
    //     let user_id = db::sessions::get_user_id(&c, auth)?;
    //     let user_key = user_key(&user_id);
    //     let username: String = c.hget(&user_key, USER_NAME)?;
    //     db::stores::delete_all_user_stores(&auth)?;
    //     c.hdel(USERS_LIST, &username.to_lowercase())?;
    //     db::sessions::delete_all_user_sessions(auth)?;
    //     Ok(c.del(&user_key)?)
    // }
    #[test]
    fn delete_user_test() {
        let token = store_user_for_test_with_reset();
        let c = db::get_connection().unwrap();
        let auth = Auth(&token.session_token);
        assert_eq!(Ok(()), delete_user(&auth));
        assert_eq!(Ok(false), c.exists(USERS_LIST));
        assert_eq!(Ok(false), c.exists("user:1"));

        store_user_for_test(); // create toto user as user:2
        let mut user = gen_user();
        user.username = "tata".to_string();
        let res = save_user(&user); // create tata user as user:3
        if res.is_err() {
            dbg!(&res);
        }
        assert_eq!(true, res.is_ok());
        let token = res.unwrap();
        let auth = Auth(&token.session_token);
        assert_eq!(Ok(()), delete_user(&auth)); // delete tata
        assert_eq!(Ok(false), c.hexists(USERS_LIST, "tata"));
        assert_eq!(Ok(true), c.hexists(USERS_LIST, "toto"));
        assert_eq!(Ok(false), c.exists("user:1"));
        assert_eq!(Ok(true), c.exists("user:2"));
        assert_eq!(Ok(false), c.exists("user:3"));
    }
}
