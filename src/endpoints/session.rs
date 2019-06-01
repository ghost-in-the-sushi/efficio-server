use crate::db::{sessions, users};
use crate::error::Result;
use crate::types::*;

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub fn login(auth_info: &AuthInfo, c: &Connection) -> Result<Token> {
    let (token, user_id) = users::login(&c, &auth_info)?;
    sessions::store_session(&c, &token.session_token, &user_id)?;
    Ok(token)
}

pub fn logout(auth: String, c: &Connection) -> Result<()> {
    let auth = Auth(&auth);
    sessions::validate_session(&c, &auth)?;
    sessions::delete_session(&c, &auth)?;
    Ok(())
}
