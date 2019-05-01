use serde::Deserialize;

use crate::db::{sessions, users};
use crate::error::Result;
use crate::token::Token;
use crate::types::*;

#[derive(Deserialize, Debug)]
pub struct AuthInfo {
    pub username: String,
    pub password: String,
}

impl Drop for AuthInfo {
    fn drop(&mut self) {
        self.password.replace_range(..self.password.len(), "0");
    }
}

pub fn login(auth_info: &AuthInfo) -> Result<Token> {
    // let auth_info = AuthInfo {
    //     username: extract_value(&auth_struct, K_USERNAME, "Missing username")?,
    //     password: extract_value(&auth_struct, K_PASSWORD, "Missing password")?,
    // };
    let (token, user_id) = users::verify_password(&auth_info)?;
    sessions::store_session(&token.session_token, &user_id)?;
    Ok(token)
}

pub fn logout(auth: String) -> Result<()> {
    let auth = Auth(&auth);
    sessions::validate_session(&auth)?;
    sessions::delete_session(&auth)?;
    Ok(())
}
