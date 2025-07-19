const dgram = require('dgram');
const axios = require('axios');
const amqp = require('amqplib'); // RabbitMQ istemcisini ekliyoruz

// Ortam değişkenlerinden servis URL'lerini ve ayarları alıyoruz.
const USER_SERVICE_URL = process.env.USER_SERVICE_URL;
const DIALPLAN_SERVICE_URL = process.env.DIALPLAN_SERVICE_URL;
const MEDIA_SERVICE_URL = process.env.MEDIA_SERVICE_URL;
const RABBITMQ_URL = process.env.RABBITMQ_URL;

const SIP_PORT = process.env.SIP_PORT || 5060;
const SIP_HOST = '0.0.0.0';

const QUEUE_NAME = 'call.events';
let rabbitChannel = null; // RabbitMQ kanalı için global bir değişken

const server = dgram.createSocket('udp4');

// --- RabbitMQ Bağlantı Fonksiyonu ---
async function connectRabbitMQ() {
    try {
        const connection = await amqp.connect(RABBITMQ_URL);
        const channel = await connection.createChannel();
        // Kuyruğun kalıcı (durable) olduğundan emin oluyoruz.
        await channel.assertQueue(QUEUE_NAME, { durable: true });
        console.log("✅ [SIP Signaling] RabbitMQ'ya başarıyla bağlandı.");
        rabbitChannel = channel;
    } catch (error) {
        console.error("❌ RabbitMQ bağlantı hatası:", error.message);
        console.log("5 saniye sonra tekrar denenecek...");
        setTimeout(connectRabbitMQ, 5000); // Hata durumunda yeniden bağlanmayı dene
    }
}

// --- Yardımcı Fonksiyonlar ---
// SIP mesajından belirli bir başlığı (header) ayıklayan fonksiyon
const getHeader = (message, name) => {
  const match = message.match(new RegExp(`^${name}:\\s*(.*)$`, 'im'));
  return match ? match[1].trim() : '';
};

// SIP URI'sinden kullanıcı adını (örn: <sip:1001@...>) veya numarayı çıkaran fonksiyon
const getPrincipalFromUri = (uri) => {
  const match = uri.match(/sip:([^@;]+)/);
  return match ? match[1] : null;
};

