// DOSYA: sentiric-sip-signaling-service/src/main.rs (SON DÜZELTİLMİŞ VERSİYON)

use std::collections::{HashMap, HashSet};
use std::env;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use rand::Rng;
use tracing::{info, error, debug, instrument, warn};
use tracing_subscriber::EnvFilter;
use tonic::transport::Channel;
use lapin::{Connection, ConnectionProperties, options::*, types::FieldTable, BasicProperties, Channel as LapinChannel};
use chrono::Utc;
use serde_json::json;
use regex::Regex;
use once_cell::sync::Lazy;

use sentiric_contracts::sentiric::{
    media::v1::{media_service_client::MediaServiceClient, AllocatePortRequest, ReleasePortRequest},
    user::v1::{user_service_client::UserServiceClient, GetUserRequest},
    dialplan::v1::{dialplan_service_client::DialplanServiceClient, GetDialplanForUserRequest},
};

static USER_EXTRACT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"sip:\+?([0-9]+)@").unwrap());
const RABBITMQ_QUEUE_NAME: &str = "call.events";

type ActiveTransactions = Arc<Mutex<HashSet<String>>>;
type ActiveCalls = Arc<Mutex<HashMap<String, u32>>>;

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
        dotenv::dotenv().ok();
        let sip_host = env::var("SIP_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let sip_port_str = env::var("INTERNAL_SIP_SIGNALING_PORT").unwrap_or_else(|_| "5060".to_string());
        let sip_port = sip_port_str.parse::<u16>()?;
        let sip_listen_addr = format!("{}:{}", sip_host, sip_port).parse()?;
        
        Ok(AppConfig {
            sip_listen_addr,
            sip_public_ip: env::var("PUBLIC_IP")?,
            user_service_url: env::var("USER_SERVICE_GRPC_URL")?,
            dialplan_service_url: env::var("DIALPLAN_SERVICE_GRPC_URL")?,
            media_service_url: env::var("MEDIA_SERVICE_GRPC_URL")?,
            rabbitmq_url: env::var("RABBITMQ_URL")?,
        })
    }
}

async fn connect_to_rabbitmq_with_retry(url: &str) -> Arc<LapinChannel> {
    let max_retries = 10;
    for i in 0..max_retries {
        if let Ok(conn) = Connection::connect(url, ConnectionProperties::default()).await {
            if let Ok(channel) = conn.create_channel().await {
                info!("RabbitMQ bağlantısı başarıyla kuruldu.");
                return Arc::new(channel);
            }
        }
        warn!(attempt = i + 1, max_attempts = max_retries, "RabbitMQ'ya bağlanılamadı. 5sn sonra tekrar denenecek...");
        sleep(Duration::from_secs(5)).await;
    }
    panic!("Maksimum deneme sayısına ulaşıldı, RabbitMQ'ya bağlanılamadı.");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().json().with_env_filter(env_filter).init();

    let config = match AppConfig::load_from_env() {
        Ok(cfg) => Arc::new(cfg),
        Err(e) => {
            error!(error = %e, "Konfigürasyon yüklenemedi.");
            return Err(e);
        }
    };
    info!("Konfigürasyon yüklendi.");
    
    let active_transactions: ActiveTransactions = Arc::new(Mutex::new(HashSet::new()));
    let active_calls: ActiveCalls = Arc::new(Mutex::new(HashMap::new()));

    let rabbit_channel = connect_to_rabbitmq_with_retry(&config.rabbitmq_url).await;
    rabbit_channel.queue_declare(RABBITMQ_QUEUE_NAME, QueueDeclareOptions { durable: true, ..Default::default() }, FieldTable::default()).await?;
    info!("'{}' kuyruğu deklare edildi.", RABBITMQ_QUEUE_NAME);

    let sock = Arc::new(UdpSocket::bind(config.sip_listen_addr).await?);
    info!(address = %config.sip_listen_addr, "SIP Signaling başlatıldı.");
    
    let mut buf = [0; 65535];
    loop {
        let (len, addr) = sock.recv_from(&mut buf).await?;
        let sock_clone = Arc::clone(&sock);
        let config_clone = Arc::clone(&config);
        let rabbit_channel_clone = Arc::clone(&rabbit_channel);
        let request_bytes = buf[..len].to_vec();
        
        let transactions_clone = Arc::clone(&active_transactions);
        let active_calls_clone = Arc::clone(&active_calls);
        
        tokio::spawn(async move {
            if let Err(e) = handle_sip_request(
                &request_bytes, 
                sock_clone, 
                addr, 
                config_clone, 
                rabbit_channel_clone,
                transactions_clone,
                active_calls_clone
            ).await {
                error!(error = %e, remote_addr = %addr, "SIP isteği işlenirken akış tamamlanamadı.");
            }
        });
    }
}

