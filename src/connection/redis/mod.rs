pub use self::connection::RedisConnection;

use resp::{Decoder, Value};
use std::ascii::AsciiExt;

mod connection;

pub trait RedisProxy {
    fn on_command(&mut self, command: Value) -> Value;
    fn on_response(&mut self, response: Value) -> Value;
}

pub struct NoopProxy;

impl RedisProxy for NoopProxy {
    fn on_command(&mut self, command: Value) -> Value {
        command
    }

    fn on_response(&mut self, response: Value) -> Value {
        response
    }
}

pub struct ComposedProxy<A: RedisProxy, B: RedisProxy> {
    proxy_a: A,
    proxy_b: B,
}

impl<A: RedisProxy, B: RedisProxy> ComposedProxy<A, B> {
    pub fn new(proxy_a: A, proxy_b: B) -> Self {
        ComposedProxy {
            proxy_a: proxy_a,
            proxy_b: proxy_b,
        }
    }
}

impl<A: RedisProxy, B: RedisProxy> RedisProxy for ComposedProxy<A, B> {
    fn on_command(&mut self, command: Value) -> Value {
        self.proxy_a.on_command(
            self.proxy_b.on_command(command)
        )
    }

    fn on_response(&mut self, response: Value) -> Value {
        self.proxy_b.on_response(
            self.proxy_a.on_response(response)
        )
    }
}

pub struct PrefixProxy;

impl RedisProxy for PrefixProxy {
    fn on_command(&mut self, command: Value) -> Value {
        match command {
            Value::Array(ref input) => {
                if input.len() == 0 {
                    command.clone()
                } else {
                    if let Value::Bulk(ref cmd) = input[0] {
                        match &*cmd.to_ascii_uppercase() {
                            "APPEND" | "BITCOUNT" | "BITPOS" |
                            "DECR" | "DECRBY" | "DUMP" |
                            "EXPIRE" | "EXPIREAT" | "GEOADD" |
                            "GEOHASH" | "GEOPOS" | "GEODIST" |
                            "GEORADIUS" | "GEORADIUSBYMEMBER" | "GET" |
                            "GETBIT" | "GETRANGE" | "GETSET" |
                            "HDEL" | "HEXISTS" | "HGET" |
                            "HGETALL" | "HINCRBY" | "HINCRBYFLOAT" |
                            "HKEYS" | "HLEN" | "HMGET" |
                            "HMSET" | "HSET" | "HSETNX" |
                            "HSTRLEN" | "HVALS" | "INCR" |
                            "INCRBY" | "INCRBYFLOAT" | "LINDEX" |
                            "LINSERT" | "LLEN" | "LPOP" |
                            "LPUSH" | "LPUSHX" | "LRANGE" |
                            "LREM" | "LSET" | "LTRIM" |
                            "MOVE" | "PERSIST" | "PEXPIRE" |
                            "PEXPIREAT" | "PFADD" | "PSETEX" |
                            "PTTL" | "RESTORE" | "RPOP" |
                            "RPUSH" | "RPUSHX" | "SADD" |
                            "SCARD" | "SET" | "SETBIT" |
                            "SETEX" | "SETNX" | "SETRANGE" |
                            "SISMEMBER" | "SMEMBERS" | "SORT" |
                            "STRLEN" | "TTL" | "TYPE" |
                            "ZADD" | "ZCARD" | "ZCOUNT" |
                            "ZINCRBY" | "ZLEXCOUNT" | "ZRANGE" |
                            "ZRANGEBYLEX" | "ZREVRANGEBYLEX" | "ZRANGEBYSCORE" |
                            "ZRANK" | "ZREM" | "ZREMRANGEBYLEX" |
                            "ZREMRANGEBYRANK" | "ZREMRANGEBYSCORE" | "ZREVRANGE" |
                            "ZREVRANGEBYSCORE" | "ZREVRANK" | "ZSCORE" |
                            "SSCAN" | "HSCAN" | "ZSCAN" => {
                                let mut out: Vec<Value> = Vec::new();
                                out.push(Value::Bulk(cmd.clone()));
                                if let Value::Bulk(ref key) = input[1] {
                                    out.push(Value::Bulk(format!("{}:{}", "prefix", key.clone())));

                                    if input.len() > 2 {
                                        for i in 2..input.len() {
                                            out.push(input[i].clone());
                                        }
                                    }
                                }

                                Value::Array(out)
                            },
                            _ => {
                                command.clone()
                            }
                        }
                    } else {
                        command.clone()
                    }
                }
            },
            _ => {
                command
            }
        }
    }

    fn on_response(&mut self, response: Value) -> Value {
        response
    }
}

pub struct LogProxy;

impl RedisProxy for LogProxy {
    fn on_command(&mut self, command: Value) -> Value {
        warn!("Received command: {:?}", command);

        command
    }

    fn on_response(&mut self, response: Value) -> Value {
        warn!("Response: {}", response.to_beautify_string());

        response
    }
}
