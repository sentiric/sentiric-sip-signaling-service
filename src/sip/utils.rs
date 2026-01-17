// sentiric-sip-signaling-service/src/sip/utils.rs

use crate::config::AppConfig;
use crate::state::ActiveCallInfo;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use tracing::{warn}; // Info gerekirse eklenebilir

static USER_EXTRACT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"sip:\+?(\d+)@").unwrap());

// Header'ları ve özellikle VIA header'larını (sıralı olarak) ayrıştırır.
pub fn parse_sip_headers(header_section: &str) -> Option<(HashMap<String, String>, Vec<String>)> {
    let mut headers = HashMap::new();
    let mut via_headers = Vec::new();
    
    for line in header_section.lines() {
        if line.trim().is_empty() { continue; }
        
        if let Some((key, value)) = line.split_once(':') {
            let key_trimmed = key.trim().to_lowercase();
            let val_trimmed = value.trim().to_string();
            
            // Compact form desteği
            let key_normalized = match key_trimmed.as_str() {
                "v" => "via",
                "f" => "from",
                "t" => "to",
                "i" => "call-id",
                "m" => "contact",
                "l" => "content-length",
                "c" => "content-type",
                _ => key_trimmed.as_str(),
            };

            if key_normalized == "via" {
                via_headers.push(val_trimmed);
            } else {
                headers.insert(key_normalized.to_string(), val_trimmed);
            }
        }
    }
    
    if !via_headers.is_empty() {
        Some((headers, via_headers))
    } else {
        warn!("SIP mesajında 'Via' başlığı bulunamadı.");
        None // Via'sız SIP mesajı geçersizdir.
    }
}

pub fn get_uri_from_header(header: &str) -> Option<String> {
    header.find('<')
        .and_then(|start| header.find('>').map(|end| header[start + 1..end].to_string()))
}

pub fn extract_user_from_uri(uri: &str) -> Option<String> {
    USER_EXTRACT_RE.captures(uri).and_then(|caps| caps.get(1)).map(|user_part| {
        let original_num = user_part.as_str();
        // Sadece rakamları al
        let mut num: String = original_num.chars().filter(|c| c.is_digit(10)).collect();
        
        // Türkiye formatı normalizasyonu (Opsiyonel ama yararlı)
        if num.len() == 11 && num.starts_with('0') {
            num = format!("90{}", &num[1..]);
        } else if num.len() == 10 && !num.starts_with("90") {
            num = format!("90{}", num);
        }
        num
    })
}

// SDP Body içinden Media IP ve Portunu çeker
pub fn extract_sdp_media_info_from_body(sip_body: &str) -> Option<String> {
    let mut ip_addr: Option<&str> = None;
    let mut port: Option<&str> = None;
    
    for line in sip_body.lines() {
        if line.starts_with("c=IN IP4 ") {
            ip_addr = line.split_whitespace().nth(2);
        }
        if line.starts_with("m=audio ") {
            port = line.split_whitespace().nth(1);
        }
    }
    
    if let (Some(ip), Some(p)) = (ip_addr, port) {
        Some(format!("{}:{}", ip, p))
    } else {
        None
    }
}

// BYE Paketi Oluşturucu
pub fn create_bye_request(call_info: &ActiveCallInfo, config: &AppConfig) -> String {
    let cseq_line = call_info.headers.get("cseq").cloned().unwrap_or_else(|| "1 INVITE".to_string());
    // CSeq numarasını artır
    let cseq_num = cseq_line.split_whitespace().next().unwrap_or("1").parse::<u32>().unwrap_or(1) + 1;
    
    let request_uri = get_uri_from_header(&call_info.contact_header)
        .unwrap_or_else(|| call_info.contact_header.clone());
    
    let branch: u32 = rand::thread_rng().gen();
    
    format!(
        "BYE {} SIP/2.0\r\n\
        Via: SIP/2.0/UDP {}:{};branch=z9hG4bK.{}\r\n\
        Max-Forwards: 70\r\n\
        From: {};tag={}\r\n\
        To: {}\r\n\
        Call-ID: {}\r\n\
        CSeq: {} BYE\r\n\
        User-Agent: Sentiric Signaling v{}\r\n\
        Content-Length: 0\r\n\
        \r\n",
        request_uri,
        config.sip_public_ip, config.sip_listen_addr.port(), branch,
        call_info.to_header, call_info.to_tag,
        call_info.from_header,
        call_info.call_id,
        cseq_num,
        config.service_version
    )
}