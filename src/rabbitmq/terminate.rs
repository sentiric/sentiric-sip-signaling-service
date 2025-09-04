// File: src/rabbitmq/terminate.rs
use super::connection::RABBITMQ_EXCHANGE_NAME;
use crate::error::ServiceError;
use crate::sip::utils::create_bye_request;
use crate::state::ActiveCalls;
use futures_util::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Channel as LapinChannel, Consumer};
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, instrument, warn};

const TERMINATION_QUEUE_NAME: &str = "sentiric.signaling.terminate";
const TERMINATION_ROUTING_KEY: &str = "call.terminate.request";

#[derive(Deserialize, Debug)]
struct TerminationRequest {
    #[serde(rename = "callId")]
    call_id: String,
}

#[instrument(skip_all)]
pub async fn listen_for_termination_requests(
    sock: Arc<UdpSocket>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
) {
    info!(queue = TERMINATION_QUEUE_NAME, "Çağrı sonlandırma kuyruğu dinleniyor...");
    let consumer = match setup_consumer(&rabbit_channel).await {
        Ok(c) => c,
        Err(e) => {
            error!(error = %e, "Çağrı sonlandırma tüketicisi başlatılamadı.");
            return;
        }
    };
    process_messages(consumer, sock, rabbit_channel, active_calls).await;
}

async fn setup_consumer(channel: &LapinChannel) -> Result<Consumer, ServiceError> {
    let queue = channel.queue_declare(TERMINATION_QUEUE_NAME, QueueDeclareOptions { durable: true, ..Default::default() }, FieldTable::default()).await?;
    channel.queue_bind(queue.name().as_str(), RABBITMQ_EXCHANGE_NAME, TERMINATION_ROUTING_KEY, QueueBindOptions::default(), FieldTable::default()).await?;
    let consumer = channel.basic_consume(queue.name().as_str(), "signaling_service_terminator", BasicConsumeOptions::default(), FieldTable::default()).await?;
    Ok(consumer)
}

async fn process_messages(
    mut consumer: Consumer,
    sock: Arc<UdpSocket>,
    rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
) {
    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let _ = delivery.ack(BasicAckOptions::default()).await;
            match serde_json::from_slice::<TerminationRequest>(&delivery.data) {
                Ok(req) => handle_termination_request(req.call_id, &sock, &rabbit_channel, &active_calls).await,
                Err(e) => error!(error = %e, "Geçersiz sonlandırma isteği formatı."),
            }
        }
    }
}

#[instrument(skip(sock, rabbit_channel, active_calls))]
async fn handle_termination_request(
    call_id: String,
    sock: &Arc<UdpSocket>,
    rabbit_channel: &Arc<LapinChannel>,
    active_calls: &ActiveCalls,
) {
    info!("Çağrı sonlandırma isteği işleniyor.");
    if let Some(call_info) = active_calls.lock().await.remove(&call_id) {
        let span = tracing::info_span!("terminate_call", trace_id = %call_info.trace_id, remote_addr = %call_info.remote_addr);
        let _enter = span.enter();
        info!("Aktif çağrı bulundu, BYE paketi oluşturuluyor ve gönderiliyor.");
        
        let bye_request = create_bye_request(&call_info);
        
        if let Err(e) = sock.send_to(bye_request.as_bytes(), call_info.remote_addr).await {
            error!(error = %e, "BYE paketi gönderilemedi.");
        } else {
            info!("BYE paketi başarıyla gönderildi.");
        }
        let event_payload = serde_json::json!({
            "eventType": "call.ended", "traceId": call_info.trace_id, "callId": call_id,
            "reason": "terminated_by_request", "timestamp": chrono::Utc::now().to_rfc3339()
        });
        if let Err(e) = rabbit_channel.basic_publish(RABBITMQ_EXCHANGE_NAME, "call.ended", BasicPublishOptions::default(), event_payload.to_string().as_bytes(), BasicProperties::default().with_delivery_mode(2)).await {
            error!(error = %e, "'call.ended' olayı yayınlanırken hata oluştu.");
        } else {
             info!("'call.ended' olayı yayınlandı.");
        }
    } else {
        warn!("Sonlandırılmak istenen çağrı aktif değil veya zaten sonlandırılmış.");
    }
}