#[instrument(skip_all, fields(remote_addr = %addr, call_id))]
async fn handle_sip_request(
    request_bytes: &[u8],
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_transactions: ActiveTransactions,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let request_str = std::str::from_utf8(request_bytes)?;
    
    if request_str.starts_with("INVITE") {
        handle_invite(request_str, sock, addr, config, rabbit_channel, active_transactions, active_calls).await
    } else if request_str.starts_with("BYE") {
        handle_bye(request_str, sock, addr, config, rabbit_channel, active_calls).await
    } else if request_str.starts_with("ACK") {
        if let Some(headers) = parse_complex_headers(request_str) {
            let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
            tracing::Span::current().record("call_id", &call_id.as_str());
            info!("ACK isteği alındı, SIP diyaloğu başarıyla kuruldu.");
        }
        Ok(())
    }
    else {
        Ok(())
    }
}

async fn handle_bye(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    _rabbit_channel: Arc<LapinChannel>,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        tracing::Span::current().record("call_id", &call_id.as_str());
        info!("BYE isteği alındı.");

        let rtp_port_to_release = {
            let mut calls_guard = active_calls.lock().await;
            calls_guard.remove(&call_id)
        };

        if let Some(rtp_port) = rtp_port_to_release {
            info!(port = rtp_port, "Çağrı sonlandırılıyor, RTP portu serbest bırakılacak.");
            let mut media_client = MediaServiceClient::<Channel>::connect(config.media_service_url.clone()).await?;
            let release_req = ReleasePortRequest { rtp_port };
            match media_client.release_port(release_req).await {
                Ok(_) => info!(port = rtp_port, "Media service'den port serbest bırakma onayı alındı."),
                Err(e) => error!(error = %e, port = rtp_port, "Media service'e port serbest bırakma isteği gönderilirken hata oluştu."),
            }
        } else {
            warn!("BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı. Port serbest bırakılamadı.");
        }

        let ok_response = create_response("200 OK", &headers, None, &config);
        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!("BYE isteğine 200 OK yanıtı gönderildi.");
    }
    Ok(())
}

// DOSYA: sentiric-sip-signaling-service/src/main.rs (SADECE handle_invite fonksiyonunu değiştirin)