// --- Ana Mesaj İşleyici ---
server.on('message', async (msg, rinfo) => {
  const message = msg.toString();
  console.log(`\n--- Yeni SIP Mesajı Alındı: ${rinfo.address}:${rinfo.port} ---`);

  // Sadece INVITE (arama başlatma) isteklerini işliyoruz. Diğerlerini (REGISTER vb.) şimdilik göz ardı ediyoruz.
  if (!message.startsWith('INVITE')) {
    console.log("-> Mesaj INVITE değil, şimdilik göz ardı ediliyor.");
    return;
  }

  console.log('📞 Gelen arama (INVITE) tespit edildi. İşlem başlatılıyor...');
  
  // Gelen isteğin başlıklarından arayan ve aranan bilgilerini alıyoruz.
  const fromUri = getHeader(message, 'From');
  const toUri = getHeader(message, 'To');
  
  const fromUser = getPrincipalFromUri(fromUri);   // Arayan kullanıcı/numara
  const toDestination = getPrincipalFromUri(toUri); // Aranan numara
  const callId = getHeader(message, 'Call-ID');    // Her çağrı için benzersiz ID

  // 1. Adım: User Service'e danışarak arayanın geçerli bir kullanıcı olup olmadığını kontrol et.
  try {
    console.log(`[1/5] 👤 User Service'e soruluyor: Kullanıcı '${fromUser}' geçerli mi?`);
    const userResponse = await axios.get(`${USER_SERVICE_URL}/users/${fromUser}`);
    
    if (userResponse.status === 200) {
      console.log(`--> ✅ Kullanıcı '${fromUser}' bulundu. (${userResponse.status})`);
    }
  } catch (error) {
    if (error.response && error.response.status === 404) {
      console.error(`--> ❌ Kullanıcı '${fromUser}' bulunamadı. Çağrı reddediliyor.`);
    } else {
      console.error(`--> ❌ User Service'e ulaşılamadı veya bir hata oluştu: ${error.message}`);
    }
    // Tanınmayan kullanıcı veya servis hatası durumunda işlemi sonlandır.
    return; 
  }

  // 2. Adım: Dialplan Service'e danışarak bu arama için ne yapılması gerektiğini sor.
  try {
    console.log(`[2/5] 🗺️  Dialplan Service'e soruluyor: Hedef '${toDestination}' için plan nedir?`);
    const dialplanResponse = await axios.get(`${DIALPLAN_SERVICE_URL}/dialplan/${toDestination}`);

    if (dialplanResponse.status === 200) {
      console.log(`--> ✅ Yönlendirme planı bulundu. (${dialplanResponse.status})`);
      console.log('--> Alınan Plan:', dialplanResponse.data);
    }
  } catch (error) {
    if (error.response && error.response.status === 404) {
        console.error(`--> ❌ Hedef '${toDestination}' için yönlendirme planı bulunamadı. Çağrı reddediliyor.`);
    } else {
        console.error(`--> ❌ Dialplan Service'e ulaşılamadı veya bir hata oluştu: ${error.message}`);
    }
    // Yönlendirme kuralı yoksa veya servis hatası varsa işlemi sonlandır.
    return;
  }
  
  // 3. Adım: Media Service'den bir RTP oturumu (port) talep et.
  let mediaInfo = null;
  try {
    console.log(`[3/5] 🔊 Media Service'e soruluyor: Yeni bir RTP oturumu aç.`);
    const mediaResponse = await axios.get(`${MEDIA_SERVICE_URL}/rtp-session`);
    
    if (mediaResponse.status === 200) {
      mediaInfo = mediaResponse.data;
      console.log(`--> ✅ Medya oturumu oluşturuldu. Host: ${mediaInfo.host}, Port: ${mediaInfo.port}`);
    }
  } catch (error) {
    console.error(`--> ❌ Medya oturumu oluşturulamadı veya Media Service'e ulaşılamadı: ${error.message}`);
    return; // Medya kanalı olmadan devam edemeyiz.
  }

  // 4. Adım: Çağrıyı, medya bilgileriyle birlikte kabul et (200 OK).
  console.log(`[4/5] ✅ Tüm kontroller başarılı. Medya bilgileriyle çağrı kabul ediliyor.`);
  const via = getHeader(message, 'Via');
  const from = getHeader(message, 'From');
  const to = getHeader(message, 'To');
  const cseq = getHeader(message, 'CSeq');
  
  // SDP (Session Description Protocol) oluşturuyoruz.
  // Karşı tarafa "sesi bu adrese ve porta gönder" demenin standart yoludur.
  // NAT sorunlarını aşmak için, Docker'ın iç IP'si yerine sunucunun genel IP'sini kullanıyoruz.
  const publicIp = process.env.PUBLIC_IP || mediaInfo.host;

  const sdpBody = [
    'v=0',
    `o=- 0 0 IN IP4 ${publicIp}`,
    's=-',
    `c=IN IP4 ${publicIp}`,
    't=0 0',
    `m=audio ${mediaInfo.port} RTP/AVP 0 8 101`,
    'a=rtpmap:0 PCMU/8000',
    'a=rtpmap:8 PCMA/8000',
    'a=rtpmap:101 telephone-event/8000',
    'a=sendrecv'
  ].join('\r\n');

  const response = [
    'SIP/2.0 200 OK', `Via: ${via}`, `From: ${from}`, `To: ${to}`,
    `Call-ID: ${callId}`, `CSeq: ${cseq}`, 'Content-Type: application/sdp',
    `Content-Length: ${sdpBody.length}`, '', sdpBody
  ].join('\r\n');

  server.send(Buffer.from(response), rinfo.port, rinfo.address, (err) => {
    if (err) {
      console.error('❌ Yanıt gönderilirken hata oluştu:', err);
    } else {
      console.log(`--> ✅ 200 OK yanıtı ${rinfo.address}:${rinfo.port} adresine gönderildi.`);
      
      // 5. Adım: Başarılı çağrı olayını RabbitMQ'ya yayınla
      if (rabbitChannel) {
        const event = {
          eventType: 'call.started',
          callId: callId,
          from: fromUser,
          to: toDestination,
          media: mediaInfo,
          timestamp: new Date().toISOString()
        };
        const eventString = JSON.stringify(event);
        // Mesajların kalıcı (persistent) olması, RabbitMQ yeniden başlasa bile kaybolmamasını sağlar.
        rabbitChannel.sendToQueue(QUEUE_NAME, Buffer.from(eventString), { persistent: true });
        console.log(`--> 🐇 [5/5] Olay '${QUEUE_NAME}' kuyruğuna yayınlandı.`);
      } else {
        console.error("❌ RabbitMQ kanalı aktif değil. Olay yayınlanamadı.");
      }
    }
  });
});

// Sunucu dinlemeye hazır olduğunda bu fonksiyon çalışır.
server.on('listening', () => {
  const address = server.address();
  console.log(`✅ [SIP Signaling] Servis UDP ${address.address}:${address.port} portunda dinlemede.`);
});

// Sunucu bir hata ile karşılaştığında bu fonksiyon çalışır.
server.on('error', (err) => {
  console.error(`❌ Sunucu hatası:\n${err.stack}`);
  server.close();
});

// Sunucuyu başlat ve RabbitMQ'ya bağlan
server.bind(SIP_PORT, SIP_HOST);
connectRabbitMQ();