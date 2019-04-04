use std::collections::HashMap;

use lazy_static::lazy_static;
use regex::Regex;
use validator;
use zxcvbn;

use crate::db;
use crate::error::{self, ServerError};
use crate::token::Token;

const K_USERNAME: &str = "username";
const K_EMAIL: &str = "email";
const K_PASSWORD: &str = "password";
const MIN_ENTROPY_SCORE: u8 = 2;

#[derive(Default, Debug)]
pub struct User {
  pub username: String,
  pub email: String,
  pub password: String,
}

impl Drop for User {
  fn drop(&mut self) {
    self.password.replace_range(..self.password.len(), "0");
    self.email.replace_range(..self.email.len(), "0");
  }
}

pub fn create_user(user_json: HashMap<String, String>) -> Result<Token, ServerError> {
  let user = User {
    username: user_json
      .get(K_USERNAME)
      .ok_or_else(|| ServerError {
        status: error::INVALID_PARAMS,
        msg: "Missing username".to_string(),
      })?
      .to_string(),
    email: user_json
      .get(K_EMAIL)
      .ok_or_else(|| ServerError {
        status: error::INVALID_PARAMS,
        msg: "Missing email".to_string(),
      })?
      .to_string(),
    password: user_json
      .get(K_PASSWORD)
      .ok_or_else(|| ServerError {
        status: error::INVALID_PARAMS,
        msg: "Missing password".to_string(),
      })?
      .to_string(),
  };

  validate_email(&user.email)?;
  validate_password(&user)?;
  validate_username(&user.username)?;

  db::store_user(&user)
}

fn validate_email(mail: &str) -> Result<(), ServerError> {
  if !validator::validate_email(mail) {
    Err(ServerError {
      status: error::INVALID_PARAMS,
      msg: "Email field is invalid".to_string(),
    })
  } else {
    Ok(())
  }
}

fn validate_password(user: &User) -> Result<(), ServerError> {
  let entropy = zxcvbn::zxcvbn(&user.password, &[&user.username, &user.email]).or_else(|_| {
    Err(ServerError {
      status: error::INVALID_PARAMS,
      msg: "Empty password".to_string(),
    })
  })?;

  if entropy.score < MIN_ENTROPY_SCORE {
    Err(ServerError {
      status: error::INVALID_PARAMS,
      msg: format!(
        "Password field is too weak (score: {}): {}",
        entropy.score.to_string(),
        entropy
          .feedback
          .unwrap_or_else(|| zxcvbn::feedback::Feedback {
            warning: Some("Unknown reason"),
            suggestions: vec![]
          })
          .warning
          .unwrap_or_else(|| "Unknown reason")
          .to_string()
      ),
    })
  } else {
    Ok(())
  }
}

fn validate_username(username: &str) -> Result<(), ServerError> {
  lazy_static! {
    static ref VALID_USERNAME_RE: Regex =
      Regex::new(r"^[a-zA-Z][0-9a-zA-Z_]*$").expect("Error in compiling username regex");
  }

  if !VALID_USERNAME_RE.is_match(username) {
    Err(ServerError {
      status: error::INVALID_PARAMS,
      msg: "Invalid username".to_string(),
    })
  } else {
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  // email validation and password validation relies on third party, test only username validation

  #[test]
  fn validate_username_test() {
    assert_eq!(true, validate_username("toto").is_ok());
    assert_eq!(true, validate_username("toto13").is_ok());
    assert_eq!(true, validate_username("toto_13").is_ok());
    assert_eq!(true, validate_username("to_to13").is_ok());
    assert_eq!(true, validate_username("t_ot_o13").is_ok());
    assert_eq!(true, validate_username("toto13_").is_ok());
    assert_eq!(true, validate_username("t").is_ok());
    assert_eq!(false, validate_username("_toto13").is_ok());
    assert_eq!(false, validate_username("42toto13").is_ok());
    assert_eq!(false, validate_username("_toto13").is_ok());
    assert_eq!(false, validate_username("_").is_ok());
    assert_eq!(false, validate_username("1").is_ok());
    assert_eq!(false, validate_username("42").is_ok());
  }
}
