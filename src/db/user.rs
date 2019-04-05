use argon2rs;
use hex_view::HexView;
use rand::{self, Rng};
use redis::{self, Commands};

use crate::db::get_connection;
use crate::db::sessions;
use crate::error::{self, Result, ServerError};
use crate::sessions::AuthInfo;
use crate::token::Token;
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

fn user_key(user_id: u32) -> String {
  format!("user:{}", user_id.to_string())
}

pub fn store_user(user: &user::User) -> Result<Token> {
  let c = get_connection()?;
  if c.hexists(USERS_LIST, &user.username)? {
    Err(ServerError {
      status: error::USERNAME_TAKEN,
      msg: format!("Username {} is not available.", &user.username),
    })
  } else {
    let mut rng = rand::thread_rng();
    let mut auth = [0u8; 32];
    rng.fill(&mut auth[..]);
    let auth = format!("{:x}", HexView::from(&auth));
    let salt_mail = rng.gen::<u64>().to_string();
    let salt_pwd = rng.gen::<u64>().to_string();
    let hashed_pwd = hash(&user.password, &salt_pwd);
    let hashed_mail = hash(&user.email, &salt_mail);

    let user_id: u32 = c.incr(NEXT_USER_ID, 1)?;
    c.hset_multiple(
      &user_key(user_id),
      &[
        (USER_NAME, &user.username),
        (USER_MAIL, &hashed_mail),
        (USER_PWD, &hashed_pwd),
        (USER_SALT_M, &salt_mail),
        (USER_SALT_P, &salt_pwd),
        (USER_AUTH, &auth),
      ],
    )?;
    c.hset(USERS_LIST, &user.username, user_id)?;
    sessions::store_session(&auth, user_id)?;
    Ok(auth.into())
  }
}

pub fn verify_password(auth_info: &AuthInfo) -> Result<(Token, u32)> {
  let c = get_connection()?;
  let user_id: u32 = c.hget(USERS_LIST, &auth_info.username).or_else(|_| {
    Err(ServerError {
      status: error::INVALID_USER_OR_PWD,
      msg: "Invalid usename or password".to_string(),
    })
  })?;
  let user_key = user_key(user_id);
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

#[cfg(test)]
mod tests {
  use super::*;

  fn reset_db() {
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

  #[test]
  fn store_user_test() {
    reset_db();
    let user = gen_user();
    let r = store_user(&user);
    assert_eq!(true, r.is_ok());
  }

  #[test]
  fn store_user_exists_test() {
    reset_db();
    let user = gen_user();
    let r = store_user(&user);
    assert_eq!(true, r.is_ok());
    let r = store_user(&user);
    assert_eq!(true, r.is_err());
  }

  #[test]
  fn login_test() {
    reset_db();
    let user = gen_user();
    let r = store_user(&user);
    assert_eq!(true, r.is_ok());

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
}
