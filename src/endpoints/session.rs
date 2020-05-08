use crate::db::{sessions, users};
use crate::error::Result;
use crate::types::*;

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub async fn login(auth_info: &AuthInfo, c: &mut Connection) -> Result<ConnectionToken> {
    users::login(c, &auth_info)
}

pub async fn logout(auth: &str, user_id: &str, c: &mut Connection) -> Result<()> {
    let auth = Auth(&auth);
    sessions::validate_session(c, &auth)?;
    sessions::delete_session(c, &auth, &UserId(user_id.to_owned()))?;
    Ok(())
}
