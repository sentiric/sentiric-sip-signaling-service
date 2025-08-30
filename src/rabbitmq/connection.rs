// File: sentiric-sip-signaling-service/src/rabbitmq/connection.rs

use lapin::{options::*, types::FieldTable, Channel as LapinChannel, Connection, ConnectionProperties, ExchangeKind};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub const RABBITMQ_EXCHANGE_NAME: &str = "sentiric_events";

pub async fn connect_with_retry(url: &str) -> Arc<LapinChannel> {
    let max_retries = 10;
    for i in 0..max_retries {
        if let Ok(conn) = Connection::connect(url, ConnectionProperties::default()).await {
            if let Ok(channel) = conn.create_channel().await {
                info!("RabbitMQ bağlantısı başarıyla kuruldu.");
                return Arc::new(channel);
            }
        }
        warn!(
            attempt = i + 1,
            max_attempts = max_retries,
            "RabbitMQ'ya bağlanılamadı. 5sn sonra tekrar denenecek..."
        );
        sleep(Duration::from_secs(5)).await;
    }
    panic!("Maksimum deneme sayısına ulaşıldı, RabbitMQ'ya bağlanılamadı.");
}

pub async fn declare_exchange(channel: &LapinChannel) -> Result<(), lapin::Error> {
    channel
        .exchange_declare(
            RABBITMQ_EXCHANGE_NAME,
            ExchangeKind::Topic, // <<< DEĞİŞİKLİK BURADA
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
}