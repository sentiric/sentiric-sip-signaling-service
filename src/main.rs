use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::{sleep, Duration};
use rand::Rng;
use tracing::{info, error, debug, instrument, Level};
use tracing_subscriber::FmtSubscriber;
use tonic::transport::Channel;
use lapin::{Connection, ConnectionProperties, options::*, types::FieldTable, BasicProperties, Channel as LapinChannel};
use chrono::Utc;
use serde_json::json;

// DÜZELTİLMİŞ IMPORT'LAR:
// Artık sadece 'sentiric-contracts' içinde gerçekten var olan
// mesajları ve istemcileri import ediyoruz.
use sentiric_contracts::sentiric::{
    media::v1::{media_service_client::MediaServiceClient, AllocatePortRequest},
    user::v1::{user_service_client::UserServiceClient, GetUserRequest},
    dialplan::v1::{dialplan_service_client::DialplanServiceClient, GetDialplanForUserRequest},
};

const RABBITMQ_QUEUE_NAME: &str = "call.events";

//--- Konfigürasyon Yapısı (Değişiklik yok) ---
#[derive(Debug, Clone)]
struct AppConfig {
    sip_listen_addr: SocketAddr,
    sip_public_ip: String,
    user_service_url: String,
    dialplan_service_url: String,
    media_service_url: String,
    rabbitmq_url: String,
}

impl AppConfig {
    fn load_from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let sip_host = env::var("SIP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port_str = env::var("SIP_PORT").unwrap_or_else(|_| "5060".to_string());
        let sip_port = sip_port_str.parse::<u16>()
            .map_err(|e| format!("Geçersiz SIP_PORT değeri: '{}'. Bir sayı olmalı. Hata: {}", sip_port_str, e))?;
        
        let sip_listen_addr = format!("{}:{}", sip_host, sip_port).parse()?;
        
        Ok(AppConfig {
            sip_listen_addr,
            sip_public_ip: env::var("PUBLIC_IP")
                .map_err(|_| "Gerekli ortam değişkeni ayarlanmamış: PUBLIC_IP".to_string())?,
            user_service_url: env::var("USER_SERVICE_GRPC_URL")
                .map_err(|_| "Gerekli ortam değişkeni ayarlanmamış: USER_SERVICE_GRPC_URL".to_string())?,
            dialplan_service_url: env::var("DIALPLAN_SERVICE_GRPC_URL")
                .map_err(|_| "Gerekli ortam değişkeni ayarlanmamış: DIALPLAN_SERVICE_GRPC_URL".to_string())?,
            media_service_url: env::var("MEDIA_SERVICE_GRPC_URL")
                .map_err(|_| "Gerekli ortam değişkeni ayarlanmamış: MEDIA_SERVICE_GRPC_URL".to_string())?,
            rabbitmq_url: env::var("RABBITMQ_URL")
                .map_err(|_| "Gerekli ortam değişkeni ayarlanmamış: RABBITMQ_URL".to_string())?,
        })
    }
}

//--- Ana Fonksiyon (Değişiklik yok) ---
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok(); // .env dosyasını yükler
    
    let subscriber = FmtSubscriber::builder().with_max_level(Level::DEBUG).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let config = Arc::new(AppConfig::load_from_env()?);
    info!(config = ?config, "Konfigürasyon başarıyla yüklendi");

    let rabbit_conn = Connection::connect(&config.rabbitmq_url, ConnectionProperties::default()).await?;
    let rabbit_channel = Arc::new(rabbit_conn.create_channel().await?);
    
    rabbit_channel.confirm_select(ConfirmSelectOptions::default()).await?;
    rabbit_channel.queue_declare(RABBITMQ_QUEUE_NAME, QueueDeclareOptions { durable: true, ..Default::default() }, FieldTable::default()).await?;
    info!("RabbitMQ bağlantısı başarıyla kuruldu ve '{}' kuyruğu deklare edildi.", RABBITMQ_QUEUE_NAME);

    let sock = Arc::new(UdpSocket::bind(config.sip_listen_addr).await?);
    info!(address = %config.sip_listen_addr, "SIP Sunucusu başlatıldı");
    
    let mut buf = [0; 65535];

    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        let sock_clone = Arc::clone(&sock);
        let config_clone = Arc::clone(&config);
        let rabbit_channel_clone = Arc::clone(&rabbit_channel);
        let request_bytes = buf[..len].to_vec();
        
        tokio::spawn(async move {
            if let Err(e) = handle_sip_request(&request_bytes, sock_clone, addr, config_clone, rabbit_channel_clone).await {
                error!(error = %e, "SIP isteği işlenirken hata oluştu");
            }
        });
    }
}

