#[cfg(not(test))]
use redis::{self, Connection};

#[cfg(test)]
use fake_redis::FakeConnection as Connection;

pub mod aisles;
pub mod ids;
pub mod products;
pub mod sessions;
pub mod stores;
pub mod users;

use crate::{error::*, types::*};

pub(crate) fn verify_permission(wanted_user_id: &UserId, user_id: &UserId) -> Result<()> {
    if wanted_user_id != user_id {
        Err(ServerError::new(
            PERMISSION_DENIED,
            "User does not have permission to edit this resource",
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn verify_permission_auth(
    c: &mut Connection,
    auth: &Auth,
    user_id: &UserId,
) -> Result<()> {
    let wanted_user_id = sessions::get_user_id(c, &auth)?;
    verify_permission(&wanted_user_id, &user_id)
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI64, Ordering};
    static DB_NUM: AtomicI64 = AtomicI64::new(0);
    pub fn get_db_addr() -> String {
        format!(
            "redis://127.0.0.1/{}",
            DB_NUM.fetch_add(1, Ordering::SeqCst)
        )
    }
}
