// ========== FILE: src/grpc/client.rs ==========
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
use tracing::instrument; // YENİ

#[instrument(name = "grpc_client_setup", skip(config))]
pub async fn create_all_grpc_clients(config: &AppConfig) -> Result<GrpcClients, ServiceError> {
    // DÜZELTME: Her bir bağlantı hatasını, hangi servise ait olduğunu belirtecek şekilde map'liyoruz.
    let user_channel = create_secure_grpc_channel(&config.user_service_url, "user-service")
        .await
        .map_err(|e| ServiceError::Generic(format!("user-service'e bağlanırken hata oluştu: {}", e)))?;
        
    let dialplan_channel = create_secure_grpc_channel(&config.dialplan_service_url, "dialplan-service")
        .await
        .map_err(|e| ServiceError::Generic(format!("dialplan-service'e bağlanırken hata oluştu: {}", e)))?;
        
    let media_channel = create_secure_grpc_channel(&config.media_service_url, "media-service")
        .await
        .map_err(|e| ServiceError::Generic(format!("media-service'e bağlanırken hata oluştu: {}", e)))?;

    Ok(GrpcClients {
        user: UserServiceClient::new(user_channel),
        dialplan: DialplanServiceClient::new(dialplan_channel),
        media: MediaServiceClient::new(media_channel),
    })
}

pub(crate) async fn create_secure_grpc_channel(
    url: &str,
    server_name: &str,
) -> Result<Channel, Box<dyn Error + Send + Sync>> {
    let cert_path = std::env::var("SIP_SIGNALING_SERVICE_CERT_PATH")?;
    let key_path = std::env::var("SIP_SIGNALING_SERVICE_KEY_PATH")?;
    let ca_path = std::env::var("GRPC_TLS_CA_PATH")?;
    let cert = tokio::fs::read(cert_path).await?;
    let key = tokio::fs::read(key_path).await?;
    let ca_cert = tokio::fs::read(ca_path).await?;
    let identity = Identity::from_pem(cert, key);
    let ca_cert = Certificate::from_pem(ca_cert);
    let tls_config = ClientTlsConfig::new()
        .domain_name(server_name)
        .ca_certificate(ca_cert)
        .identity(identity);
    let endpoint = Channel::from_shared(format!("https://{}", url))?
        .tls_config(tls_config)?
        .connect_timeout(Duration::from_secs(5))
        .keep_alive_while_idle(true)
        .timeout(Duration::from_secs(10));
    let channel = endpoint.connect().await?;
    Ok(channel)
}