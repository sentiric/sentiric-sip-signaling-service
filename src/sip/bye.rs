use super::utils::parse_complex_headers;
use crate::app_state::AppState;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::responses;
use lapin::{options::BasicPublishOptions, BasicProperties};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{info, warn, Span};

pub async fn handle(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        Span::current().record("call_id", &call_id as &str);
        info!("BYE isteği alındı.");

        let ok_response = responses::create_response("200 OK", &headers, None, &state.config, addr);
        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!("BYE isteğine 200 OK yanıtı gönderildi.");

        if let Some(call_info) = state.active_calls.lock().await.remove(&call_id) {
            Span::current().record("trace_id", &call_info.trace_id as &str);
            info!(port = call_info.rtp_port, "Çağrı kullanıcı tarafından sonlandırıldı, aktif çağrı kaydı silindi.");

            // --- YENİ MANTIK BAŞLANGICI ---
            if let Some(rabbit_channel) = &state.rabbit {
                let event_payload = serde_json::json!({
                    "eventType": "call.ended",
                    "traceId": call_info.trace_id,
                    "callId": call_id,
                    "reason": "normal_clearing_by_user",
                    "timestamp": chrono::Utc::now().to_rfc3339()
                });

                if let Err(e) = rabbit_channel.basic_publish(
                    RABBITMQ_EXCHANGE_NAME,
                    "call.ended",
                    BasicPublishOptions::default(),
                    event_payload.to_string().as_bytes(),
                    BasicProperties::default().with_delivery_mode(2),
                ).await {
                    error!(error = %e, "'call.ended' olayı yayınlanırken hata oluştu.");
                } else {
                    info!("'call.ended' olayı başarıyla yayınlandı.");
                }
            } else {
                warn!("RabbitMQ bağlantısı aktif değil, 'call.ended' olayı yayınlanamadı.");
            }
            // --- YENİ MANTIK SONU ---

            warn!(port = call_info.rtp_port, "Port, agent'ın son işlemleri için açık bırakıldı. Karantina mekanizması temizleyecek.");
        } else {
            warn!("BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı (muhtemelen agent tarafından zaten sonlandırılmıştı).");
        }
    }
    Ok(())
}