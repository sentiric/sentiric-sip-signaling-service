// File: sentiric-sip-signaling-service/src/rabbitmq/terminate.rs

use super::connection::RABBITMQ_EXCHANGE_NAME;
use crate::sip::utils::create_bye_request;
use crate::state::ActiveCalls;
use futures_util::StreamExt;
use lapin::{options::*, types::FieldTable, BasicProperties, Channel as LapinChannel, Consumer};
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{error, info, instrument, warn};

const TERMINATION_QUEUE_NAME: &str = "sentiric.signaling.terminate";

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

    let mut consumer: Consumer = match rabbit_channel
        .queue_declare(
            TERMINATION_QUEUE_NAME,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await
    {
        Ok(queue) => {
            if let Err(e) = rabbit_channel
                .queue_bind(
                    queue.name().as_str(),
                    RABBITMQ_EXCHANGE_NAME,
                    "call.terminate.request",
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await
            {
                error!(error = %e, "Kuyruk exchange'e bağlanamadı.");
                return;
            }
            match rabbit_channel
                .basic_consume(
                    queue.name().as_str(),
                    "signaling_service_terminator",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    error!(error = %e, "RabbitMQ consumer oluşturulamadı.");
                    return;
                }
            }
        }
        Err(e) => {
            error!(error = %e, "RabbitMQ kuyruğu oluşturulamadı.");
            return;
        }
    };

    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            let _ = delivery.ack(BasicAckOptions::default()).await;
            if let Ok(req) = serde_json::from_slice::<TerminationRequest>(&delivery.data) {
                let call_id = req.call_id;
                info!(call_id = %call_id, "Çağrı sonlandırma isteği alındı.");

                if let Some(call_info) = active_calls.lock().await.remove(&call_id) {
                    let span = tracing::info_span!("terminate_call", call_id = %call_id, trace_id = %call_info.trace_id, remote_addr = %call_info.remote_addr);
                    let _enter = span.enter();

                    info!("Aktif çağrı bulundu, BYE paketi gönderiliyor.");
                    let bye_request = create_bye_request(&call_info.headers);
                    if let Err(e) = sock.send_to(bye_request.as_bytes(), call_info.remote_addr).await {
                        error!(error = %e, "BYE paketi gönderilemedi.");
                    } else {
                        info!("BYE paketi başarıyla gönderildi.");
                    }

                    let event_payload = serde_json::json!({
                        "eventType": "call.ended",
                        "traceId": call_info.trace_id,
                        "callId": call_id,
                        "reason": "terminated_by_request",
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    });
                    let _ = rabbit_channel
                        .basic_publish(
                            RABBITMQ_EXCHANGE_NAME,
                            "call.ended", // <<< DEĞİŞİKLİK BURADA
                            BasicPublishOptions::default(),
                            event_payload.to_string().as_bytes(),
                            BasicProperties::default().with_delivery_mode(2),
                        )
                        .await;
                    info!("'call.ended' olayı yayınlandı.");
                } else {
                    warn!(call_id = %call_id, "Sonlandırılmak istenen çağrı aktif değil.");
                }
            } else {
                error!("Geçersiz sonlandırma isteği formatı.");
            }
        }
    }
}