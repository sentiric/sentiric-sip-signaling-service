// sentiric-sip-signaling-service/src/sip/utils.rs
use super::call_context::CallContext; // `call_context`'e erişim için
use crate::config::AppConfig;
use crate::state::ActiveCallInfo;
use once_cell::sync::Lazy;
use rand::distributions::{Alphanumeric, DistString};
use rand::Rng;
use regex::Regex;
use std::collections::HashMap;
use tracing::{info, instrument, warn};

static USER_EXTRACT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"sip:\+?(\d+)@").unwrap());

// DÜZELTME: Fonksiyon artık (HashMap, Vec<String>) döndürüyor.
pub fn parse_sip_headers(header_section: &str) -> Option<(HashMap<String, String>, Vec<String>)> {
    let mut headers = HashMap::new();
    let mut via_headers = Vec::new();
    
    for line in header_section.lines() {
        if line.is_empty() { continue; }
        if let Some((key, value)) = line.split_once(':') {
            let key_trimmed = key.trim();
            let key_lower = key_trimmed.to_lowercase();
            
            // Via başlıklarını sırasıyla bir vektöre ekle.
            if key_lower == "via" || key_lower == "v" {
                via_headers.push(line.to_string());
            } else {
                headers.insert(key_lower, value.trim().to_string());
            }
        }
    }
    
    if !via_headers.is_empty() {
        Some((headers, via_headers))
    } else {
        warn!("Gelen SIP isteğinde 'via' başlığı bulunamadı.");
        None
    }
}


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

#[instrument(skip_all, fields(call_id = %call_info.call_id))]
pub fn create_bye_request(call_info: &ActiveCallInfo, config: &AppConfig) -> String {
    let cseq_line = call_info.headers.get("CSeq").cloned().unwrap_or_default();
    let cseq_num = cseq_line.split_whitespace().next().unwrap_or("1").parse::<u32>().unwrap_or(1) + 1;
    let request_uri = get_uri_from_header(&call_info.contact_header)
        .unwrap_or_else(|| call_info.contact_header.clone());
    
    let mut lines = Vec::new();
    lines.push(format!("BYE {} SIP/2.0", request_uri));

    let branch: String = rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).map(char::from).collect();
    lines.push(format!("Via: SIP/2.0/UDP {}:{};branch=z9hG4bK.{}", 
        config.sip_listen_addr.ip(), 
        config.sip_listen_addr.port(), 
        branch));
    
    lines.push(format!("Max-Forwards: 70"));
    lines.push(format!("From: {};tag={}", call_info.to_header, call_info.to_tag));
    lines.push(format!("To: {}", call_info.from_header));
    lines.push(format!("Call-ID: {}", call_info.call_id));
    lines.push(format!("CSeq: {} BYE", cseq_num));
    lines.push(format!("User-Agent: Sentiric Signaling Service v{}", config.service_version));
    lines.push(format!("Content-Length: 0"));
    
    lines.join("\r\n") + "\r\n\r\n"
}