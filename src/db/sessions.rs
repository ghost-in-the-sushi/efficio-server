use redis::{self, Commands, PipelineCommands};

use crate::db::get_connection;
use crate::db::users;
use crate::error::{self, Result, ServerError};
use crate::types::*;

const SESSIONS_LIST: &str = "sessions";

fn user_session_key(user_id: &UserId) -> String {
  format!("sessions:{}", user_id.to_string())
}

pub fn get_user_id(c: &redis::Connection, auth: &str) -> Result<UserId> {
  Ok(UserId(c.hget(SESSIONS_LIST, auth)?))
}

pub fn store_session(auth: &str, user_id: &UserId) -> Result<()> {
  let c = get_connection()?;
  if c.hexists(SESSIONS_LIST, auth)? {
    Err(ServerError {
      status: error::INTERNAL_ERROR,
      msg: "Auth already exists".to_string(),
    })
  } else {
    let user_session_key = user_session_key(user_id);
    redis::transaction(&c, &[SESSIONS_LIST, &user_session_key], |pipe| {
      pipe
        .hset(SESSIONS_LIST, auth, user_id.0)
        .ignore()
        .sadd(&user_session_key, auth)
        .query(&c)
    })?;

    Ok(())
  }
}

pub fn validate_session(auth: &str) -> Result<()> {
  let c = get_connection()?;
  if c.hexists(SESSIONS_LIST, auth)? {
    Ok(())
  } else {
    Err(ServerError {
      status: error::UNAUTHORISED,
      msg: "Not logged in".to_string(),
    })
  }
}

fn delete_session_with_connection(
  c: &redis::Connection,
  auth: &str,
  user_id: &UserId,
) -> Result<()> {
  let user_session_key = user_session_key(user_id);
  Ok(redis::transaction(
    c,
    &[SESSIONS_LIST, &user_session_key],
    |pipe| {
      pipe
        .hdel(SESSIONS_LIST, auth)
        .ignore()
        .srem(&user_session_key, auth)
        .query(c)
    },
  )?)
}

pub fn delete_session(auth: &str) -> Result<()> {
  let c = get_connection()?;
  // save user_id before deleting the auth from sessions
  let user_id = get_user_id(&c, auth)?;
  delete_session_with_connection(&c, auth, &user_id)?;
  users::regen_auth(&c, &user_id)
}

pub fn delete_all_user_sessions(auth: &str) -> Result<()> {
  let c = get_connection()?;
  let user_id = UserId(c.hget(SESSIONS_LIST, auth)?);
  let all_user_sessions: Vec<String> = c.smembers(user_session_key(&user_id))?;
  all_user_sessions
    .iter()
    .map(|a| delete_session_with_connection(&c, &a, &user_id))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const AUTH: &str = "tokenauth";

  fn store_session_test() {
    users::tests::reset_db();
    assert_eq!(true, store_session(AUTH, &UserId(1)).is_ok());
    let c = get_connection().unwrap();
    let res: bool = c.hexists(SESSIONS_LIST, AUTH).unwrap();
    assert_eq!(true, res);
    assert_eq!(false, store_session(AUTH, &UserId(1)).is_ok());
  }

  #[test]
  fn validate_session_test() {
    store_session_test();
    assert_eq!(true, validate_session(AUTH).is_ok());
    assert_eq!(false, validate_session("notpresentauth").is_ok());
  }

  #[test]
  fn get_user_id_test() {
    store_session_test();
    let c = get_connection().unwrap();
    assert_eq!(1, get_user_id(&c, AUTH).unwrap().0);
  }

  #[test]
  fn delete_session_test() {
    users::tests::store_user_for_test();
    let c = get_connection().unwrap();
    let user_auth: String = c.hget("user:1", "auth").unwrap();
    get_user_id_test();
    assert_eq!(true, delete_session(AUTH).is_ok());
    let new_auth: String = c.hget("user:1", "auth").unwrap();
    // check that we change the auth token on logout
    assert_ne!(user_auth, new_auth);
    let res: bool = c.hexists(SESSIONS_LIST, AUTH).unwrap();
    assert_eq!(false, res);
  }

  #[test]
  fn delete_all_user_sessions_test() {
    store_session_test();
    assert_eq!(true, store_session("AUTH2", &UserId(1)).is_ok());
    assert_eq!(true, delete_all_user_sessions(AUTH).is_ok());
    let c = get_connection().unwrap();
    let res: bool = c.exists(SESSIONS_LIST).unwrap();
    assert_eq!(false, res);
    let res: bool = c.exists(user_session_key(&UserId(1))).unwrap();
    assert_eq!(false, res);
  }

}
