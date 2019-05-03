use lazy_static::lazy_static;
use regex::Regex;
use serde::Deserialize;
use validator;
use zxcvbn;

use crate::consts::*;
use crate::db;
use crate::error::{self, Result, ServerError};
use crate::token::Token;
use crate::types::*;

#[derive(Default, Deserialize, Debug)]
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

pub fn create_user(user: &User) -> Result<Token> {
    validate_email(&user.email)?;
    validate_password(&user)?;
    validate_username(&user.username)?;

    db::save_user(&user)
}

pub fn delete_user(auth: String) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(&auth)?;
    db::users::delete_user(&auth)
}

fn validate_email(mail: &str) -> Result<()> {
    if !validator::validate_email(mail) {
        Err(ServerError::new(
            error::INVALID_PARAMS,
            "Email field is invalid",
        ))
    } else {
        Ok(())
    }
}

fn validate_password(user: &User) -> Result<()> {
    let entropy = zxcvbn::zxcvbn(&user.password, &[&user.username, &user.email])
        .or_else(|_| Err(ServerError::new(error::INVALID_PARAMS, "Empty password")))?;

    if entropy.score < MIN_ENTROPY_SCORE {
        Err(ServerError::new(
            error::INVALID_PARAMS,
            &format!(
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
        ))
    } else {
        Ok(())
    }
}

fn validate_username(username: &str) -> Result<()> {
    lazy_static! {
        static ref VALID_USERNAME_RE: Regex =
            Regex::new(r"^[a-zA-Z][0-9a-zA-Z_]*$").expect("Error in compiling username regex");
    }

    if !VALID_USERNAME_RE.is_match(username) {
        Err(ServerError::new(error::INVALID_PARAMS, "Invalid username"))
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
