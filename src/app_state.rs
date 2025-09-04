// File: src/app_state.rs
use crate::config::AppConfig;
use crate::error::ServiceError;
use crate::grpc::client::create_all_grpc_clients;
use crate::rabbitmq;
use crate::redis;
use crate::state::ActiveCalls;
use lapin::Channel as LapinChannel;
use redis::Client as RedisClient;
use sentiric_contracts::sentiric::{
    dialplan::v1::dialplan_service_client::DialplanServiceClient,
    media::v1::media_service_client::MediaServiceClient,
    user::v1::user_service_client::UserServiceClient,
};
use std::sync::Arc;
use std::time::Duration; // YENİ
use tonic::transport::Channel as GrpcChannel;
use tracing::{info, warn}; // YENİ

pub struct GrpcClients {
    pub user: UserServiceClient<GrpcChannel>,
    pub dialplan: DialplanServiceClient<GrpcChannel>,
    pub media: MediaServiceClient<GrpcChannel>,
}

pub struct AppState {
    pub config: Arc<AppConfig>,
    pub active_calls: ActiveCalls,
    pub redis: Arc<RedisClient>,
    pub rabbit: Option<Arc<LapinChannel>>,
    pub grpc: GrpcClients,
}

impl AppState {
    // YENİ: Başlangıç mantığı artık kendi içinde retry içeriyor.
    pub async fn new_critical(config: Arc<AppConfig>) -> Result<Self, ServiceError> {
        const MAX_STARTUP_RETRIES: u32 = 10;
        const STARTUP_RETRY_DELAY: Duration = Duration::from_secs(5);
        
        let mut grpc_clients = None;
        for attempt in 1..=MAX_STARTUP_RETRIES {
            info!(attempt, max_attempts = MAX_STARTUP_RETRIES, "Kritik gRPC bağımlılıkları başlatılıyor...");
            match create_all_grpc_clients(config.as_ref()).await {
                Ok(clients) => {
                    grpc_clients = Some(clients);
                    info!("✅ Kritik gRPC bağımlılıkları başarıyla kuruldu.");
                    break;
                }
                Err(e) => {
                     warn!(
                        error = %e,
                        "Kritik gRPC bağımlılıkları başlatılamadı. {} saniye sonra tekrar denenecek...",
                        STARTUP_RETRY_DELAY.as_secs()
                    );
                    if attempt == MAX_STARTUP_RETRIES {
                        return Err(e); // Hata ile geri dön.
                    }
                    tokio::time::sleep(STARTUP_RETRY_DELAY).await;
                }
            }
        }

        info!("Kritik Redis bağımlılığı başlatılıyor...");
        let redis_client = Arc::new(redis::connect_with_retry(&config.redis_url).await);
        info!("✅ Kritik Redis bağımlılığı başarıyla kuruldu.");

        Ok(AppState {
            config,
            active_calls: Arc::new(Default::default()),
            redis: redis_client,
            rabbit: None,
            grpc: grpc_clients.expect("gRPC başlatma döngüsü `None` ile bitemez."),
        })
    }
    
    pub async fn connect_rabbitmq(&mut self) {
        if let Ok(rabbit_channel) = rabbitmq::connection::try_connect(&self.config.rabbitmq_url).await {
            rabbitmq::connection::declare_exchange(&rabbit_channel).await.ok();
            self.rabbit = Some(Arc::new(rabbit_channel));
        }
    }
}