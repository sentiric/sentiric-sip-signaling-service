// sentiric-sip-signaling-service/src/sip/call_context.rs

use crate::error::ServiceError;
use crate::sip::utils;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct CallContext {
    pub headers: HashMap<String, String>,
    pub via_headers: Vec<String>, // Orijinal sıralamayı korumak için Vec
    pub raw_body: String,
    pub remote_addr: SocketAddr,
    
    // Sık kullanılan header'lar için kısayollar
    pub call_id: String,
    pub from_header: String,
    pub to_header: String,
    pub contact_header: String,
    pub record_route_header: Option<String>,
    
    // İş mantığı için ayrıştırılmış alanlar
    pub caller_id: String,
    pub destination_number: String,
    pub trace_id: String,
}

impl CallContext {
    pub fn from_request(request_str: &str, remote_addr: SocketAddr, trace_id: String) -> Result<Self, ServiceError> {
        // Header ve Body Ayrımı (Çift CRLF)
        let parts: Vec<&str> = request_str.splitn(2, "\r\n\r\n").collect();
        let header_part = parts.get(0).unwrap_or(&"");
        let raw_body = parts.get(1).unwrap_or(&"").to_string();
        
        // Headerları Parse Et (utils modülünü kullanıyoruz)
        let (headers, via_headers) = utils::parse_sip_headers(header_part)
            .ok_or_else(|| ServiceError::SipParse("SIP başlıkları okunamadı".to_string()))?;
        
        // Kritik alanları çek
        let call_id = headers.get("call-id").cloned().unwrap_or_default();
        let from_header = headers.get("from").cloned().unwrap_or_default();
        let to_header = headers.get("to").cloned().unwrap_or_default();
        let contact_header = headers.get("contact").cloned().unwrap_or_default();
        let record_route_header = headers.get("record-route").cloned();
        
        // Numaraları ayıkla
        let caller_id = utils::extract_user_from_uri(&from_header).unwrap_or_else(|| "unknown".to_string());
        let destination_number = utils::extract_user_from_uri(&to_header).unwrap_or_else(|| "unknown".to_string());

        if call_id.is_empty() {
             return Err(ServiceError::SipParse("Call-ID eksik".to_string()));
        }

        Ok(Self {
            headers,
            via_headers,
            raw_body,
            remote_addr,
            call_id,
            from_header,
            to_header,
            contact_header,
            record_route_header,
            caller_id,
            destination_number,
            trace_id,
        })
    }
}