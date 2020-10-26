use lazy_static::lazy_static;
use regex::Regex;

#[cfg(not(test))]
use redis::Connection;

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

use crate::{
    db,
    endpoints::INVALID_PARAMS,
    error::{Result, ServerError},
    types::*,
};

const MIN_ENTROPY_SCORE: u8 = 2;

pub async fn create_user(user: &User, c: &mut Connection) -> Result<ConnectionToken> {
    validate_email(&user.email)?;
    validate_password(&user)?;
    validate_username(&user.username)?;
    db::users::save_user(c, &user)
}

pub async fn delete_user(auth: &str, user_id: &str, c: &mut Connection) -> Result<()> {
    let auth = Auth(&auth);
    db::sessions::validate_session(c, &auth)?;
    db::users::delete_user(c, &auth, &UserId(user_id.to_string()))
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
        .map_err(|_| ServerError::new(INVALID_PARAMS, "Empty password"))?;

    if entropy.score() < MIN_ENTROPY_SCORE {
        Err(ServerError::new(
            INVALID_PARAMS,
            &format!(
                "Password field is too weak (score: {}): {}",
                entropy.score().to_string(),
                entropy.feedback().as_ref().map_or_else(
                    || "Unknown reason".to_string(),
                    |v| v
                        .warning()
                        .map_or_else(|| "Unknown reason".to_string(), |v| format!("{}", v))
                )
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
