// File: sentiric-sip-signaling-service/src/sip/bye.rs (TAM VE GÜNCELLENMİŞ HALİ)

use super::utils::{create_response, parse_complex_headers};
use crate::config::AppConfig;
use crate::state::ActiveCalls;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{info, warn, Span};

// RabbitMQ ile ilgili importlar ve parametreler kaldırıldı.
pub async fn handle_bye(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        Span::current().record("call_id", &call_id as &str);
        info!("BYE isteği alındı.");

        let ok_response = create_response("200 OK", &headers, None, &config, addr);
        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!("BYE isteğine 200 OK yanıtı gönderildi.");

        // --- DEĞİŞİKLİK BURADA: Sadece kaydı sil, olay yayınlama yok ---
        // Genellikle terminate_request daha hızlı davranıp bu kaydı silmiş olur.
        // Bu kod, bir güvenlik önlemi olarak kalır.
        if let Some(call_info) = active_calls.lock().await.remove(&call_id) {
            Span::current().record("trace_id", &call_info.trace_id as &str);
            info!(port = call_info.rtp_port, "Çağrı kullanıcı tarafından sonlandırıldı, aktif çağrı kaydı silindi.");
            
            // Olay yayınlama kısmı burada olmayacak. Bu, terminate_request'in görevi.
            warn!(port = call_info.rtp_port, "Port, agent'ın son işlemleri için açık bırakıldı. Karantina mekanizması temizleyecek.");
        } else {
            warn!("BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı (muhtemelen agent tarafından zaten sonlandırılmıştı).");
        }
    }
    Ok(())
}