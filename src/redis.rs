// File: sentiric-sip-signaling-service/src/redis.rs

// DÜZELTME 1: `Client` tipini bu modülün dışına `pub` olarak açıyoruz.
pub use redis::Client; 
// DÜZELTME 2: `AsyncCommands` trait'ini de public yapıyoruz ki set_registration içinde kullanabilelim.
pub use redis::{AsyncCommands, RedisResult};

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};


pub async fn connect_with_retry(url: &str) -> Client {
    let max_retries = 10;
    for i in 0..max_retries {
        if let Ok(client) = redis::Client::open(url) {
            if client.get_multiplexed_async_connection().await.is_ok() {
                info!("Redis bağlantısı başarıyla kuruldu.");
                return client;
            }
        }
        warn!(
            attempt = i + 1,
            max_attempts = max_retries,
            "Redis'e bağlanılamadı. 5sn sonra tekrar denenecek..."
        );
        sleep(Duration::from_secs(5)).await;
    }
    panic!("Maksimum deneme sayısına ulaşıldı, Redis'e bağlanılamadı.");
}

pub async fn set_registration(
    client: &Client,
    aor: &str, 
    contact_uri: &str,
    expires: u64,
) -> RedisResult<()> {
    let mut conn = client.get_multiplexed_async_connection().await?;
    conn.set_ex(aor, contact_uri, expires).await
}