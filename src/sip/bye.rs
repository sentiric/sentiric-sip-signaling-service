// File: sentiric-sip-signaling-service/src/sip/bye.rs

use super::utils::{create_response, parse_complex_headers};
use crate::config::AppConfig;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::state::ActiveCalls;
use lapin::{options::*, BasicProperties, Channel as LapinChannel};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{info, warn, Span};

pub async fn handle_bye(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        Span::current().record("call_id", &call_id as &str);
        info!("BYE isteği alındı.");

        let ok_response = create_response("200 OK", &headers, None, &config);
        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!("BYE isteğine 200 OK yanıtı gönderildi.");

        if let Some(call_info) = active_calls.lock().await.remove(&call_id) {
            Span::current().record("trace_id", &call_info.trace_id as &str);
            info!(port = call_info.rtp_port, "Çağrı sonlandırılıyor, olay yayınlanacak.");

            let event_payload = serde_json::json!({
                "eventType": "call.ended",
                "traceId": call_info.trace_id,
                "callId": call_id,
                "reason": "normal_clearing",
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            rabbit_channel
                .basic_publish(
                    RABBITMQ_EXCHANGE_NAME,
                    "call.ended", // <<< DEĞİŞİKLİK BURADA
                    BasicPublishOptions::default(),
                    event_payload.to_string().as_bytes(),
                    BasicProperties::default().with_delivery_mode(2),
                )
                .await?
                .await?;
            info!("'call.ended' olayı başarıyla yayınlandı.");

            warn!(port = call_info.rtp_port, "Port, agent'ın son işlemleri için açık bırakıldı. Karantina mekanizması temizleyecek.");
        } else {
            warn!("BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı.");
        }
    }
    Ok(())
}