async fn handle_invite(
    request_str: &str,
    sock: Arc<UdpSocket>,
    addr: SocketAddr,
    config: Arc<AppConfig>,
    rabbit_channel: Arc<LapinChannel>,
    active_transactions: ActiveTransactions,
    active_calls: ActiveCalls,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(mut headers) = parse_complex_headers(request_str) {
        let call_id = headers.get("Call-ID").cloned().unwrap_or_default();
        if call_id.is_empty() {
            warn!("Call-ID bulunamayan INVITE paketi atlandı.");
            return Ok(());
        }
        tracing::Span::current().record("call_id", &call_id.as_str());
        
        // --- KRİTİK DEĞİŞİKLİK: Yinelenen isteği en başta kontrol et ve hemen çık! ---
        if active_calls.lock().await.contains_key(&call_id) || active_transactions.lock().await.contains(&call_id) {
             warn!(call_id = %call_id, "Yinelenen veya hala aktif olan bir çağrı için INVITE isteği alındı ve atlandı.");
             // İstemciye meşgul olduğunu bildirebiliriz, bu timeout'u önleyebilir.
             sock.send_to(create_response("486 Busy Here", &headers, None, &config).as_bytes(), addr).await?;
             return Ok(());
        }
        
        // --- Geliştirilmiş Transaction Yönetimi ---
        // Bu noktadan sonra bu çağrıyı işlemeye başlıyoruz, hafızaya ekleyelim.
        active_transactions.lock().await.insert(call_id.clone());
        
        debug!("Yeni INVITE işleniyor...");

        // Fonksiyon bittiğinde transaction'ı temizlemek için bir guard oluşturalım.
        struct TransactionGuard {
            call_id: String,
            transactions: ActiveTransactions,
        }
        impl Drop for TransactionGuard {
            fn drop(&mut self) {
                let transactions = self.transactions.clone();
                let call_id = self.call_id.clone();
                tokio::spawn(async move {
                    debug!(call_id = %call_id, "Transaction tamamlandı, geçici listeden siliniyor.");
                    transactions.lock().await.remove(&call_id);
                });
            }
        }
        let _guard = TransactionGuard {
            call_id: call_id.clone(),
            transactions: active_transactions.clone(),
        };


        let from_uri = headers.get("From").cloned().unwrap_or_default();
        let to_uri = headers.get("To").cloned().unwrap_or_default();
        let user_id_to_check = extract_user_from_uri(&from_uri).ok_or("From header'ından kullanıcı ID'si ayııklanamadı.")?;
        
        sock.send_to(create_response("100 Trying", &headers, None, &config).as_bytes(), addr).await?;
        
        let mut user_client = UserServiceClient::connect(config.user_service_url.clone()).await?;
        let user_res = user_client.get_user(GetUserRequest { id: user_id_to_check.clone() }).await?.into_inner();
        
        if user_res.user.is_none() {
            sock.send_to(create_response("404 Not Found", &headers, None, &config).as_bytes(), addr).await?;
            return Err(format!("Kullanıcı bulunamadı: {}", user_id_to_check).into());
        }
        let found_user = user_res.user.unwrap();
        info!(user_id = %found_user.id, "Kullanıcı doğrulandı.");
        
        let mut dialplan_client = DialplanServiceClient::connect(config.dialplan_service_url.clone()).await?;
        let dialplan_res = dialplan_client.get_dialplan_for_user(GetDialplanForUserRequest { user_id: found_user.id.clone() }).await?.into_inner();
        
        if dialplan_res.dialplan_id.is_empty() {
            sock.send_to(create_response("404 Not Found", &headers, None, &config).as_bytes(), addr).await?;
            return Err(format!("Yönlendirme planı bulunamadı: {}", found_user.id).into());
        }
        info!(dialplan_id = %dialplan_res.dialplan_id, "Yönlendirme planı alındı.");
        
        let mut media_client = MediaServiceClient::<Channel>::connect(config.media_service_url.clone()).await?;
        let media_res = media_client.allocate_port(AllocatePortRequest { call_id: call_id.clone() }).await?.into_inner();
        let server_rtp_port = media_res.rtp_port;
        info!(rtp_port = server_rtp_port, "Medya portu ayrıldı.");
        
        let to_header = headers.get("To").cloned().unwrap_or_default();
        let to_tag = format!(";tag={}", rand::thread_rng().gen::<u32>());
        headers.insert("To".to_string(), format!("{}{}", to_header, to_tag));
        sock.send_to(create_response("180 Ringing", &headers, None, &config).as_bytes(), addr).await?;
        sleep(Duration::from_millis(100)).await;

        let sdp_body = format!("v=0\r\no=- {0} {0} IN IP4 {1}\r\ns=Sentiric\r\nc=IN IP4 {1}\r\nt=0 0\r\nm=audio {2} RTP/AVP 0\r\na=rtpmap:0 PCMU/8000\r\n", rand::thread_rng().gen::<u32>(), config.sip_public_ip, server_rtp_port);
        let ok_response = create_response("200 OK", &headers, Some(&sdp_body), &config);
        
        // --- DEĞİŞİKLİK: Sadece 200 OK göndermeden hemen önce kalıcı listeye ekle ---
        {
            let mut calls_guard = active_calls.lock().await;
            calls_guard.insert(call_id.clone(), server_rtp_port);
            info!(call_id = %call_id, port = server_rtp_port, "Yeni aktif çağrı haritaya eklendi.");
        }

        sock.send_to(ok_response.as_bytes(), addr).await?;
        info!(port = server_rtp_port, "Arama başarıyla cevaplandı!");
        
        let caller_rtp_addr = extract_sdp_media_info(request_str).ok_or("INVITE'ın SDP bölümünden medya bilgisi alınamadı.")?;
        info!(target_addr = %caller_rtp_addr, "Arayan tarafın RTP adresi SDP'den okundu.");

        let event_payload = json!({
            "eventType": "call.started",
            "callId": call_id,
            "from": from_uri,
            "to": to_uri,
            "media": {
                "server_rtp_port": server_rtp_port,
                "caller_rtp_addr": caller_rtp_addr
            },
            "timestamp": Utc::now().to_rfc3339(),
        });
        rabbit_channel.basic_publish("", RABBITMQ_QUEUE_NAME, BasicPublishOptions::default(), event_payload.to_string().as_bytes(), BasicProperties::default().with_delivery_mode(2)).await?.await?;
        info!("'call.started' olayı RabbitMQ'ya başarıyla yayınlandı.");
    }
    Ok(())
}

