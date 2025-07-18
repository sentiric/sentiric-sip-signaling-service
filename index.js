const dgram = require('dgram');
const axios = require('axios');

// Ortam değişkenlerinden servis URL'lerini al
const USER_SERVICE_URL = process.env.USER_SERVICE_URL || 'http://user-service:3001';
const DIALPLAN_SERVICE_URL = process.env.DIALPLAN_SERVICE_URL || 'http://dialplan-service:3002';

const SIP_PORT = process.env.SIP_PORT || 5060;
const SIP_HOST = '0.0.0.0';

const server = dgram.createSocket('udp4');

// --- Yardımcı Fonksiyonlar ---
// SIP mesajından belirli bir başlığı (header) ayıklayan fonksiyon
const getHeader = (message, name) => {
  const match = message.match(new RegExp(`^${name}:\\s*(.*)$`, 'im'));
  return match ? match[1].trim() : '';
};

// SIP URI'sinden kullanıcı adını (örn: <sip:1001@...>) çıkaran fonksiyon
const getUsernameFromUri = (uri) => {
  const match = uri.match(/sip:([^@;]+)/);
  return match ? match[1] : null;
}

// --- Ana Mesaj İşleyici ---
server.on('message', async (msg, rinfo) => {
  const message = msg.toString();
  console.log(`\n--- Yeni SIP Mesajı Alındı: ${rinfo.address}:${rinfo.port} ---`);

  // Sadece INVITE isteklerini işliyoruz
  if (!message.startsWith('INVITE')) {
    console.log("-> Mesaj INVITE değil, şimdilik göz ardı ediliyor.");
    return;
  }

  console.log('📞 Gelen arama (INVITE) tespit edildi. İşlem başlatılıyor...');
  
  // Gelen isteğin temel başlıklarını alıyoruz
  const fromUri = getHeader(message, 'From');
  const toUri = getHeader(message, 'To');
  
  const fromUser = getUsernameFromUri(fromUri);
  const toUser = getUsernameFromUri(toUri);

  // 1. Adım: User Service'e danışarak arayanın geçerli bir kullanıcı olup olmadığını kontrol et
  try {
    console.log(`[1/3] 👤 User Service'e soruluyor: Kullanıcı '${fromUser}' geçerli mi?`);
    const userResponse = await axios.get(`${USER_SERVICE_URL}/users/${fromUser}`);
    
    if (userResponse.status === 200) {
      console.log(`--> ✅ Kullanıcı '${fromUser}' bulundu. (${userResponse.status})`);
    }
    // Not: Gerçek bir sistemde burada kimlik doğrulaması (digest auth) yapılır.
    // Şimdilik sadece kullanıcının varlığını kontrol ediyoruz.

  } catch (error) {
    console.error(`--> ❌ Kullanıcı '${fromUser}' bulunamadı veya User Service'e ulaşılamadı.`);
    // Burada 401 Unauthorized veya 404 Not Found gibi bir SIP hatası dönebiliriz.
    // Şimdilik devam ediyoruz.
    return;
  }

  // 2. Adım: Dialplan Service'e danışarak bu arama için ne yapılması gerektiğini sor
  try {
    console.log(`[2/3] 🗺️  Dialplan Service'e soruluyor: Hedef '${toUser}' için plan nedir?`);
    const dialplanResponse = await axios.get(`${DIALPLAN_SERVICE_URL}/dialplan/${toUser}`);

    if (dialplanResponse.status === 200) {
      console.log(`--> ✅ Yönlendirme planı bulundu. (${dialplanResponse.status})`);
      console.log('--> Alınan Plan:', dialplanResponse.data);
    }
  } catch (error) {
    console.error(`--> ❌ Hedef '${toUser}' için yönlendirme planı bulunamadı veya Dialplan Service'e ulaşılamadı.`);
    // Burada 404 Not Found SIP hatası dönebiliriz.
    return;
  }

  // 3. Adım: Tüm kontroller başarılıysa, çağrıyı kabul et (200 OK)
  console.log(`[3/3] ✅ Tüm kontroller başarılı. Çağrı kabul ediliyor.`);
  const via = getHeader(message, 'Via');
  const from = getHeader(message, 'From');
  const to = getHeader(message, 'To');
  const callId = getHeader(message, 'Call-ID');
  const cseq = getHeader(message, 'CSeq');

  const response = [
    'SIP/2.0 200 OK', `Via: ${via}`, `From: ${from}`, `To: ${to}`,
    `Call-ID: ${callId}`, `CSeq: ${cseq}`, 'Content-Length: 0', '\r\n'
  ].join('\r\n');

  server.send(Buffer.from(response), rinfo.port, rinfo.address, (err) => {
    if (err) {
      console.error('❌ Yanıt gönderilirken hata oluştu:', err);
    } else {
      console.log(`--> ✅ 200 OK yanıtı ${rinfo.address}:${rinfo.port} adresine gönderildi.`);
    }
  });
});

server.on('listening', () => {
  const address = server.address();
  console.log(`✅ [SIP Signaling] Servis UDP ${address.address}:${address.port} portunda dinlemede.`);
});

server.on('error', (err) => {
  console.error(`❌ Sunucu hatası:\n${err.stack}`);
  server.close();
});

server.bind(SIP_PORT, SIP_HOST);