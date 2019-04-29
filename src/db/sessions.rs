use redis::{self, Commands, PipelineCommands};

use crate::db::get_connection;
use crate::db::users;
use crate::error::{self, Result, ServerError};
use crate::types::*;

const SESSIONS_LIST: &str = "sessions";

fn user_sessions_key(user_id: &UserId) -> String {
    format!("sessions:{}", **user_id)
}

pub fn get_user_id(c: &redis::Connection, auth: &Auth) -> Result<UserId> {
    let id = c.hget(SESSIONS_LIST, auth.0)?;
    Ok(UserId(id))
}

pub fn store_session(auth: &str, user_id: &UserId) -> Result<()> {
    let c = get_connection()?;
    if c.hexists(SESSIONS_LIST, auth)? {
        Err(ServerError::new(
            error::INTERNAL_ERROR,
            "Auth already exists",
        ))
    } else {
        let user_session_key = user_sessions_key(user_id);
        redis::transaction(&c, &[SESSIONS_LIST, &user_session_key], |pipe| {
            pipe.hset(SESSIONS_LIST, auth, **user_id)
                .ignore()
                .sadd(&user_session_key, auth)
                .query(&c)
        })?;

        Ok(())
    }
}

pub fn validate_session(auth: &Auth) -> Result<()> {
    let c = get_connection()?;
    if c.hexists(SESSIONS_LIST, auth.0)? {
        let user_id = get_user_id(&c, auth)?;
        if c.sismember(&user_sessions_key(&user_id), auth.0)? {
            Ok(())
        } else {
            Err(ServerError::new(
                error::UNAUTHORISED,
                "x-auth-token does not belong to this user",
            ))
        }
    } else {
        Err(ServerError::new(error::UNAUTHORISED, "Not logged in"))
    }
}

fn delete_session_with_connection(
    c: &redis::Connection,
    auth: &Auth,
    user_id: &UserId,
) -> Result<()> {
    let user_session_key = user_sessions_key(user_id);
    Ok(redis::transaction(
        c,
        &[SESSIONS_LIST, &user_session_key],
        |pipe| {
            pipe.hdel(SESSIONS_LIST, auth.0)
                .ignore()
                .srem(&user_session_key, auth.0)
                .query(c)
        },
    )?)
}

pub fn delete_session(auth: &Auth) -> Result<()> {
    let c = get_connection()?;
    // save user_id before deleting the auth from sessions
    let user_id = get_user_id(&c, auth)?;
    delete_session_with_connection(&c, auth, &user_id)?;
    users::regen_auth(&c, &user_id)
}

pub fn delete_all_user_sessions(auth: &Auth) -> Result<()> {
    let c = get_connection()?;
    let user_id = UserId(c.hget(SESSIONS_LIST, auth.0)?);
    let all_user_sessions: Vec<String> = c.smembers(user_sessions_key(&user_id))?;
    all_user_sessions
        .iter()
        .map(|a| delete_session_with_connection(&c, &Auth(a), &user_id))
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db::tests::*;

    pub const AUTH: Auth = Auth("tokenauth");
    pub const AUTH2: Auth = Auth("anothertokenauth");

    pub fn store_session_for_test(auth: &Auth) {
        let user_id = UserId(1);
        assert_eq!(Ok(()), store_session(auth, &user_id));

        let c = get_connection().unwrap();
        let res: bool = c.hexists(SESSIONS_LIST, auth.0).unwrap();
        assert_eq!(true, res);
        let res: bool = c.sismember(&user_sessions_key(&user_id), auth.0).unwrap();
        assert_eq!(true, res);
    }

    fn store_session_test_with_reset() {
        reset_db();
        store_session_for_test(&AUTH);
        assert_eq!(
            Err(ServerError::new(
                error::INTERNAL_ERROR,
                "Auth already exists",
            )),
            store_session(&AUTH, &UserId(1))
        );
    }

    #[test]
    fn validate_session_test() {
        store_session_test_with_reset();
        assert_eq!(Ok(()), validate_session(&AUTH));
        assert_eq!(
            Err(ServerError::new(error::UNAUTHORISED, "Not logged in")),
            validate_session(&Auth("notpresentauth"))
        );
        let c = get_connection().unwrap();
        // tamper user sessions list
        let _: i32 = c.srem(&user_sessions_key(&UserId(1)), AUTH.0).unwrap();
        assert_eq!(
            Err(ServerError::new(
                error::UNAUTHORISED,
                "x-auth-token does not belong to this user",
            )),
            validate_session(&AUTH)
        );
    }

    #[test]
    fn get_user_id_test() {
        store_session_test_with_reset();
        let c = get_connection().unwrap();
        assert_eq!(1, *get_user_id(&c, &AUTH).unwrap());
        store_session_for_test(&AUTH2);
        assert_eq!(1, *get_user_id(&c, &AUTH2).unwrap());
    }

    #[test]
    fn delete_session_test() {
        users::tests::store_user_for_test_with_reset();
        let c = get_connection().unwrap();
        let user_auth: String = c.hget("user:1", "auth").unwrap();
        get_user_id_test();
        assert_eq!(Ok(()), delete_session(&AUTH));
        let new_auth: String = c.hget("user:1", "auth").unwrap();
        // check that we change the auth token on logout
        assert_ne!(user_auth, new_auth);
        let res: bool = c.hexists(SESSIONS_LIST, AUTH.0).unwrap();
        assert_eq!(false, res);
    }

    #[test]
    fn delete_all_user_sessions_test() {
        store_session_test_with_reset();
        assert_eq!(Ok(()), store_session("AUTH2", &UserId(1)));
        assert_eq!(Ok(()), delete_all_user_sessions(&AUTH));
        let c = get_connection().unwrap();
        let res: bool = c.exists(SESSIONS_LIST).unwrap();
        assert_eq!(false, res);
        let res: bool = c.exists(user_sessions_key(&UserId(1))).unwrap();
        assert_eq!(false, res);
    }

}
