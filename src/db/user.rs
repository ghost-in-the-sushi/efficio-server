use argon2rs;
use hex_view::HexView;
use rand::{self, Rng};
use redis::{self, Commands};

use crate::db::get_connection;
use crate::error::{self, ServerError};
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

pub fn store_user(user: &user::User) -> Result<Token, ServerError> {
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
    let hashed_pwd = hash(&user.password.as_str(), &salt_pwd.as_str());
    let hashed_mail = hash(&user.email.as_str(), &salt_mail.as_str());

    let user_id: i32 = c.incr(NEXT_USER_ID, 1)?;
    let user_key = format!("user:{}", user_id.to_string());
    c.hset_multiple(
      &user_key,
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
    Ok(auth.into())
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
    let user = gen_user();
    let r = store_user(&user);
    assert_eq!(true, r.is_ok());

    reset_db();
  }

  #[test]
  fn store_user_exists_test() {
    let user = gen_user();
    let r = store_user(&user);
    assert_eq!(true, r.is_ok());
    let r = store_user(&user);
    assert_eq!(true, r.is_err());

    reset_db();
  }
}
