use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone, Debug)]
pub struct ActiveCallInfo {
    pub remote_addr: SocketAddr,
    pub rtp_port: u32,
    pub trace_id: String,
    pub to_tag: String,
    pub created_at: Instant,
    pub headers: HashMap<String, String>,
    pub via_headers: Vec<String>, // Yanıt verirken ayna gibi geri yansıtılacak
    pub call_id: String,
    pub from_header: String,
    pub to_header: String,
    pub contact_header: String,
    #[allow(dead_code)]
    pub record_route_header: Option<String>,
    pub raw_body: String,
    pub answered_event_published: Arc<Mutex<bool>>, 
    // [YENİ] Retransmission için son üretilen başarılı yanıtı saklayabiliriz (Optimization)
    // Şimdilik sadece gerekli alanları tutuyoruz.
}

pub type ActiveCalls = Arc<Mutex<HashMap<String, ActiveCallInfo>>>;

pub async fn cleanup_old_transactions(transactions: ActiveCalls) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let mut guard = transactions.lock().await;
        let before_count = guard.len();
        // 5 dakikadan eski çağrıları temizle (Normalde BYE ile silinmeli ama sızıntı koruması)
        guard.retain(|_call_id, call_info| call_info.created_at.elapsed() < Duration::from_secs(300));
        let after_count = guard.len();
        if before_count > after_count {
            info!(
                cleaned = before_count - after_count,
                remaining = after_count,
                "🧹 Eski çağrı kayıtları temizlendi."
            );
        }
    }
}