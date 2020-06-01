use redis::{self, ConnectionInfo, IntoConnectionInfo, RedisResult};

use crate::fake_connection::FakeConnection;

pub struct FakeCient {
    info: ConnectionInfo,
}

impl FakeCient {
    pub fn open<T: IntoConnectionInfo>(params: T) -> RedisResult<FakeCient> {
        Ok(FakeCient {
            info: params.into_connection_info()?,
        })
    }

    pub fn get_connection(&self) -> RedisResult<FakeConnection> {
        Ok(FakeConnection::new(self.info.db))
    }
}
