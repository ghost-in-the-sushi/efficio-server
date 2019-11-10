#[cfg(not(test))]
use redis::{self, transaction, Commands, Connection, PipelineCommands};

#[cfg(test)]
use fake_redis::{transaction, FakeConnection as Connection};

use crate::error::{self, Result, ServerError};
use crate::types::*;

const SESSIONS_LIST: &str = "sessions";

fn user_sessions_key(user_id: &UserId) -> String {
    format!("sessions:{}", **user_id)
}

pub fn get_user_id(c: &mut Connection, auth: &Auth) -> Result<UserId> {
    let id = c.hget(SESSIONS_LIST, auth.0)?;
    Ok(UserId(id))
}

pub fn store_session(c: &mut Connection, auth: &str, user_id: &UserId) -> Result<()> {
    if c.hexists(SESSIONS_LIST, auth)? {
        Err(ServerError::new(
            error::INTERNAL_ERROR,
            "Auth already exists",
        ))
    } else {
        let user_session_key = user_sessions_key(user_id);
        transaction(c, &[SESSIONS_LIST, &user_session_key], |c, pipe| {
            pipe.hset(SESSIONS_LIST, auth, user_id.to_string())
                .ignore()
                .sadd(&user_session_key, auth)
                .query(c)
        })?;

        Ok(())
    }
}

pub fn validate_session(c: &mut Connection, auth: &Auth) -> Result<()> {
    if c.hexists(SESSIONS_LIST, auth.0)? {
        let user_id = get_user_id(c, auth)?;
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

fn delete_session_with_connection(c: &mut Connection, auth: &Auth, user_id: &UserId) -> Result<()> {
    let user_session_key = user_sessions_key(user_id);
    Ok(transaction(
        c,
        &[SESSIONS_LIST, &user_session_key],
        |c, pipe| {
            pipe.hdel(SESSIONS_LIST, auth.0)
                .ignore()
                .srem(&user_session_key, auth.0)
                .query(c)
        },
    )?)
}

pub fn delete_session(c: &mut Connection, auth: &Auth, wanted_user_id: &UserId) -> Result<()> {
    let user_id = get_user_id(c, auth)?;
    if user_id == *wanted_user_id {
        delete_session_with_connection(c, &auth, &user_id)
    } else {
        Err(ServerError::new(
            error::UNAUTHORISED,
            "x-auth-token does not belong to this user",
        ))
    }
}

pub fn delete_all_user_sessions(c: &mut Connection, auth: &Auth) -> Result<()> {
    let user_id = UserId(c.hget(SESSIONS_LIST, auth.0)?);
    let all_user_sessions: Vec<String> = c.smembers(&user_sessions_key(&user_id))?;
    all_user_sessions
        .iter()
        .map(|a| delete_session_with_connection(c, &Auth(a), &user_id))
        .collect()
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::db::{salts::tests::*, tests::*};
    use fake_redis::FakeCient as Client;

    pub const AUTH: Auth = Auth("tokenauth");
    pub const AUTH2: Auth = Auth("anothertokenauth");

    pub fn store_session_for_test(c: &mut Connection, auth: &Auth) {
        let user_id = UserId(HASH_1.to_owned());
        assert_eq!(Ok(()), store_session(c, auth, &user_id));
        assert_eq!(Ok(true), c.hexists(SESSIONS_LIST, auth.0));
        assert_eq!(Ok(true), c.sismember(&user_sessions_key(&user_id), auth.0));
        assert_eq!(
            Err(ServerError::new(
                error::INTERNAL_ERROR,
                "Auth already exists",
            )),
            store_session(c, &AUTH, &UserId(HASH_1.to_owned()))
        );
    }

    #[test]
    fn validate_session_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        store_session_for_test(&mut c, &AUTH);
        assert_eq!(Ok(()), validate_session(&mut c, &AUTH));
        assert_eq!(
            Err(ServerError::new(error::UNAUTHORISED, "Not logged in")),
            validate_session(&mut c, &Auth("notpresentauth"))
        );
        // tamper user sessions list
        let _: i32 = c
            .srem(&user_sessions_key(&UserId(HASH_1.to_owned())), AUTH.0)
            .unwrap();
        assert_eq!(
            Err(ServerError::new(
                error::UNAUTHORISED,
                "x-auth-token does not belong to this user",
            )),
            validate_session(&mut c, &AUTH)
        );
    }

    #[test]
    fn get_user_id_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        store_session_for_test(&mut c, &AUTH);
        assert_eq!(Ok(UserId(HASH_1.to_owned())), get_user_id(&mut c, &AUTH));
        store_session_for_test(&mut c, &AUTH2);
        assert_eq!(Ok(UserId(HASH_1.to_owned())), get_user_id(&mut c, &AUTH2));
    }

    #[test]
    fn delete_session_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        store_session_for_test(&mut c, &AUTH);
        assert_eq!(
            Ok(()),
            delete_session(&mut c, &AUTH, &UserId(HASH_1.to_owned()))
        );
        assert_eq!(Ok(false), c.exists(SESSIONS_LIST));
        assert_eq!(
            Ok(false),
            c.exists(&user_sessions_key(&UserId(HASH_1.to_owned())))
        );
    }

    #[test]
    fn delete_all_user_sessions_test() {
        let client = Client::open(get_db_addr().as_str()).unwrap();
        let mut c = client.get_connection().unwrap();
        store_session_for_test(&mut c, &AUTH);
        let u = UserId(HASH_1.to_owned());
        assert_eq!(Ok(()), store_session(&mut c, "AUTH2", &u));
        assert_eq!(Ok(()), delete_all_user_sessions(&mut c, &AUTH));
        assert_eq!(Ok(false), c.exists(SESSIONS_LIST));
        assert_eq!(Ok(false), c.exists(&user_sessions_key(&u)));
    }
}
