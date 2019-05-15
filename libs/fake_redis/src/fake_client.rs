use redis::{self, IntoConnectionInfo, RedisResult};

use crate::fake_connection::FakeConnection;

pub struct FakeCient {}

impl FakeCient {
    pub fn open<T: IntoConnectionInfo>(_params: T) -> RedisResult<FakeCient> {
        Ok(FakeCient {})
    }

    pub fn get_connection(&self) -> RedisResult<FakeConnection> {
        Ok(FakeConnection::new())
    }
}
