// File: src/sip/utils.rs

use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;
use tracing::{info, warn};

static USER_EXTRACT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"sip:\+?(\d+)@").unwrap());

/// Gelen SIP isteğindeki başlıkları basit bir HashMap'e ayrıştırır.
/// Strateji B+ gereği, bu fonksiyon artık çoklu Via veya Record-Route başlıklarını
/// işlemek zorunda değildir. Bu sorumluluk `sip-gateway-service`'e aittir.
pub fn parse_complex_headers(request: &str) -> Option<HashMap<String, String>> {
    let mut headers = HashMap::new();
    for line in request.lines() {
        if line.is_empty() {
            break;
        }
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_string(), value.trim().to_string());
        }
    }
    
    if headers.contains_key("Via") {
        Some(headers)
    } else {
        warn!("Gelen SIP isteğinde Via başlığı bulunamadı (Gateway'den gelmemiş olabilir).");
        None
    }
}

// YENİ EKLENDİ: Bu fonksiyon derleme hatasını düzeltmek için geri eklendi.
// 'Contact' gibi başlıkların içinden URI'ı (<sip:...>) çıkarmak için kullanılır.
pub fn get_uri_from_header(header: &str) -> Option<String> {
    header.find('<').and_then(|start| header.find('>').map(|end| header[start + 1..end].to_string()))
}


pub fn extract_user_from_uri(uri: &str) -> Option<String> {
    USER_EXTRACT_RE.captures(uri).and_then(|caps| caps.get(1)).map(|user_part| {
        let original_num = user_part.as_str();
        let mut num: String = original_num.chars().filter(|c| c.is_digit(10)).collect();
        if num.len() == 11 && num.starts_with('0') {
            num = format!("90{}", &num[1..]);
        } else if num.len() == 10 && !num.starts_with("90") {
            num = format!("90{}", num);
        }
        let normalized_num = num;
        if original_num != normalized_num {
            info!(original = %original_num, normalized = %normalized_num, "Telefon numarası normalize edildi.");
        }
        normalized_num
    })
}

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