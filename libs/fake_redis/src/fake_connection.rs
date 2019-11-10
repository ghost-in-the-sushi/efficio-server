use std::clone::Clone;
use std::collections::HashMap;
use std::sync::Mutex;

use derive_new::new;
use lazy_static::lazy_static;
use redis::{self, from_redis_value, FromRedisValue, RedisResult, ToRedisArgs, Value};

lazy_static! {
    static ref POOL: Mutex<HashMap<i64, Storages>> = Mutex::new(HashMap::new());
}

#[derive(new, Debug)]
struct Storages {
    #[new(default)]
    pub k: HashMap<String, Value>,
    #[new(default)]
    pub h: HashMap<String, HashMap<String, Value>>,
    #[new(default)]
    pub s: HashMap<String, Vec<Value>>,
}

#[derive(new)]
pub struct FakeConnection {
    pub db: i64,
}

impl FakeConnection {
    pub fn get<RV: FromRedisValue>(&mut self, key: &str) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        from_redis_value(&db.k.get(key).map_or_else(|| Value::Nil, Clone::clone))
    }

    pub fn del<RV: FromRedisValue>(&mut self, key: &str) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        from_redis_value(&db.k.remove(&key.to_owned()).map_or_else(
            || {
                db.h.remove(&key.to_owned()).map_or_else(
                    || {
                        db.s.remove(&key.to_owned())
                            .map_or_else(|| Value::Int(0), |_| Value::Int(1))
                    },
                    |_| Value::Int(1),
                )
            },
            |_| Value::Int(1),
        ))
    }

    pub fn set<V: ToRedisArgs, RV: FromRedisValue>(
        &mut self,
        key: &str,
        value: V,
    ) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let v = value.to_redis_args();
        db.k.insert(key.to_owned(), Value::Data(v[0].clone()));
        from_redis_value(&Value::Okay)
    }

    pub fn exists<RV: FromRedisValue>(&mut self, key: &str) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        // dbg!(&db);
        from_redis_value(&Value::Int(
            (db.k.contains_key(&key.to_owned())
                || db.h.contains_key(&key.to_owned())
                || db.s.contains_key(&key.to_owned())) as i64,
        ))
    }

    pub fn incr<V: Into<i64> + Copy, RV: FromRedisValue>(
        &mut self,
        key: &str,
        delta: V,
    ) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        from_redis_value(
            &db.k
                .entry(key.to_owned())
                .and_modify(|e| {
                    if let Value::Int(ref mut e) = e {
                        *e += delta.into()
                    }
                })
                .or_insert_with(|| Value::Int(delta.into())),
        )
    }

    pub fn hset<V: ToRedisArgs, RV: FromRedisValue>(
        &mut self,
        key: &str,
        field: &str,
        value: V,
    ) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let mut is_new = false;
        let v = value.to_redis_args();
        db.h.entry(key.to_owned())
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
        &mut self,
        key: &str,
        items: &[(&str, V)],
    ) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        for item in items {
            let field = item.0;
            let value = &item.1;
            let v = value.to_redis_args();
            db.h.entry(key.to_owned())
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

    pub fn hget<RV: FromRedisValue>(&mut self, key: &str, field: &str) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        from_redis_value(&db.h.get(key).map_or_else(
            || Value::Nil,
            |h| h.get(field).map_or_else(|| Value::Nil, Clone::clone),
        ))
    }

    pub fn hdel<RV: FromRedisValue>(&mut self, key: &str, field: &str) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let mut need_delete_key = false;
        let r = db.h.get_mut(key).map_or_else(
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
            db.h.remove(key);
        }
        r
    }

    pub fn hexists<RV: FromRedisValue>(&mut self, key: &str, field: &str) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        db.h.get(key).map_or_else(
            || from_redis_value(&Value::Int(false as i64)),
            |h| from_redis_value(&Value::Int(h.contains_key(field) as i64)),
        )
    }

    // pub fn sadd<M: ToRedisArgs, RV: FromRedisValue>(
    //     &mut self,
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
        &mut self,
        key: &str,
        member: M,
    ) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let mut need_delete_key = false;
        let v = member.to_redis_args();
        let r = db.s.get_mut(key).map_or_else(
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
            db.s.remove(key);
        }
        r
    }

    pub fn smembers<RV: FromRedisValue>(&mut self, key: &str) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        from_redis_value(
            &db.s
                .get(key)
                .map_or_else(|| Value::Nil, |s| Value::Bulk(s.to_vec())),
        )
    }

    pub fn sismember<M: ToRedisArgs, RV: FromRedisValue>(
        &mut self,
        key: &str,
        member: M,
    ) -> RedisResult<RV> {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let v = member.to_redis_args();
        db.s.get(key).map_or_else(
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
}

#[derive(new)]
pub struct FakePipeline {
    db: i64,
}

impl FakePipeline {
    pub fn del(&mut self, key: &str) -> &mut Self {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        db.k.remove(&key.to_owned());
        db.h.remove(&key.to_owned());
        db.s.remove(&key.to_owned());
        self
    }

    pub fn sadd<M: ToRedisArgs>(&mut self, key: &str, member: M) -> &mut Self {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let v = member.to_redis_args();
        db.s.entry(key.to_owned())
            .and_modify(|h| h.push(Value::Data(v[0].clone())))
            .or_insert_with(|| vec![Value::Data(v[0].clone())]);
        self
    }

    pub fn srem<M: ToRedisArgs>(&mut self, key: &str, member: M) -> &mut Self {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let mut need_delete_key = false;
        let v = member.to_redis_args();
        db.s.get_mut(key).map(|s| {
            s.iter()
                .position(|i| *i == Value::Data(v[0].clone()))
                .map(|i| {
                    s.remove(i);
                    need_delete_key = s.is_empty();
                })
        });
        if need_delete_key {
            db.s.remove(key);
        }
        self
    }

    pub fn hset<V: ToRedisArgs>(&mut self, key: &str, field: &str, value: V) -> &mut Self {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let v = value.to_redis_args();
        db.h.entry(key.to_owned())
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
            });
        self
    }

    pub fn hdel(&mut self, key: &str, field: &str) -> &mut Self {
        let mut pool = POOL.lock().unwrap();
        let db = pool.entry(self.db).or_insert_with(Storages::new);
        let mut need_delete_key = false;
        if let Some(h) = db.h.get_mut(key) {
            h.remove(field);
            need_delete_key = h.is_empty();
        };
        if need_delete_key {
            db.h.remove(key);
        }
        self
    }

    pub fn ignore(&mut self) -> &mut Self {
        self
    }

    pub fn query<T: FromRedisValue>(&mut self, _con: &FakeConnection) -> RedisResult<T> {
        from_redis_value(&Value::Nil)
    }

    pub fn atomic(&mut self) -> &mut Self {
        self
    }
}
