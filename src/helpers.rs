use std::collections::HashMap;

use crate::error::{self, ServerError};

pub fn extract_value(
  h: &HashMap<String, String>,
  key: &str,
  err_msg: &str,
) -> Result<String, ServerError> {
  Ok(
    h.get(key)
      .ok_or_else(|| ServerError {
        status: error::INVALID_PARAMS,
        msg: err_msg.to_string(),
      })?
      .to_string(),
  )
}
