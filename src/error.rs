use thiserror::Error;
use std::net::SocketAddr;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("Yapılandırma hatası: {0}")]
    Config(#[from] std::env::VarError),

    #[error("I/O hatası: {0}")]
    Io(#[from] std::io::Error),

    #[error("UDP soketi '{addr}' adresine bağlanamadı: {source}")]
    SocketBind { addr: SocketAddr, source: std::io::Error },

    #[error("SIP paketi ayrıştırılamadı: {0}")]
    SipParse(String),

    #[error("gRPC istemci hatası: {0}")]
    GrpcClient(#[from] tonic::transport::Error),

    #[error("gRPC servis hatası: {0}")]
    GrpcStatus(#[from] tonic::Status),

    #[error("RabbitMQ hatası: {0}")]
    RabbitMq(#[from] lapin::Error),

    #[error("Redis hatası: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("Serileştirme hatası (serde_json): {0}")]
    SerdeJson(#[from] serde_json::Error),

    #[error("Geçersiz başlık (Tonic): {0}")]
    InvalidHeader(#[from] tonic::metadata::errors::InvalidMetadataValue),
    
    // --- YENİ EKLENEN VARYANT ---
    #[error("Loglama filtresi hatası: {0}")]
    TracingFilter(#[from] tracing_subscriber::filter::ParseError),
    // --- DEĞİŞİKLİK SONU ---

    #[error("Beklenmedik bir hata oluştu: {0}")]
    Generic(String),
}

impl From<Box<dyn std::error::Error + Send + Sync>> for ServiceError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        ServiceError::Generic(err.to_string())
    }
}