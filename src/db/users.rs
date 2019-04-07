use argon2rs;
use hex_view::HexView;
use rand::{self, Rng};
use redis::{self, Commands};

use crate::db::{get_connection, sessions};
use crate::error::{self, Result, ServerError};
use crate::session::AuthInfo;
use crate::token::Token;
use crate::types::*;
use crate::user;

const NEXT_USER_ID: &str = "next_user_id";
const USER_PWD: &str = "password";
const USER_MAIL: &str = "email";
const USER_SALT_M: &str = "salt_mail";
const USER_SALT_P: &str = "salt_password";
const USER_NAME: &str = "username";
const USER_AUTH: &str = "auth";
const USERS_LIST: &str = "users";

fn hash(data: &str, salt: &str) -> String {
  format!(
    "{:x}",
    HexView::from(&argon2rs::argon2i_simple(&data, &salt))
  )
}

fn user_key(user_id: &UserId) -> String {
  format!("user:{}", user_id.to_string())
}

fn gen_auth(rng: &mut rand::rngs::ThreadRng) -> String {
  let mut auth = [0u8; 32];
  rng.fill(&mut auth[..]);
  format!("{:x}", HexView::from(&auth))
}

pub fn save_user(user: &user::User) -> Result<Token> {
  let c = get_connection()?;
  let norm_username = user.username.to_lowercase();
  if c.hexists(USERS_LIST, &norm_username)? {
    Err(ServerError {
      status: error::USERNAME_TAKEN,
      msg: format!("Username {} is not available.", &user.username),
    })
  } else {
    let mut rng = rand::thread_rng();
    let auth = gen_auth(&mut rng);
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
        (USER_AUTH, &auth),
      ],
    )?;
    c.hset(USERS_LIST, &norm_username, user_id.0)?;
    sessions::store_session(&auth, &user_id)?;
    Ok(auth.into())
  }
}

pub fn delete_user(auth: &str) -> Result<()> {
  let c = get_connection()?;
  let user_id = sessions::get_user_id(&c, auth)?;
  let user_key = user_key(&user_id);
  let username: String = c.hget(&user_key, USER_NAME)?;
  c.hdel(USERS_LIST, username.to_lowercase())?;
  sessions::delete_all_user_sessions(auth)?;
  // TODO delete all future user dependent data
  Ok(c.del(&user_key)?)
}

pub fn verify_password(auth_info: &AuthInfo) -> Result<(Token, UserId)> {
  let c = get_connection()?;
  let user_id = UserId(
    c.hget(USERS_LIST, &auth_info.username.to_lowercase())
      .or_else(|_| {
        Err(ServerError {
          status: error::INVALID_USER_OR_PWD,
          msg: "Invalid usename or password".to_string(),
        })
      })?,
  );
  let user_key = user_key(&user_id);
  let salt_pwd: String = c.hget(&user_key, USER_SALT_P)?;
  let stored_pwd: String = c.hget(&user_key, USER_PWD)?;
  let hashed_pwd = hash(&auth_info.password, &salt_pwd);
  if hashed_pwd == stored_pwd {
    let auth: String = c.hget(&user_key, USER_AUTH)?;
    Ok((auth.into(), user_id))
  } else {
    Err(ServerError {
      status: error::INVALID_USER_OR_PWD,
      msg: "Invalid usename or password".to_string(),
    })
  }
}

pub fn regen_auth(c: &redis::Connection, user_id: &UserId) -> Result<()> {
  let mut rng = rand::thread_rng();
  c.hset(&user_key(user_id), USER_AUTH, gen_auth(&mut rng))?;
  Ok(())
}

#[cfg(test)]
pub mod tests {
  use super::*;

  pub fn reset_db() {
    let c = get_connection().expect("should have connection");
    let _: () = redis::cmd("FLUSHDB").query(&c).expect("error on flush");
  }

  fn gen_user() -> user::User {
    user::User {
      username: "toto".to_string(),
      password: "pwd".to_string(),
      email: "m@m.com".to_string(),
    }
  }

  pub fn store_user_for_test() {
    reset_db();
    let user = gen_user();
    assert_eq!(true, save_user(&user).is_ok());
  }

  fn store_user_test() {
    store_user_for_test();
    let user = gen_user();
    let c = get_connection().unwrap();
    let res: bool = c.exists("user:1").unwrap();
    assert_eq!(true, res);
    let res: bool = c
      .hexists(USERS_LIST, &user.username.to_lowercase())
      .unwrap();
    assert_eq!(true, res);
  }

  #[test]
  fn store_user_exists_test() {
    store_user_test();
    let mut user = gen_user();
    assert_eq!(false, save_user(&user).is_ok());
    user.username = "ToTo".to_string(); // username uniqueness should be case insensitive
    assert_eq!(false, save_user(&user).is_ok());
  }

  #[test]
  fn login_test() {
    store_user_test();

    let login = AuthInfo {
      username: "toto".to_string(),
      password: "pwd".to_string(),
    };
    assert_eq!(true, verify_password(&login).is_ok());

    let login = AuthInfo {
      username: "toto".to_string(),
      password: "pwdb".to_string(),
    };
    assert_eq!(false, verify_password(&login).is_ok());

    let login = AuthInfo {
      username: "tato".to_string(),
      password: "pwd".to_string(),
    };
    assert_eq!(false, verify_password(&login).is_ok());
  }

  #[test]
  fn delete_user_test() {
    store_user_test();
    let c = get_connection().unwrap();
    let auth: String = c.hget(&user_key(&UserId(1)), USER_AUTH).unwrap();
    assert_eq!(true, delete_user(&auth).is_ok());
    let res: bool = c.exists(USERS_LIST).unwrap();
    assert_eq!(false, res);
    store_user_test();
    let mut user = gen_user();
    user.username = "tata".to_string();
    assert_eq!(true, save_user(&user).is_ok());
    let auth: String = c.hget(&user_key(&UserId(1)), USER_AUTH).unwrap();
    assert_eq!(true, delete_user(&auth).is_ok());
    let res: bool = c.hexists(USERS_LIST, &user.username).unwrap();
    assert_eq!(true, res);
    let res: bool = c.hexists(USERS_LIST, "toto").unwrap();
    assert_eq!(false, res);
    let res: bool = c.exists("user:1").unwrap();
    assert_eq!(false, res);
  }
}
