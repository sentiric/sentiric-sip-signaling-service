// ========== DOSYA: sentiric-sip-signaling-service/src/sip/ack.rs (DERLENDİ VE DOĞRULANDI) ==========
use crate::app_state::AppState;
use crate::rabbitmq::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::utils::parse_complex_headers;
use crate::state::ActiveCallInfo;
use lapin::{options::BasicPublishOptions, BasicProperties};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{info, instrument, warn, Span};

#[instrument(skip_all, fields(remote_addr = %addr, call_id))]
pub async fn handle(
    request_str: &str,
    _sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        Span::current().record("call_id", &call_id as &str);
        info!("ACK isteği alındı, çağrı kurulumu tamamlanıyor.");

        // Klonlanmış veriyi tutmak için bir değişken tanımlıyoruz.
        let mut call_info_to_publish: Option<ActiveCallInfo> = None;

        // --- KİLİT KAPSAMI BAŞLANGICI ---
        // Bu blok, MutexGuard'ın ömrünü sınırlar. Blok bittiğinde kilit otomatik olarak serbest kalır.
        {
            let mut active_calls = state.active_calls.lock().await;
            if let Some(call_info) = active_calls.get_mut(&call_id) {
                Span::current().record("trace_id", &call_info.trace_id as &str);

                let mut answered_guard = call_info.answered_event_published.lock().await;
                if *answered_guard {
                    info!("Bu çağrı için 'call.answered' olayı zaten gönderilmiş, yinelenen ACK görmezden geliniyor.");
                    // Kapsamın sonuna ulaşıldığında kilitler otomatik serbest kalacak.
                } else {
                    // Bayrağı true yap.
                    *answered_guard = true;
                    // Olayı yayınlamak için gerekli bilgiyi klonla.
                    call_info_to_publish = Some(call_info.clone());
                }
            } else {
                warn!("ACK alınan çağrı aktif çağrılar listesinde bulunamadı.");
            }
        } // --- KİLİT KAPSAMI SONU --- `active_calls` kilidi burada serbest bırakıldı.

        // Artık kilit serbest, asenkron işlemi güvenle yapabiliriz.
        if let Some(call_info) = call_info_to_publish {
            if let Some(rabbit_channel) = &state.rabbit {
                publish_call_answered_event(&call_info, rabbit_channel).await?;
            } else {
                warn!("RabbitMQ bağlantısı aktif değil, 'call.answered' olayı yayınlanamadı.");
            }
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