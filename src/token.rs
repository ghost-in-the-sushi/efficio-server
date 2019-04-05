use hex_view::HexView;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

#[derive(Debug, Serialize, Deserialize)]
pub struct Token {
  pub session_token: String,
}

impl Deref for Token {
  type Target = String;

  fn deref(&self) -> &String {
    &self.session_token
  }
}

impl From<[u8; 32]> for Token {
  fn from(s: [u8; 32]) -> Self {
    Token {
      session_token: format!("{:x}", HexView::from(&s)),
    }
  }
}

impl From<String> for Token {
  fn from(s: String) -> Self {
    Token { session_token: s }
  }
}

impl From<&str> for Token {
  fn from(s: &str) -> Self {
    Token {
      session_token: s.to_string(),
    }
  }
}
