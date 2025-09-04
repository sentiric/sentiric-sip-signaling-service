// File: sentiric-sip-signaling-service/src/rabbitmq/connection.rs
use lapin::{options::*, types::FieldTable, Channel as LapinChannel, Connection, ConnectionProperties, ExchangeKind};
use tracing::info;

pub const RABBITMQ_EXCHANGE_NAME: &str = "sentiric_events";

// Artık sadece bu fonksiyon kullanılıyor.
pub async fn try_connect(url: &str) -> Result<LapinChannel, lapin::Error> {
    let conn = Connection::connect(url, ConnectionProperties::default()).await?;
    let channel = conn.create_channel().await?;
    info!("RabbitMQ bağlantısı başarıyla kuruldu.");
    Ok(channel)
}

// DÜZELTME: Bu fonksiyon artık kullanılmadığı için kaldırıldı.
// pub async fn connect_with_retry(url: &str) -> LapinChannel { ... }

pub async fn declare_exchange(channel: &LapinChannel) -> Result<(), lapin::Error> {
    channel
        .exchange_declare(
            RABBITMQ_EXCHANGE_NAME,
            ExchangeKind::Topic,
            ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
}