use std::collections::HashMap;

use crate::consts::*;
use crate::db::{sessions, user};
use crate::error::{self, Result, ServerError};
use crate::helpers::*;
use crate::token::Token;

pub struct AuthInfo {
  pub username: String,
  pub password: String,
}

impl Drop for AuthInfo {
  fn drop(&mut self) {
    self.password.replace_range(..self.password.len(), "0");
  }
}

pub fn login(auth_struct: HashMap<String, String>) -> Result<Token> {
  let auth_info = AuthInfo {
    username: extract_value(&auth_struct, K_USERNAME, "Missing username")?,
    password: extract_value(&auth_struct, K_PASSWORD, "Missing password")?,
  };
  let (token, user_id) = user::verify_password(&auth_info)?;
  sessions::store_session(&token.session_token, user_id)?;
  Ok(token)
}

pub fn logout(auth: String) -> Result<()> {
  sessions::del_session(&auth)?;
  Ok(())
}
