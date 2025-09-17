// ========== DOSYA: sentiric-sip-signaling-service/src/sip/ack.rs (YENİ VE TAM DOSYA) ==========
use crate::app_state::AppState;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::utils::parse_complex_headers;
use crate::state::ActiveCallInfo;
use lapin::{options::BasicPublishOptions, BasicProperties};
use std::error::Error;
use std::net::SocketAddr; // <-- EKSİK OLAN IMPORT EKLENDİ
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{info, instrument, warn, Span};

#[instrument(skip_all, fields(remote_addr = %addr, call_id))]
pub async fn handle(
    request_str: &str,
    _sock: Arc<UdpSocket>,
    _addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        Span::current().record("call_id", &call_id as &str);
        info!("ACK isteği alındı, çağrı kurulumu tamamlanıyor.");

        let mut active_calls = state.active_calls.lock().await;
        if let Some(call_info) = active_calls.get_mut(&call_id) {
            Span::current().record("trace_id", &call_info.trace_id as &str);
            
            // Sadece bir kez "answered" olayı göndermek için kontrol
            let mut answered_guard = call_info.answered_event_published.lock().await;
            if *answered_guard {
                info!("Bu çağrı için 'call.answered' olayı zaten gönderilmiş, yinelenen ACK görmezden geliniyor.");
                return Ok(());
            }
            
            // Bayrağı true yap ve olayı gönder
            *answered_guard = true;
            
            // active_calls kilidini, rabbitmq'ya göndermeden önce serbest bırakalım
            let call_info_clone = call_info.clone();
            drop(active_calls);

            if let Some(rabbit_channel) = &state.rabbit {
                publish_call_answered_event(&call_info_clone, rabbit_channel).await?;
            } else {
                warn!("RabbitMQ bağlantısı aktif değil, 'call.answered' olayı yayınlanamadı.");
            }
        } else {
            warn!("ACK alınan çağrı aktif çağrılar listesinde bulunamadı.");
        }
    }
    Ok(())
}

#[instrument(skip_all, fields(trace_id = %call_info.trace_id, call_id = %call_info.call_id))]
async fn publish_call_answered_event(
    call_info: &ActiveCallInfo,
    rabbit_channel: &Arc<lapin::Channel>,
) -> Result<(), crate::error::ServiceError> {
    let event_payload = serde_json::json!({
        "eventType": "call.answered",
        "traceId": &call_info.trace_id,
        "callId": &call_info.call_id,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    rabbit_channel
        .basic_publish(
            RABBITMQ_EXCHANGE_NAME,
            "call.answered",
            BasicPublishOptions::default(),
            event_payload.to_string().as_bytes(),
            BasicProperties::default().with_delivery_mode(2),
        )
        .await?
        .await?;
    
    info!("'call.answered' olayı başarıyla yayınlandı.");
    Ok(())
}