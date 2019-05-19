use redis::{self, FromRedisValue, RedisResult};

mod fake_client;
mod fake_connection;

pub use fake_client::*;
pub use fake_connection::*;

pub fn transaction<T: FromRedisValue, F: FnMut(&mut FakePipeline) -> RedisResult<T>>(
    con: &FakeConnection,
    _keys: &[&str],
    mut func: F,
) -> RedisResult<T> {
    let mut pipe = FakePipeline::new(con.db);
    func(&mut pipe)
}
