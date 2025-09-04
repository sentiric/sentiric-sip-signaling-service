// File: src/sip/bye.rs
use super::utils::parse_complex_headers;
use crate::app_state::AppState;
use crate::sip::responses;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{info, warn, Span};

// Fonksiyon imzası AppState alacak şekilde güncellendi.
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

        // Yanıt oluşturucu fonksiyonu kullanılıyor.
        let ok_response = responses::create_response("200 OK", &headers, None, &state.config, addr);
        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!("BYE isteğine 200 OK yanıtı gönderildi.");

        if let Some(call_info) = state.active_calls.lock().await.remove(&call_id) {
            Span::current().record("trace_id", &call_info.trace_id as &str);
            info!(port = call_info.rtp_port, "Çağrı kullanıcı tarafından sonlandırıldı, aktif çağrı kaydı silindi.");
            warn!(port = call_info.rtp_port, "Port, agent'ın son işlemleri için açık bırakıldı. Karantina mekanizması temizleyecek.");
        } else {
            warn!("BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı (muhtemelen agent tarafından zaten sonlandırılmıştı).");
        }
    }
    Ok(())
}