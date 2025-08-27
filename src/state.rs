// ========== FILE: src/state.rs ==========
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::info;

#[derive(Clone)]
pub struct ActiveCallInfo {
    pub remote_addr: SocketAddr,
    pub rtp_port: u32,
    pub trace_id: String,
    pub created_at: Instant,
    pub headers: HashMap<String, String>,
}

pub type ActiveCalls = Arc<Mutex<HashMap<String, ActiveCallInfo>>>;

pub async fn cleanup_old_transactions(transactions: ActiveCalls) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    loop {
        interval.tick().await;
        let mut guard = transactions.lock().await;
        let before_count = guard.len();
        guard.retain(|_call_id, call_info| call_info.created_at.elapsed() < Duration::from_secs(120));
        let after_count = guard.len();
        if before_count > after_count {
            info!(
                cleaned_count = before_count - after_count,
                remaining_count = after_count,
                "Temizlik görevi: Eski işlemler temizlendi."
            );
        }
    }
}