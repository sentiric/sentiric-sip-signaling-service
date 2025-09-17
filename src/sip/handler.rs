// ========== DOSYA: sentiric-sip-signaling-service/src/sip/handler.rs (TAM VE GÜNCEL İÇERİK) ==========
use super::{ack, bye, invite, register};
use crate::app_state::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::{debug, error, info, instrument};

#[instrument(skip_all, fields(remote_addr = %addr, call_id, trace_id))]
pub async fn handle_sip_request(
    request_bytes: Vec<u8>,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    state: Arc<AppState>,
) {
    let request_str = match std::str::from_utf8(&request_bytes) {
        Ok(s) => s,
        Err(e) => {
            error!(error = %e, "Geçersiz UTF-8 dizisi alındı.");
            return;
        }
    };

    debug!(
        request_from = %addr,
        request_body = %request_str.replace("\r\n", "\\r\n"),
        "Gelen ham SIP isteği."
    );

    let result = if request_str.starts_with("REGISTER") {
        info!("REGISTER isteği işleniyor...");
        register::handle(request_str, sock, addr, state).await
    } else if request_str.starts_with("INVITE") {
        info!("INVITE isteği işleniyor...");
        invite::handle(request_str, sock, addr, state).await
    } else if request_str.starts_with("BYE") {
        info!("BYE isteği işleniyor...");
        bye::handle(request_str, sock, addr, state).await
    } else if request_str.starts_with("ACK") {
        info!("ACK isteği işleniyor...");
        ack::handle(request_str, sock, addr, state).await
    } else {
        debug!(
            method = &request_str[..request_str.find(' ').unwrap_or(10)],
            "Desteklenmeyen veya ilgisiz SIP metodu, görmezden geliniyor."
        );
        Ok(())
    };

    if let Err(e) = result {
        error!(error = %e, "SIP isteği işlenirken hata oluştu.");
    }
}