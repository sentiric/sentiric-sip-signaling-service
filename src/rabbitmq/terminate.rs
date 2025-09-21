// sentiric-sip-signaling-service/src/rabbitmq/terminate.rs

// Bu dosyanın tüm içeriği silinebilir veya boş bırakılabilir.
// Artık bu servis RabbitMQ'dan sonlandırma isteği dinlemeyecek.
// Bu sorumluluk gRPC'ye taşındı.

// use super::connection::RABBITMQ_EXCHANGE_NAME;
// use crate::app_state::AppState;
// use crate::error::ServiceError;
// use crate::sip::utils;
// use crate::state::ActiveCallInfo;
// use futures_util::StreamExt;
// use lapin::{options::*, types::FieldTable, BasicProperties, Channel as LapinChannel, Consumer};
// use rand::Rng;
// use serde::Deserialize;
// use std::sync::Arc;
// use tokio::net::UdpSocket;
// use tracing::{error, info, instrument, warn};

// #[derive(Deserialize, Debug)]
// struct TerminationRequest {
//     #[serde(rename = "callId")]
//     call_id: String,
// }

// #[instrument(skip_all)]
// pub async fn listen_for_termination_requests(sock: Arc<UdpSocket>, state: Arc<AppState>) {
//     if state.rabbit.is_none() {
//         warn!("RabbitMQ bağlantısı olmadığından çağrı sonlandırma dinleyicisi başlatılamadı.");
//         return;
//     }
//     let rabbit_channel = state.rabbit.as_ref().unwrap();
//     info!(queue = TERMINATION_QUEUE_NAME, "Çağrı sonlandırma kuyruğu dinleniyor...");
//     let consumer = match setup_consumer(rabbit_channel).await {
//         Ok(c) => c,
//         Err(e) => {
//             error!(error = %e, "Çağrı sonlandırma tüketicisi başlatılamadı.");
//             return;
//         }
//     };
//     process_messages(consumer, sock, state).await;
// }

// async fn setup_consumer(channel: &LapinChannel) -> Result<Consumer, ServiceError> {
//     let queue = channel.queue_declare(TERMINATION_QUEUE_NAME, QueueDeclareOptions { durable: true, ..Default::default() }, FieldTable::default()).await?;
//     channel.queue_bind(queue.name().as_str(), RABBITMQ_EXCHANGE_NAME, TERMINATION_ROUTING_KEY, QueueBindOptions::default(), FieldTable::default()).await?;
//     let consumer = channel.basic_consume(queue.name().as_str(), "signaling_service_terminator", BasicConsumeOptions::default(), FieldTable::default()).await?;
//     Ok(consumer)
// }

// async fn process_messages(mut consumer: Consumer, sock: Arc<UdpSocket>, state: Arc<AppState>) {
//     while let Some(delivery) = consumer.next().await {
//         if let Ok(delivery) = delivery {
//             let _ = delivery.ack(BasicAckOptions::default()).await;
//             match serde_json::from_slice::<TerminationRequest>(&delivery.data) {
//                 Ok(req) => handle_termination_request(req.call_id, &sock, &state).await,
//                 Err(e) => error!(error = %e, "Geçersiz sonlandırma isteği formatı."),
//             }
//         }
//     }
// }

// #[instrument(skip(sock, state))]
// async fn handle_termination_request(call_id: String, sock: &Arc<UdpSocket>, state: &Arc<AppState>) {
//     info!("Çağrı sonlandırma isteği işleniyor.");
//     if let Some(call_info) = state.active_calls.lock().await.remove(&call_id) {
//         let span = tracing::info_span!("terminate_call", trace_id = %call_info.trace_id, remote_addr = %call_info.remote_addr);
//         let _enter = span.enter();
//         info!("Aktif çağrı bulundu, BYE paketi oluşturuluyor ve gönderiliyor.");
        
//         let bye_request = create_bye_request(&call_info, &state.config);
        
//         if let Err(e) = sock.send_to(bye_request.as_bytes(), call_info.remote_addr).await {
//             error!(error = %e, "BYE paketi gönderilemedi.");
//         } else {
//             info!("BYE paketi başarıyla gönderildi.");
//         }
        
//         if let Some(rabbit_channel) = &state.rabbit {
//              let event_payload = serde_json::json!({
//                 "eventType": "call.ended", 
//                 "traceId": call_info.trace_id, 
//                 "callId": call_id,
//                 "reason": "terminated_by_request", 
//                 "timestamp": chrono::Utc::now().to_rfc3339()
//             });
//             if let Err(e) = rabbit_channel.basic_publish(
//                 RABBITMQ_EXCHANGE_NAME, 
//                 "call.ended", 
//                 BasicPublishOptions::default(), 
//                 event_payload.to_string().as_bytes(), 
//                 BasicProperties::default().with_delivery_mode(2)
//             ).await {
//                 error!(error = %e, "'call.ended' olayı yayınlanırken hata oluştu.");
//             } else {
//                  info!("'call.ended' olayı (terminate request sonrası) yayınlandı.");
//             }
//         } else {
//             warn!("RabbitMQ bağlantısı aktif değil, 'call.ended' olayı yayınlanamadı.");
//         }
//     } else {
//         warn!("Sonlandırılmak istenen çağrı aktif değil veya zaten sonlandırılmış.");
//     }
// }

// #[instrument(skip(call_info, config))]
// fn create_bye_request(call_info: &ActiveCallInfo, config: &crate::config::AppConfig) -> String {
//     let cseq_line = call_info.headers.get("CSeq").cloned().unwrap_or_default();
//     let cseq_num = cseq_line.split_whitespace().next().unwrap_or("1").parse::<u32>().unwrap_or(1) + 1;
//     let request_uri = utils::get_uri_from_header(&call_info.contact_header)
//         .unwrap_or_else(|| call_info.contact_header.clone());
//     let mut lines = Vec::new();
//     lines.push(format!("BYE {} SIP/2.0", request_uri));
//     let branch: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).map(char::from).collect();
//     lines.push(format!("Via: SIP/2.0/UDP {}:{};branch=z9hG4bK.{}", 
//         config.sip_listen_addr.ip(), 
//         config.sip_listen_addr.port(), 
//         branch));
//     lines.push(format!("Max-Forwards: 70"));
//     lines.push(format!("From: {};tag={}", call_info.to_header, call_info.to_tag));
//     lines.push(format!("To: {}", call_info.from_header));
//     lines.push(format!("Call-ID: {}", call_info.call_id));
//     lines.push(format!("CSeq: {} BYE", cseq_num));
//     lines.push(format!("User-Agent: Sentiric Signaling Service v{}", config.service_version));
//     lines.push(format!("Content-Length: 0"));
//     lines.join("\r\n") + "\r\n\r\n"
// }

// const TERMINATION_QUEUE_NAME: &str = "sentiric.signaling.terminate";
// const TERMINATION_ROUTING_KEY: &str = "call.terminate.request";