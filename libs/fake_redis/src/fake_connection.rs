use std::collections::HashMap;
use std::sync::Mutex;

use derive_new::new;
use lazy_static::lazy_static;
use redis::{self, from_redis_value, FromRedisValue, RedisResult, ToRedisArgs, Value};

lazy_static! {
    static ref STORE_KV: Mutex<HashMap<String, Value>> = Mutex::new(HashMap::new());
    static ref STORE_H: Mutex<HashMap<String, HashMap<String, Value>>> = Mutex::new(HashMap::new());
    static ref STORE_S: Mutex<HashMap<String, Vec<Value>>> = Mutex::new(HashMap::new());
}

#[derive(new)]
pub struct FakeConnection {}

impl FakeConnection {
    pub fn get<RV: FromRedisValue>(&self, key: &str) -> RedisResult<RV> {
        let storage = STORE_KV.lock().unwrap();
        from_redis_value(&storage.get(key).map_or_else(|| Value::Nil, |e| e.clone()))
    }

    pub fn del<RV: FromRedisValue>(&self, key: &str) -> RedisResult<RV> {
        let mut storage = STORE_KV.lock().unwrap();
        from_redis_value(&storage.remove(&key.to_owned()).map_or_else(
            || {
                let mut store_h = STORE_H.lock().unwrap();
                store_h.remove(&key.to_owned()).map_or_else(
                    || {
                        let mut store_s = STORE_S.lock().unwrap();
                        store_s
                            .remove(&key.to_owned())
                            .map_or_else(|| Value::Int(0), |_| Value::Int(1))
                    },
                    |_| Value::Int(1),
                )
            },
            |_| Value::Int(1),
        ))
    }

    pub fn exists<RV: FromRedisValue>(&self, key: &str) -> RedisResult<RV> {
        // println!("exists: {}", &key);
        let storage = STORE_KV.lock().unwrap();
        let store_h = STORE_H.lock().unwrap();
        let store_s = STORE_S.lock().unwrap();
        // dbg!(&storage);
        // dbg!(&store_h);
        // dbg!(&store_s);
        from_redis_value(&Value::Int(
            (storage.contains_key(&key.to_owned())
                || store_h.contains_key(&key.to_owned())
                || store_s.contains_key(&key.to_owned())) as i64,
        ))
    }

    pub fn incr<V: Into<i64> + Copy, RV: FromRedisValue>(
        &self,
        key: &str,
        delta: V,
    ) -> RedisResult<RV> {
        // dbg!(&key);
        let mut storage = STORE_KV.lock().unwrap();
        from_redis_value(
            &storage
                .entry(key.to_owned())
                .and_modify(|e| match e {
                    Value::Int(ref mut e) => *e += delta.into(),
                    _ => (),
                })
                .or_insert_with(|| Value::Int(0i64 + delta.into())),
        )
    }

    pub fn hset<V: ToRedisArgs, RV: FromRedisValue>(
        &self,
        key: &str,
        field: &str,
        value: V,
    ) -> RedisResult<RV> {
        // dbg!(&key);
        // dbg!(&field);
        let mut storage = STORE_H.lock().unwrap();
        let mut is_new = false;
        let v = value.to_redis_args();
        storage
            .entry(key.to_owned())
            .and_modify(|h| {
                h.entry(field.to_owned())
                    .and_modify(|e| {
                        *e = Value::Data(v[0].clone());
                    })
                    .or_insert_with(|| {
                        is_new = true;
                        Value::Data(v[0].clone())
                    });
            })
            .or_insert_with(|| {
                let mut h = HashMap::new();
                h.insert(field.to_owned(), Value::Data(v[0].clone()));
                h
            });
        from_redis_value(&Value::Int(is_new as i64))
    }

    pub fn hset_multiple<V: ToRedisArgs, RV: FromRedisValue>(
        &self,
        key: &str,
        items: &[(&str, V)],
    ) -> RedisResult<RV> {
        let mut storage = STORE_H.lock().unwrap();
        for item in items {
            let field = item.0;
            let value = &item.1;
            let v = value.to_redis_args();
            storage
                .entry(key.to_owned())
                .and_modify(|e| {
                    e.entry(field.to_owned())
                        .and_modify(|e| {
                            *e = Value::Data(v[0].clone());
                        })
                        .or_insert_with(|| Value::Data(v[0].clone()));
                })
                .or_insert_with(|| {
                    let mut h = HashMap::new();
                    h.insert(field.to_owned(), Value::Data(v[0].clone()));
                    h
                });
        }
        from_redis_value(&Value::Okay)
    }

    pub fn hget<RV: FromRedisValue>(&self, key: &str, field: &str) -> RedisResult<RV> {
        let storage = STORE_H.lock().unwrap();
        from_redis_value(&storage.get(key).map_or_else(
            || Value::Nil,
            |h| h.get(field).map_or_else(|| Value::Nil, |e| e.clone()),
        ))
    }

    pub fn hdel<RV: FromRedisValue>(&self, key: &str, field: &str) -> RedisResult<RV> {
        let mut storage = STORE_H.lock().unwrap();
        let mut need_delete_key = false;
        let r = storage.get_mut(key).map_or_else(
            || from_redis_value(&Value::Int(0)),
            |h| {
                let r = h.remove(field).map_or_else(
                    || from_redis_value(&Value::Int(0)),
                    |_| from_redis_value(&Value::Int(1)),
                );
                need_delete_key = h.is_empty();
                r
            },
        );
        if need_delete_key {
            storage.remove(key);
        }
        r
    }