//--- SIP İsteği İşleyici (DÜZELTİLMİŞ) ---
#[instrument(skip_all, fields(remote_addr = %addr))]
async fn handle_sip_request(
    request_bytes: &[u8],
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request_str = std::str::from_utf8(request_bytes)?;

    if !request_str.starts_with("INVITE") {
        return Ok(());
    }

    debug!("Gelen INVITE:\n{}", request_str);
    
    if let Some(mut headers) = parse_complex_headers(request_str) {
        let from_uri = headers.get("From").cloned().unwrap_or_default();
        let to_uri = headers.get("To").cloned().unwrap_or_default();
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        
        // DÜZELTME: Artık fonksiyon String döndürdüğü için to_string() gerekmiyor.
        let user_id_to_check = extract_user_from_uri(&from_uri).unwrap_or_default();

        sock.send_to(create_response("100 Trying", &headers, None, &config).as_bytes(), addr).await?;

        // Adım 1: Kullanıcıyı Doğrula (GetUser RPC'sini kullanarak)
        let mut user_client = UserServiceClient::connect(config.user_service_url.clone()).await?;
        let user_req = GetUserRequest { id: user_id_to_check.to_string() };
        let user_res = user_client.get_user(user_req).await?.into_inner();
        
        // user_res.user boş değilse, kullanıcı var demektir.
        if user_res.user.is_none() {
            error!(user_id = %user_id_to_check, "Kullanıcı doğrulaması başarısız oldu. Çağrı reddediliyor.");
            sock.send_to(create_response("403 Forbidden", &headers, None, &config).as_bytes(), addr).await?;
            return Ok(());
        }
        let found_user = user_res.user.unwrap();
        info!(user_id = %found_user.id, "Kullanıcı doğrulandı.");

        // Adım 2: Yönlendirme Planını Al
        let mut dialplan_client = DialplanServiceClient::connect(config.dialplan_service_url.clone()).await?;
        let dialplan_req = GetDialplanForUserRequest { user_id: found_user.id.clone() };
        let dialplan_res = dialplan_client.get_dialplan_for_user(dialplan_req).await?.into_inner();
        
        if dialplan_res.dialplan_id.is_empty() {
            error!("Yönlendirme planı bulunamadı. Çağrı reddediliyor.");
            sock.send_to(create_response("404 Not Found", &headers, None, &config).as_bytes(), addr).await?;
            return Ok(());
        }
        info!(dialplan_id = %dialplan_res.dialplan_id, "Yönlendirme planı alındı.");

        // Adım 3: Medya Portu Ayır
        let mut media_client = MediaServiceClient::<Channel>::connect(config.media_service_url.clone()).await?;
        let media_req = AllocatePortRequest { call_id: call_id.clone() };
        let media_res = media_client.allocate_port(media_req).await?.into_inner();
        let rtp_port = media_res.rtp_port;
        info!(rtp_port = rtp_port, "Medya portu ayrıldı.");
        
        // Adım 4 & 5... (Geri kalan kod aynı)
        let to_header = headers.get("To").cloned().unwrap_or_default();
        let to_tag = format!(";tag={}", rand::thread_rng().gen::<u32>());
        headers.insert("To".to_string(), format!("{}{}", to_header, to_tag));

        sock.send_to(create_response("180 Ringing", &headers, None, &config).as_bytes(), addr).await?;
        sleep(Duration::from_millis(100)).await;

        let sdp_body = format!(
            "v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n",
            rand::thread_rng().gen::<u32>(), config.sip_public_ip, rtp_port
        );
        let ok_response = create_response("200 OK", &headers, Some(&sdp_body), &config);
        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!(port = rtp_port, "Arama başarıyla cevaplandı!");
        
        let event_payload = json!({
            "eventType": "call.started",
            "callId": call_id,
            "from": from_uri,
            "to": to_uri,
            "media": { "host": config.sip_public_ip, "port": rtp_port },
            "timestamp": Utc::now().to_rfc3339(),
        });

        let confirmation = rabbit_channel.basic_publish(
            "",
            RABBITMQ_QUEUE_NAME,
            BasicPublishOptions::default(),
            event_payload.to_string().as_bytes(),
            BasicProperties::default().with_delivery_mode(2),
        ).await?;
        
        confirmation.await?;
        info!("'call.started' olayı RabbitMQ'ya başarıyla yayınlandı ve onaylandı.");
    }
    Ok(())
}

