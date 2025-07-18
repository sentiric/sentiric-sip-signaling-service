// Gerekli olan UDP modülünü (dgram) yüklüyoruz.
const dgram = require('dgram');

// Sunucu ayarlarını ortam değişkenlerinden alıyoruz, yoksa varsayılanı kullanıyoruz.
const SIP_PORT = process.env.SIP_PORT || 5060;
const SIP_HOST = '0.0.0.0'; // Konteyner içinde tüm ağ arayüzlerini dinle

// Bir UDP4 (IPv4 için) sunucusu oluşturuyoruz.
const server = dgram.createSocket('udp4');

// Sunucu dinlemeye hazır olduğunda bu fonksiyon çalışır.
server.on('listening', () => {
  const address = server.address();
  console.log(`✅ [SIP Signaling] Servis UDP ${address.address}:${address.port} portunda dinlemede.`);
});

// Bir UDP paketi (SIP mesajı) geldiğinde bu fonksiyon çalışır.
server.on('message', (msg, rinfo) => {
  const message = msg.toString();
  console.log(`\n--- Yeni SIP Mesajı Alındı: ${rinfo.address}:${rinfo.port} ---`);
  console.log(message);
  console.log('----------------------------------------------------');

  // Gelen mesaj bir INVITE (arama başlatma) isteği mi diye kontrol ediyoruz.
  if (message.startsWith('INVITE')) {
    console.log('📞 Gelen arama (INVITE) tespit edildi. 200 OK yanıtı hazırlanıyor...');
    
    // Gelen isteğin temel başlıklarını (header) alarak yanıt oluşturuyoruz.
    // Bu, çok temel bir yanıttır ve sadece çağrının kurulduğunu simüle eder.
    const getHeader = (name) => {
      const match = message.match(new RegExp(`^${name}:\\s*(.*)$`, 'im'));
      return match ? match[1].trim() : '';
    };

    const via = getHeader('Via');
    const from = getHeader('From');
    const to = getHeader('To');
    const callId = getHeader('Call-ID');
    const cseq = getHeader('CSeq');

    // Temel bir 200 OK yanıtı oluşturuyoruz.
    const response = [
      'SIP/2.0 200 OK',
      `Via: ${via}`,
      `From: ${from}`,
      `To: ${to}`,
      `Call-ID: ${callId}`,
      `CSeq: ${cseq}`,
      'Content-Length: 0',
      '\r\n' // Mesajın sonu
    ].join('\r\n');

    // Oluşturduğumuz yanıtı, isteğin geldiği adrese ve porta geri gönderiyoruz.
    server.send(Buffer.from(response), rinfo.port, rinfo.address, (err) => {
      if (err) {
        console.error('❌ Yanıt gönderilirken hata oluştu:', err);
      } else {
        console.log(`✅ [SIP Signaling] 200 OK yanıtı ${rinfo.address}:${rinfo.port} adresine gönderildi.`);
      }
    });
  }
});

// Sunucu bir hata ile karşılaştığında bu fonksiyon çalışır.
server.on('error', (err) => {
  console.error(`❌ Sunucu hatası:\n${err.stack}`);
  server.close();
});

// Sunucuyu belirtilen port ve hostta dinlemeye başlatıyoruz.
server.bind(SIP_PORT, SIP_HOST);