    pub fn hexists<RV: FromRedisValue>(&self, key: &str, field: &str) -> RedisResult<RV> {
        // println!("hexists: {}/{}", &key, &field);
        let store_h = STORE_H.lock().unwrap();
        // dbg!(&store_h);
        store_h.get(key).map_or_else(
            || from_redis_value(&Value::Int(false as i64)),
            |h| from_redis_value(&Value::Int(h.contains_key(field) as i64)),
        )
    }

    // pub fn sadd<M: ToRedisArgs, RV: FromRedisValue>(
    //     &self,
    //     key: &str,
    //     member: M,
    // ) -> RedisResult<RV> {
    //     dbg!(&key);
    //     let mut storage = STORE_S.lock().unwrap();
    //     let v = member.to_redis_args();
    //     let mut is_new = false;
    //     storage
    //         .entry(key.to_owned())
    //         .and_modify(|h| h.push(Value::Data(v[0].clone())))
    //         .or_insert_with(|| {
    //             is_new = true;
    //             vec![Value::Data(v[0].clone())]
    //         });
    //     from_redis_value(&Value::Int(is_new as i64))
    // }

    pub fn srem<M: ToRedisArgs, RV: FromRedisValue>(
        &self,
        key: &str,
        member: M,
    ) -> RedisResult<RV> {
        let mut storage = STORE_S.lock().unwrap();
        let mut need_delete_key = false;
        let v = member.to_redis_args();
        let r = storage.get_mut(key).map_or_else(
            || from_redis_value(&Value::Int(0)),
            |s| {
                s.iter()
                    .position(|i| *i == Value::Data(v[0].clone()))
                    .map_or_else(
                        || from_redis_value(&Value::Int(0)),
                        |i| {
                            s.remove(i);
                            need_delete_key = s.is_empty();
                            from_redis_value(&Value::Int(1))
                        },
                    )
            },
        );
        if need_delete_key {
            storage.remove(key);
        }
        r
    }

    pub fn smembers<RV: FromRedisValue>(&self, key: &str) -> RedisResult<RV> {
        let storage = STORE_S.lock().unwrap();
        from_redis_value(
            &storage
                .get(key)
                .map_or_else(|| Value::Nil, |s| Value::Bulk(s.to_vec())),
        )
    }

    pub fn sismember<M: ToRedisArgs, RV: FromRedisValue>(
        &self,
        key: &str,
        member: M,
    ) -> RedisResult<RV> {
        let storage = STORE_S.lock().unwrap();
        let v = member.to_redis_args();
        storage.get(key).map_or_else(
            || from_redis_value(&Value::Int(false as i64)),
            |s| {
                s.iter()
                    .position(|i| *i == Value::Data(v[0].clone()))
                    .map_or_else(
                        || from_redis_value(&Value::Int(false as i64)),
                        |_| from_redis_value(&Value::Int(true as i64)),
                    )
            },
        )
    }

    pub fn reset(&self) {
        let mut storage = STORE_KV.lock().unwrap();
        let mut store_h = STORE_H.lock().unwrap();
        let mut store_s = STORE_S.lock().unwrap();
        storage.clear();
        store_h.clear();
        store_s.clear();
    }
}

#[derive(new)]
pub struct FakePipeline {}

impl FakePipeline {
    pub fn del<'a>(&mut self, key: &str) -> &mut Self {
        let mut storage = STORE_KV.lock().unwrap();
        let mut store_h = STORE_H.lock().unwrap();
        let mut store_s = STORE_S.lock().unwrap();
        storage.remove(&key.to_owned());
        store_h.remove(&key.to_owned());
        store_s.remove(&key.to_owned());
        self
    }

    pub fn sadd<'a, M: ToRedisArgs>(&mut self, key: &str, member: M) -> &mut Self {
        let mut storage = STORE_S.lock().unwrap();
        let v = member.to_redis_args();
        storage
            .entry(key.to_owned())
            .and_modify(|h| h.push(Value::Data(v[0].clone())))
            .or_insert_with(|| vec![Value::Data(v[0].clone())]);
        self
    }

    pub fn srem<'a, M: ToRedisArgs>(&mut self, key: &str, member: M) -> &mut Self {
        let mut storage = STORE_S.lock().unwrap();
        let mut need_delete_key = false;
        let v = member.to_redis_args();
        storage.get_mut(key).map(|s| {
            s.iter()
                .position(|i| *i == Value::Data(v[0].clone()))
                .map(|i| {
                    s.remove(i);
                    need_delete_key = s.is_empty();
                })
        });
        if need_delete_key {
            storage.remove(key);
        }
        self
    }

    pub fn hset<'a, V: ToRedisArgs>(&mut self, key: &str, field: &str, value: V) -> &mut Self {
        let mut storage = STORE_H.lock().unwrap();
        let v = value.to_redis_args();
        storage
            .entry(key.to_owned())
            .and_modify(|h| {
                h.entry(field.to_owned())
                    .and_modify(|e| {
                        *e = Value::Data(v[0].clone());
                    })
                    .or_insert_with(|| Value::Data(v[0].clone()));
            })
            .or_insert_with(|| {
                let mut h = HashMap::new();
                h.insert(field.to_owned(), Value::Data(v[0].clone()));
                h
            });;
        self
    }

    pub fn hdel<'a>(&mut self, key: &str, field: &str) -> &mut Self {
        let mut storage = STORE_H.lock().unwrap();
        let mut need_delete_key = false;
        storage.get_mut(key).map(|h| {
            h.remove(field);
            need_delete_key = h.is_empty();
        });
        if need_delete_key {
            storage.remove(key);
        }
        self
    }

    pub fn ignore(&mut self) -> &mut Self {
        self
    }

    pub fn query<T: FromRedisValue>(&self, _con: &FakeConnection) -> RedisResult<T> {
        from_redis_value(&Value::Int(1))
    }

    pub fn atomic(&mut self) -> &mut Self {
        self
    }
}
