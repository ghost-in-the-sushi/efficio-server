use crate::db::{sessions, users};
use crate::error::Result;
use crate::types::*;

pub fn login(auth_info: &AuthInfo) -> Result<Token> {
    let (token, user_id) = users::login(&auth_info)?;
    sessions::store_session(&token.session_token, &user_id)?;
    Ok(token)
}

pub fn logout(auth: String) -> Result<()> {
    let auth = Auth(&auth);
    sessions::validate_session(&auth)?;
    sessions::delete_session(&auth)?;
    Ok(())
}
