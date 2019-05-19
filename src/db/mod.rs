#[cfg(not(test))]
use redis::{self, Client, Connection};

#[cfg(test)]
use fake_redis::{FakeCient as Client, FakeConnection as Connection};

pub mod aisles;
pub mod products;
pub mod sessions;
pub mod stores;
pub mod users;

use crate::error::*;
use crate::types::*;

pub fn get_client(addr: &str) -> Client {
    Client::open(addr).expect("Error while creating redis client.")
}

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

pub(crate) fn verify_permission_auth(c: &Connection, auth: &Auth, user_id: &UserId) -> Result<()> {
    let wanted_user_id = sessions::get_user_id(&c, &auth)?;
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

    // pub fn reset_db(c: &Connection) {
    //     c.reset();
    //     //let _: () = redis::cmd("FLUSHDB").query(&c).expect("error on flush");
    // }
}
