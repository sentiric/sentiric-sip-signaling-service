// sentiric-sip-signaling-service/src/sip/bye.rs

use crate::app_state::AppState;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::responses;
use crate::sip::utils::parse_sip_headers;
use crate::sip::utils::extract_sdp_media_info_from_body; // Gerekirse ekleyin
use lapin::{options::BasicPublishOptions, BasicProperties};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, instrument, warn, Span};

#[instrument(skip_all, fields(remote_addr = %addr, call_id))]
pub async fn handle(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let (headers, via_headers) = parse_sip_headers(request_str).ok_or("Geçersiz başlıklar")?;
    let call_id = headers.get("call-id").cloned().unwrap_or_default();
    Span::current().record("call_id", &call_id as &str);
    info!("BYE isteği alındı.");

    let ok_response = responses::create_response_from_parts("200 OK", &headers, &via_headers, None, &state.config, addr);
    sock.send_to(ok_response.as_bytes(), addr).await?;
    info!("BYE isteğine 200 OK yanıtı gönderildi.");

    if let Some(call_info) = state.active_calls.lock().await.remove(&call_id) {
        Span::current().record("trace_id", &call_info.trace_id as &str);
        info!(port = call_info.rtp_port, "Çağrı kullanıcı tarafından sonlandırıldı.");

        if let Some(rabbit_channel) = &state.rabbit {
            
            // --- GÜNCELLEME: MediaInfo Ekleme ---
            let sdp_info = extract_sdp_media_info_from_body(&call_info.raw_body).unwrap_or_default();
            
            let event_payload = serde_json::json!({
                "eventType": "call.ended",
                "traceId": call_info.trace_id,
                "callId": call_id,
                "reason": "normal_clearing_by_user",
                "timestamp": chrono::Utc::now().to_rfc3339(),
                // YENİ: Agent servisin portu temizleyebilmesi için
                "mediaInfo": {
                    "callerRtpAddr": sdp_info,
                    "serverRtpPort": call_info.rtp_port
                }
            });
            // ------------------------------------

            if let Err(e) = rabbit_channel.basic_publish(
                RABBITMQ_EXCHANGE_NAME,
                "call.ended",
                BasicPublishOptions::default(),
                event_payload.to_string().as_bytes(),
                BasicProperties::default().with_delivery_mode(2).with_content_type("application/json".into()),
            ).await {
                error!(error = %e, "'call.ended' olayı yayınlanırken hata oluştu.");
            } else {
                info!("'call.ended' olayı başarıyla yayınlandı.");
            }
        } else {
            warn!("RabbitMQ bağlantısı aktif değil, 'call.ended' olayı yayınlanamadı.");
        }
        
    } else {
        warn!("BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı.");
    }
    Ok(())
}