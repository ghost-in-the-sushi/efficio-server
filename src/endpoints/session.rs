use crate::db::{sessions, users};
use crate::error::Result;
use crate::types::*;

#[cfg(not(test))]
use redis::Client;

#[cfg(test)]
use fake_redis::FakeCient as Client;

pub fn login(auth_info: &AuthInfo, db_client: &Client) -> Result<Token> {
    let c = db_client.get_connection()?;
    let (token, user_id) = users::login(&c, &auth_info)?;
    sessions::store_session(&c, &token.session_token, &user_id)?;
    Ok(token)
}

pub fn logout(auth: String, db_client: &Client) -> Result<()> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    sessions::validate_session(&c, &auth)?;
    sessions::delete_session(&c, &auth)?;
    Ok(())
}
