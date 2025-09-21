// sentiric-sip-signaling-service/src/grpc/client.rs
use crate::app_state::GrpcClients;
use crate::config::AppConfig;
use crate::error::ServiceError;
use sentiric_contracts::sentiric::{
    dialplan::v1::dialplan_service_client::DialplanServiceClient,
    media::v1::media_service_client::MediaServiceClient,
    user::v1::user_service_client::UserServiceClient,
};
use std::error::Error;
use std::time::Duration;
use tonic::transport::{Certificate, Channel, ClientTlsConfig, Identity};
use tracing::{info, instrument};

#[instrument(name = "grpc_client_setup", skip(config))]
pub async fn create_all_grpc_clients(config: &AppConfig) -> Result<GrpcClients, ServiceError> {
    let user_channel = create_secure_grpc_channel(
        &config.user_service_url,
        "user-service",
        config,
    )
    .await
    .map_err(|e| ServiceError::Generic(format!("user-service'e bağlanırken hata oluştu: {}", e)))?;

    let dialplan_channel = create_secure_grpc_channel(
        &config.dialplan_service_url,
        "dialplan-service",
        config,
    )
    .await
    .map_err(|e| {
        ServiceError::Generic(format!(
            "dialplan-service'e bağlanırken hata oluştu: {}",
            e
        ))
    })?;

    let media_channel = create_secure_grpc_channel(
        &config.media_service_url,
        "media-service",
        config,
    )
    .await
    .map_err(|e| ServiceError::Generic(format!("media-service'e bağlanırken hata oluştu: {}", e)))?;

    Ok(GrpcClients {
        user: UserServiceClient::new(user_channel),
        dialplan: DialplanServiceClient::new(dialplan_channel),
        media: MediaServiceClient::new(media_channel),
    })
}

// --- DEĞİŞTİRİLMİŞ VE NİHAİ FONKSİYON ---
async fn create_secure_grpc_channel(
    url: &str,
    server_name: &str,
    config: &AppConfig,
) -> Result<Channel, Box<dyn Error + Send + Sync>> {
    // 1. İstemcinin (bu servis) kendi kimliğini yükle
    let cert = tokio::fs::read(&config.cert_path).await?;
    let key = tokio::fs::read(&config.key_path).await?;
    let client_identity = Identity::from_pem(cert, key);

    // 2. Güvenilecek Kök Sertifika Otoritesini (CA) yükle
    let ca_cert = tokio::fs::read(&config.ca_path).await?;
    let server_ca_certificate = Certificate::from_pem(ca_cert);

    // 3. Tonic'in kendi TLS yapılandırmasını oluştur
    let tls_config = ClientTlsConfig::new()
        // Sunucu sertifikasındaki ismin bu olması gerektiğini belirt (doğrulama için kritik)
        .domain_name(server_name)
        // Güveneceğimiz CA'yı belirt
        .ca_certificate(server_ca_certificate)
        // Kendi kimliğimizi (sertifika + anahtar) belirt
        .identity(client_identity);

    // 4. Güvenli kanalı oluştur ve bağlan
    let endpoint = Channel::from_shared(format!("https://{}", url))?
        .connect_timeout(Duration::from_secs(5))
        .tls_config(tls_config)?;

    info!(url=%url, server_name=%server_name, "Güvenli gRPC kanalına bağlanılıyor...");
    let channel = endpoint.connect().await?;
    info!(url=%url, "gRPC bağlantısı başarılı.");
    Ok(channel)
}
// --- DEĞİŞİKLİK SONU ---