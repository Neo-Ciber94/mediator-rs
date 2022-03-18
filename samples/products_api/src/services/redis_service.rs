use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use redis::{Client, Commands, RedisError};
use serde::Serialize;
use serde::de::DeserializeOwned;
pub type SharedRedisService<V> = Arc<Mutex<RedisService<V>>>;

#[derive(Debug, Clone)]
pub struct RedisService<V> {
    client: Client,
    base_key: String,
    _marker: PhantomData<V>
}

#[allow(unused)]
impl<V> RedisService<V> where V: Serialize + DeserializeOwned {
    pub fn new<S: Into<String>>(client: Client, key: S) -> Self {
        let base_key = key.into();
        Self { client, base_key, _marker: PhantomData }
    }

    pub fn set<S: AsRef<str>>(&mut self, key: S, value: V) -> Result<(), redis::RedisError> {
        let json = serde_json::to_string(&value).map_err(|e| {
            RedisError::from((redis::ErrorKind::TypeError, "Failed to serialize value", e.to_string()))
        })?;

        let key = format!("{}:{}", self.base_key, key.as_ref());
        self.client.set(key, json)
    }

    pub fn delete<S: AsRef<str>>(&mut self, key: S) -> Result<Option<V>, redis::RedisError> {
        let key = format!("{}:{}", self.base_key, key.as_ref());
        let json: Option<String> = self.client.get(key.clone())?;

        match json {
            Some(json) => {
                let value: V = serde_json::from_str(&json).map_err(|e| {
                    RedisError::from((redis::ErrorKind::TypeError, "Failed to deserialize value", e.to_string()))
                })?;
                self.client.del(key)?;
                Ok(Some(value))
            },
            None => Ok(None)
        }
    }

    pub fn get<S: AsRef<str>>(&mut self, key: S) -> Result<Option<V>, redis::RedisError> {
        let key = format!("{}:{}", self.base_key, key.as_ref());
        let json : Option<String> = self.client.get(key)?;

        match json {
            Some(json) => {
                let value: V = serde_json::from_str(&json).map_err(|e| {
                    RedisError::from((redis::ErrorKind::TypeError, "Failed to deserialize value", e.to_string()))
                })?;
                Ok(Some(value))
            },
            None => Ok(None)
        }
    }

    pub fn get_all(&mut self) -> Result<Vec<V>, redis::RedisError> {
        let mut values = Vec::new();

        let pattern = format!("{}:*", self.base_key);
        let iter: redis::Iter<String> = self.client.scan_match(pattern)?;
        let keys = iter.collect::<Vec<String>>();

        for key in keys {
            let json: String = self.client.get(key)?;
            let value: V = serde_json::from_str(&json).map_err(|e| {
                RedisError::from((redis::ErrorKind::TypeError, "Failed to deserialize value", e.to_string()))
            })?;
            values.push(value);
        }

        Ok(values)
    }
}