fn extract_user_from_uri(uri: &str) -> Option<String> {
    USER_EXTRACT_RE.captures(uri).and_then(|caps| caps.get(1)).map(|user_part| user_part.as_str().to_string())
}

// DOSYA: sentiric-sip-signaling-service/src/main.rs (SADECE create_response fonksiyonunu güncelleyin)

fn create_response(status_line: &str, headers: &HashMap<String, String>, sdp: Option<&str>, config: &AppConfig) -> String {
    let body = sdp.unwrap_or("");
    let content_length = body.len();
    
    let record_route_lines = headers.get("Record-Route").map_or(String::new(), |routes| {
        routes.split(", ").map(|route| format!("Record-Route: {}\r\n", route)).collect::<String>()
    });

    let via_lines = headers.get("Via").map_or(String::new(), |vias| {
        vias.split(", ").map(|via| format!("Via: {}\r\n", via)).collect::<String>()
    });

    let empty_string = String::new();
    let cseq_full = headers.get("CSeq").unwrap_or(&empty_string);
    
    // --- KRİTİK DEĞİŞİKLİK: Contact başlığında PUBLIC_IP kullanıyoruz! ---
    // Bu, ACK ve BYE mesajlarının bize doğru şekilde geri dönmesini sağlar.
    let contact_header = format!("<sip:{}@{}:{}>", 
        "sentiric-signal", // Kullanıcı adı önemli değil, genellikle servis adı kullanılır
        config.sip_public_ip, 
        config.sip_listen_addr.port()
    );

    format!(
        "SIP/2.0 {}\r\n\
         {}\
         {}\
         From: {}\r\n\
         To: {}\r\n\
         Call-ID: {}\r\n\
         CSeq: {}\r\n\
         Contact: {}\r\n\
         Server: Sentiric Signaling Service\r\n\
         Content-Type: application/sdp\r\n\
         Content-Length: {}\r\n\
         \r\n\
         {}",
        status_line,
        via_lines,
        record_route_lines,
        headers.get("From").unwrap_or(&empty_string),
        headers.get("To").unwrap_or(&empty_string),
        headers.get("Call-ID").unwrap_or(&empty_string),
        cseq_full,
        contact_header, // Güncellenmiş Contact başlığını buraya koyuyoruz
        content_length,
        body
    )
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
        headers.insert("Via".to_string(), via_headers.join(", "));
        if !record_route_headers.is_empty() {
            headers.insert("Record-Route".to_string(), record_route_headers.join(", "));
        }
        Some(headers)
    } else {
        warn!("Gelen SIP isteğinde Via başlığı bulunamadı.");
        None
    }
}

fn extract_sdp_media_info(sip_request: &str) -> Option<String> {
    let mut ip_addr: Option<&str> = None;
    let mut port: Option<&str> = None;
    if let Some(sdp_part) = sip_request.split("\r\n\r\n").nth(1) {
        for line in sdp_part.lines() {
            if line.starts_with("c=IN IP4 ") {
                ip_addr = line.split_whitespace().nth(2);
            }
            if line.starts_with("m=audio ") {
                port = line.split_whitespace().nth(1);
            }
        }
    }
    if let (Some(ip), Some(p)) = (ip_addr, port) {
        Some(format!("{}:{}", ip, p))
    } else {
        None
    }
}