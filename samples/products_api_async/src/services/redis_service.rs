use mediator::futures::StreamExt;
use redis::aio::Connection;
use redis::{AsyncCommands, Client, RedisError, RedisResult};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct RedisService<V> {
    client: Client,
    base_key: String,
    _marker: PhantomData<V>,
}

#[allow(unused)]
impl<V> RedisService<V>
where
    V: Serialize + DeserializeOwned,
{
    pub fn new<S: Into<String>>(client: Client, key: S) -> Self {
        let base_key = key.into();
        Self {
            client,
            base_key,
            _marker: PhantomData,
        }
    }

    pub async fn set<S: AsRef<str>>(&mut self, key: S, value: V) -> Result<(), redis::RedisError> {
        let json = serde_json::to_string(&value).map_err(|e| {
            RedisError::from((
                redis::ErrorKind::TypeError,
                "Failed to serialize value",
                e.to_string(),
            ))
        })?;

        let mut con = self.connection().await?;
        let key = format!("{}:{}", self.base_key, key.as_ref());
        con.set(key, json).await
    }

    pub async fn delete<S: AsRef<str>>(&mut self, key: S) -> Result<Option<V>, redis::RedisError> {
        let mut con = self.connection().await?;
        let key = format!("{}:{}", self.base_key, key.as_ref());
        let json: Option<String> = con.get(key.clone()).await?;

        match json {
            Some(json) => {
                let value: V = serde_json::from_str(&json).map_err(|e| {
                    RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Failed to deserialize value",
                        e.to_string(),
                    ))
                })?;
                con.del(key).await?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub async fn get<S: AsRef<str>>(&mut self, key: S) -> Result<Option<V>, redis::RedisError> {
        let mut con = self.connection().await?;
        let key = format!("{}:{}", self.base_key, key.as_ref());
        let json: Option<String> = con.get(key).await?;

        match json {
            Some(json) => {
                let value: V = serde_json::from_str(&json).map_err(|e| {
                    RedisError::from((
                        redis::ErrorKind::TypeError,
                        "Failed to deserialize value",
                        e.to_string(),
                    ))
                })?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    pub async fn get_all(&mut self) -> Result<Vec<V>, redis::RedisError> {
        let pattern = format!("{}:*", self.base_key);
        let mut con = self.connection().await?;

        let mut iter = con
            .scan_match(pattern)
            .await?
            .collect::<Vec<String>>()
            .await;

        let mut values = Vec::new();

        for key in iter {
            let json: String = con.get(key).await?;
            let value: V = serde_json::from_str(&json).map_err(|e| {
                RedisError::from((
                    redis::ErrorKind::TypeError,
                    "Failed to deserialize value",
                    e.to_string(),
                ))
            })?;

            values.push(value);
        }

        Ok(values)
    }

    // Get a connection to the Redis server
    async fn connection(&self) -> RedisResult<Connection> {
        self.client.get_async_connection().await
    }
}
