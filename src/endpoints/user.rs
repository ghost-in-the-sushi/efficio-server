use lazy_static::lazy_static;
use regex::Regex;
use validator;
use zxcvbn;

#[cfg(not(test))]
use redis::Client;

#[cfg(test)]
use fake_redis::FakeCient as Client;

use crate::db;
use crate::endpoints::INVALID_PARAMS;
use crate::error::{Result, ServerError};
use crate::types::*;

const MIN_ENTROPY_SCORE: u8 = 2;

pub fn create_user(user: &User, db_client: &Client) -> Result<Token> {
    validate_email(&user.email)?;
    validate_password(&user)?;
    validate_username(&user.username)?;
    let c = db_client.get_connection()?;
    db::users::save_user(&c, &user)
}

pub fn delete_user(auth: String, db_client: &Client) -> Result<()> {
    let auth = Auth(&auth);
    let c = db_client.get_connection()?;
    db::sessions::validate_session(&c, &auth)?;
    db::users::delete_user(&c, &auth)
}

fn validate_email(mail: &str) -> Result<()> {
    if !validator::validate_email(mail) {
        Err(ServerError::new(INVALID_PARAMS, "Email field is invalid"))
    } else {
        Ok(())
    }
}

fn validate_password(user: &User) -> Result<()> {
    let entropy = zxcvbn::zxcvbn(&user.password, &[&user.username, &user.email])
        .or_else(|_| Err(ServerError::new(INVALID_PARAMS, "Empty password")))?;

    if entropy.score < MIN_ENTROPY_SCORE {
        Err(ServerError::new(
            INVALID_PARAMS,
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
        Err(ServerError::new(INVALID_PARAMS, "Invalid username"))
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
