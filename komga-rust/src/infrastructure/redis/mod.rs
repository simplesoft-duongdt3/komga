use redis::{Client, AsyncCommands};
use std::env;

pub struct RedisCache {
    client: Client,
}

impl RedisCache {
    pub async fn new() -> Self {
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://localhost:6379".to_string());
        
        let client = Client::open(redis_url.as_str())
            .expect("Failed to connect to Redis");
        
        Self { client }
    }

    pub async fn get(&self, key: &str) -> Option<String> {
        let mut con = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(_) => return None,
        };
        
        match con.get(key).await {
            Ok(value) => Some(value),
            Err(_) => None,
        }
    }

    pub async fn set(&self, key: &str, value: &str, ttl_seconds: Option<u64>) -> bool {
        let mut con = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(_) => return false,
        };
        
        let result: redis::RedisResult<()> = if let Some(ttl) = ttl_seconds {
            con.set_ex(key, value, ttl).await
        } else {
            con.set(key, value).await
        };
        
        result.is_ok()
    }

    pub async fn delete(&self, key: &str) -> bool {
        let mut con = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(_) => return false,
        };
        let result: redis::RedisResult<usize> = con.del(key).await;
        result.is_ok()
    }

    pub async fn exists(&self, key: &str) -> bool {
        let mut con = match self.client.get_async_connection().await {
            Ok(c) => c,
            Err(_) => return false,
        };
        let result: redis::RedisResult<bool> = con.exists(key).await;
        result.unwrap_or(false)
    }
}

impl Clone for RedisCache {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
        }
    }
}

pub struct SessionStore {
    cache: RedisCache,
}

impl SessionStore {
    pub async fn new() -> Self {
        Self {
            cache: RedisCache::new().await,
        }
    }

    pub async fn get_session(&self, session_id: &str) -> Option<String> {
        self.cache.get(&format!("session:{}", session_id)).await
    }

    pub async fn set_session(&self, session_id: &str, data: &str, ttl_seconds: u64) -> bool {
        self.cache.set(&format!("session:{}", session_id), data, Some(ttl_seconds)).await
    }

    pub async fn delete_session(&self, session_id: &str) -> bool {
        self.cache.delete(&format!("session:{}", session_id)).await
    }
}

impl Clone for SessionStore {
    fn clone(&self) -> Self {
        Self {
            cache: self.cache.clone(),
        }
    }
}