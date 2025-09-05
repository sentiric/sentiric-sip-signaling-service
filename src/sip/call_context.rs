// File: src/sip/call_context.rs
use crate::error::ServiceError;
use crate::sip::utils;
use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::warn;

#[derive(Debug, Clone)]
pub struct CallContext {
    pub headers: HashMap<String, String>,
    pub raw_body: String,
    pub remote_addr: SocketAddr,
    pub call_id: String,
    pub from_header: String,
    pub to_header: String,
    pub contact_header: String,
    pub record_route_header: Option<String>,
    pub caller_id: String,
    pub destination_number: String,
    pub trace_id: String,
}

impl CallContext {
    pub fn from_request(request_str: &str, remote_addr: SocketAddr, trace_id: String) -> Result<Self, ServiceError> {
        let parts: Vec<&str> = request_str.split("\r\n\r\n").collect();
        let header_part = parts.get(0).unwrap_or(&"");
        let raw_body = parts.get(1).unwrap_or(&"").to_string();
        
        let mut headers = utils::parse_complex_headers(header_part)
            .ok_or_else(|| ServiceError::SipParse("SIP başlıkları ayrıştırılamadı".to_string()))?;
        
        // =========================================================================
        //   PRAGMATİK UYUMLULUK DÜZELTMESİ (OPERATÖR KAYNAKLI)
        // =========================================================================
        // Bazı telekom operatörlerinin (testlerde Sippy Softswitch v2021-PRODUCTION.408)
        // `Record-Route` başlığında "transport" yerine "trasport" yazım hatası yaptığı gözlemlenmiştir.
        // Bu hatalı parametre, giden BYE isteğimizde `Route` başlığı olarak geri
        // gönderildiğinde operatör tarafından reddedilmekteydi (`475 Bad URI`).
        // Gateway olarak, bu bilinen hatayı proaktif olarak düzelterek uyumluluğu artırıyoruz.

        // Telekom sağlayıcısının 'trasport' yazım hatasına uyum sağlamak için bu bloğu geçici olarak devre dışı bırakıyoruz.
        // Bu, giden BYE isteğindeki Route başlığının, sağlayıcının beklediği hatalı formatla eşleşmesini sağlayacaktır.
        // if let Some(rr) = headers.get_mut("Record-Route") {
        //     if rr.contains("trasport=") {
        //         // TS_KAREL_TRUST ve TS_ROITEL_TRUST operatörleri (şimdilik bilinenler)
        //         // `trasport=udp` şeklinde hatalı bir Record-Route başlığı gönderiyor.
        //         // Bu hatayı düzeltiyoruz.
        //         let fixed_rr = rr.replace("trasport=", "transport=");
        //         warn!(
        //             source = %remote_addr,
        //             original_record_route = %rr,
        //             fixed_record_route = %fixed_rr,
        //             "Gelen Record-Route başlığında 'trasport' yazım hatası tespit edildi, düzeltiliyor."
        //         );
        //         *rr = fixed_rr;
        //     }
        // }
        
        // =========================================================================            

        
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        let from_header = headers.get("From").cloned().unwrap_or_default();
        let to_header = headers.get("To").cloned().unwrap_or_default();
        let contact_header = headers.get("Contact").cloned().unwrap_or_default();
        let record_route_header = headers.get("Record-Route").cloned();
        let caller_id = utils::extract_user_from_uri(&from_header).unwrap_or_else(|| "unknown".to_string());
        let destination_number = utils::extract_user_from_uri(&to_header).unwrap_or_else(|| "unknown".to_string());

        Ok(Self {
            headers, raw_body, remote_addr, call_id, from_header, to_header,
            contact_header, record_route_header, caller_id, destination_number, trace_id,
        })
    }
}