//--- Yardımcı Fonksiyonlar (create_response ve parse_complex_headers aynı) ---

// YENİ ve DAHA SAĞLAM Yardımcı Fonksiyon
fn extract_user_from_uri(uri: &str) -> Option<String> {
    // Örnek: "Azmi Sahin" <sip:+905548777858@127.0.0.1>;tag=...
    // Bu string içinden sadece rakamları alıp birleştirecek.
    // Sonuç: "905548777858"
    
    // SIP URI'sinin başlangıcını bul
    if let Some(start_index) = uri.find("sip:") {
        // 'sip:' sonrasından başla
        let relevant_part = &uri[start_index..];
        // Sadece rakam karakterlerini topla
        let numbers: String = relevant_part.chars().filter(|c| c.is_digit(10)).collect();
        if !numbers.is_empty() {
            return Some(numbers);
        }
    }
    None
}


fn parse_complex_headers(request: &str) -> Option<HashMap<String, String>> {
    let mut headers = HashMap::new();
    let mut via_headers = Vec::new();
    let mut record_route_headers = Vec::new();
    for line in request.lines() {
        if line.is_empty() { break; }
        if let Some((key, value)) = line.split_once(':') {
            let key_trimmed = key.trim();
            let value_trimmed = value.trim().to_string();
            match key_trimmed.to_lowercase().as_str() {
                "via" | "v" => via_headers.push(value_trimmed),
                "record-route" => record_route_headers.push(value_trimmed),
                _ => { headers.insert(key_trimmed.to_string(), value_trimmed); }
            }
        }
    }
    if !via_headers.is_empty() {
        headers.insert("Via".to_string(), via_headers.join(","));
        if !record_route_headers.is_empty() {
            headers.insert("Record-Route".to_string(), record_route_headers.join(","));
        }
        Some(headers)
    } else { None }
}

fn create_response(status_line: &str, headers: &HashMap<String, String>, sdp: Option<&str>, config: &AppConfig) -> String {
    let body = sdp.unwrap_or("");
    let content_length = body.len();
    let record_route_line = match headers.get("Record-Route") {
        Some(routes) => format!("Record-Route: {}\r\n", routes),
        None => String::new(),
    };
    format!(
        "SIP/2.0 {}\r\nVia: {}\r\n{}From: {}\r\nTo: {}\r\nCall-ID: {}\r\nCSeq: {}\r\nContact: <sip:signal@{}:{}>\r\nContent-Type: application/sdp\r\nContent-Length: {}\r\n\r\n{}",
        status_line,
        headers.get("Via").unwrap_or(&String::new()),
        record_route_line,
        headers.get("From").unwrap_or(&String::new()),
        headers.get("To").unwrap_or(&String::new()),
        headers.get("Call-ID").unwrap_or(&String::new()),
        headers.get("CSeq").unwrap_or(&String::new()),
        config.sip_public_ip,
        config.sip_listen_addr.port(),
        content_length,
        body
    )
}