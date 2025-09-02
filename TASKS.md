# 🚦 SIP Servisleri - Görev Listesi

Bu belge, `sip-signaling` ve `sip-gateway` servislerinin ortak sorumluluğu olan kritik çağrı kontrol hatalarını gidermek için gereken görevleri tanımlar.

---

### **FAZ 1: Stabil ve Fonksiyonel Omurga (Tamamlandı)**

**Amaç:** Platformun temel çağrı kurulum/sonlandırma, kimlik doğrulama ve gözlemlenebilirlik yeteneklerini sağlamlaştırmak. Bu faz, platformun üzerine yeni özelliklerin inşa edileceği sağlam zemini oluşturmuştur.

-   [x] **Görev ID: SIG-001 - Çekirdek `INVITE`/`BYE` Akışı**
    -   **Durum:** ✅ **Tamamlandı**
-   [x] **Görev ID: SIG-002 - Senkron Orkestrasyon Mantığı**
    -   **Durum:** ✅ **Tamamlandı**
-   [x] **Görev ID: SIG-003 - Asenkron Olay Yayınlama**
    -   **Durum:** ✅ **Tamamlandı**
-   [x] **Görev ID: SIG-004 - Gözlemlenebilirlik Standardı Uyumu**
    -   **Durum:** ✅ **Tamamlandı**
-   [x] **Görev ID: SIG-005 - Uzaktan Çağrı Sonlandırma**
    -   **Durum:** ✅ **Tamamlandı**
-   [x] **Görev ID: SIG-006 - Kodun Modülerleştirilmesi**
    -   **Durum:** ✅ **Tamamlandı**
-   [x] **Görev ID: SIG-007 - Yinelenen `INVITE` Yönetimi**
    -   **Durum:** ✅ **Tamamlandı**
-   [x] **Görev ID: SIG-008 - `REGISTER` Metodu ile Kimlik Doğrulama**
    -   **Durum:** ✅ **Tamamlandı**

---

### **FAZ 2: Hibrit Etkileşim ve Gelişmiş Yönlendirme (Mevcut Odak)**

-   **Görev ID: SIG-BUG-01 - Çağrı Sonlandırma (`BYE`) Akışını Sağlamlaştırma**
    -   **Durum:** ⬜ **Yapılacak**
    -   **Öncelik:** **KRİTİK**
    -   **Stratejik Önem:** Bu hata, çağrıların gereksiz yere uzun süre açık kalmasına, yanlış faturalandırmaya (süre hesaplama) ve kötü bir kullanıcı deneyimine neden olmaktadır. Platformun temel güvenilirliğini doğrudan etkiler.
    -   **Problem Tanımı:** Test çağrısı 1.5 dakika boyunca açık kalmış ve manuel kapatılmıştır. Loglar, `agent-service`'in sonlandırma isteğini gönderdiğini, `sip-signaling`'in bir `BYE` gönderdiğini ancak telekom operatörünün bunu almadığı için sürekli yeni `BYE` istekleri gönderdiğini göstermektedir.
    -   **Çözüm Stratejisi:** Sorun, `sip-gateway`'in gelen `INVITE` paketindeki `Via` ve `Record-Route` başlıklarını, `BYE` gibi daha sonraki isteklerin doğru rotayı takip edebileceği şekilde modifiye etmemesinden kaynaklanmaktadır. Gateway, bir Session Border Controller (SBC) gibi davranarak bu başlıkları kendi adresiyle güncellemeli ve çağrı boyunca durumu korumalıdır.
    -   **Kabul Kriterleri:**
        -   [ ] `agent-service`, çağrıyı sonlandırma komutunu verdikten sonra, kullanıcının softphone'u veya telefon hattı **5 saniye içinde otomatik olarak kapanmalıdır.**
        -   [ ] `sip-signaling` loglarında artık tekrarlayan "BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı" uyarısı görülmemelidir.
    -   **Tahmini Süre:** ~2-3 Gün

-   **Görev ID: SIG-FEAT-01 - `call.answered` Olayını Yayınlama**
    -   **Durum:** ⬜ **Yapılacak**
    -   **Öncelik:** YÜKSEK
    -   **Stratejik Önem:** Doğru çağrı süresi ve maliyet hesaplaması için temel veriyi sağlar. Raporlama doğruluğu için zorunludur.
    -   **Bağımlılıklar:** `CDR-FEAT-01`
    -   **Çözüm Stratejisi:** `sip/invite.rs` içinde, istemciye `200 OK` yanıtı başarıyla gönderildikten hemen sonra, `RabbitMQ`'ya `call.answered` tipinde yeni bir olay yayınlanmalıdır. Bu olay `call_id`, `trace_id` ve `timestamp` içermelidir.
    -   **Kabul Kriterleri:**
        -   [ ] Bir çağrı cevaplandığında, RabbitMQ'da `call.answered` olayı görülmelidir.
        -   [ ] `cdr-service` bu olayı işleyerek `calls` tablosundaki `answer_time` sütununu doldurmalıdır.
    -   **Tahmini Süre:** ~3-4 Saat

-   [ ] **Görev ID:** `AGENT-BUG-05`
    *   **Açıklama:** `call.terminate.request` olayı yayınlanırken, JSON payload'una `eventType: "call.terminate.request"` alanının eklenmesini sağla.
    *   **Kabul Kriterleri:**
        *   [ ] `call_events` tablosunda artık `event_type` alanı boş olan kayıtlar görülmemelidir.

-   [ ] **Görev ID: SIG-009 - P2P Çağrı Yönlendirme (SIP Proxy Mantığı)**
    -   **Durum:** ⬜ **Planlandı (İkinci Öncelik)**
    -   **Stratejik Önem:** Platformun teknik yeterliliğini kanıtlar ve dahili test/gözlem yeteneklerini muazzam artırır. Geliştiricilerin ve ajanların, `media-service` ve `agent-service`'in canlı davranışını bir softphone aracılığıyla doğrudan test etmelerini sağlar.
    -   **Tahmini Süre:** ~3-5 gün (SIP kütüphanesi kullanılmazsa)
    -   **Kabul Kriterleri:**
        -   [ ] Aranan URI'nin bir telefon hattı mı (`90...`) yoksa bir SIP kullanıcısı mı olduğu tespit edilmeli.
        -   [ ] SIP kullanıcısı hedefleniyorsa, `dialplan-service`'e gidilmemeli.
        -   [ ] Hedef kullanıcının kayıtlı `contact` adresi Redis'ten okunmalı.
        -   [ ] Gelen `INVITE` paketi, `Request-URI` hedef kullanıcının `contact` adresi olacak şekilde modifiye edilmeli ve `Via`/`Record-Route` başlıkları güncellenerek hedefe gönderilmelidir.
        -   [ ] Uçtan uca test: Bir softphone'dan (`1001`) diğerine (`2001`) yapılan arama başarıyla kurulmalı ve iki taraf arasında ses akışı sağlanmalıdır.

---

### **FAZ 3: Protokol Uyumluluğu ve Dayanıklılık**


-   [ ] **Görev ID: SIG-012 - Çağrı Transferi (`REFER`)**
    -   **Durum:** ⬜ **Planlandı (SIRADAKİ EN YÜKSEK ÖNCELİK)**
    -   **Stratejik Önem:** Bu görev, AI'ın çağrıyı bir insana devredebilmesinin ("escape hatch") teknik temelidir. Bu olmadan, `web-agent-ui` gibi insan odaklı arayüzler işlevsiz kalır. Platformun hibrit bir yapıya kavuşması için **zorunludur**.
    -   **Tahmini Süre:** ~2-3 gün
    -   **Kabul Kriterleri:**
        -   [ ] Aktif bir çağrı sırasında gelen `REFER` isteği doğru bir şekilde parse edilmeli.
        -   [ ] `Refer-To` başlığındaki hedefe (örn: `sip:2001@sentiric.com`) yeni bir `INVITE` isteği gönderilerek "kör transfer" (blind transfer) başlatılmalı.
        -   [ ] Transferin durumu (`100 Trying`, `200 OK`, `503 Service Unavailable` vb.) standartlara uygun `NOTIFY` mesajları ile `REFER`'ı başlatan tarafa bildirilmelidir.
        -   [ ] **İlişkili Görev:** `agent-service`, "operatöre bağlan" niyeti algıladığında bu `REFER` mekanizmasını tetikleyecek mantığı içermelidir.


-   [ ] **Görev ID: SIG-BUG-01 - Çağrı Sonlandırma (`BYE`) Akışını Sağlamlaştırma (YÜKSEK ÖNCELİK)**
    -   **Durum:** ⬜ Planlandı
    -   **Açıklama:** `agent-service` tarafından tetiklenen çağrı sonlandırma işleminin, istemci softphone'u güvenilir bir şekilde kapatmaması sorununun çözülmesi. Bu, hem doğru faturalandırma (süre hesaplama) hem de iyi bir kullanıcı deneyimi için kritiktir.
    -   **Kabul Kriterleri:**
        -   [ ] `agent-service` `call.terminate.request` olayını yayınladıktan sonra, `sip-signaling` tarafından gönderilen `BYE` paketinin istemciye ulaştığı ve istemcinin çağrıyı **otomatik olarak sonlandırdığı** doğrulanmalıdır.
        -   [ ] Bu akış sırasında gönderilen `BYE` paketinin SIP başlıkları (`Via`, `Route`, `Record-Route` vb.) incelenmeli ve RFC standartlarına uygunluğu kontrol edilmelidir.
        -   [ ] Çağrı sonlandıktan sonra, istemciden gelebilecek yinelenen `BYE` istekleri, servisin çökmesine veya hatalı davranışına neden olmamalı, güvenli bir şekilde `481 Call/Transaction Does Not Exist` yanıtı ile karşılanmalıdır.
        
**Amaç:** Platformun standart SIP istemcileriyle tam uyumlu çalışmasını sağlamak ve beklenmedik senaryolara karşı daha dayanıklı hale getirmek.

-   [ ] **Görev ID: SIG-011 - `CANCEL` Metodu Desteği**
    -   **Durum:** ⬜ **Planlandı**
    -   **Stratejik Önem:** Çağrı kurulum sürecini daha sağlam hale getirir ve kaynakların (özellikle `media-service` portları) gereksiz yere meşgul edilmesini önler. Protokol uyumluluğu için önemlidir.
    -   **Tahmini Süre:** ~1-2 gün
    -   **Kabul Kriterleri:**
        -   [ ] `INVITE` gönderildikten, ancak `200 OK` alınmadan önce gelen bir `CANCEL` isteği, ilgili çağrı kurulum sürecini (tüm gRPC çağrıları dahil) iptal etmelidir.
        -   [ ] Eğer `media-service`'ten port tahsis edildiyse, derhal `ReleasePort` komutuyla iade edilmelidir.
        -   [ ] Hem orijinal `INVITE`'a (`487 Request Terminated`) hem de `CANCEL`'a (`200 OK`) standartlara uygun yanıtlar gönderilmelidir.

-   [ ] **Görev ID: SIG-010 - Kullanıcı Durum Yönetimi (Presence)**
    -   **Durum:** ⬜ **Planlandı**
    -   **Stratejik Önem:** `web-agent-ui`'da hangi ajanların müsait, meşgul veya çevrimdışı olduğunu göstermenin temelini oluşturur. Bu, akıllı çağrı yönlendirme (müsait ajana aktarma) için bir ön koşuldur.
    -   **Tahmini Süre:** ~2-3 gün
    -   **Kabul Kriterleri:**
        -   [ ] `PUBLISH` metodu işlenerek kullanıcı durumları (online, busy vb.) alınmalı ve Redis'te saklanmalı.
        -   [ ] `SUBSCRIBE` metodu ile bir kullanıcının başka bir kullanıcının durumunu takip etme talebi yönetilmeli.
        -   [ ] Durum değişikliği olduğunda, abone olan kullanıcılara `NOTIFY` mesajı ile bildirim gönderilmeli.

---

### **FAZ 4: Uzun Vadeli İyileştirmeler ve Teknik Borç Ödemesi**

**Amaç:** Platformun güvenliğini, bakımını ve ölçeklenebilirliğini en üst düzeye çıkarmak.

-   [ ] **Görev ID: SIG-013 - Gelişmiş Kimlik Doğrulama Mantığı**
    -   **Durum:** ⬜ **Planlandı**
    -   **Stratejik Önem:** Güvenlik ve kimlik doğrulama mantığını tek bir sorumlu serviste (`user-service`) merkezileştirerek "Tek Sorumluluk Prensibi"ni güçlendirir ve bakımı kolaylaştırır.
    -   **Bağımlılık:** `sentiric-user-service`'de yeni bir `VerifySipPassword` RPC'sinin oluşturulmasını gerektirir (`USER-007`).
    -   **Tahmini Süre:** ~1 gün (bağımlılık tamamlandıktan sonra)
    -   **Kabul Kriterleri:**
        -   [ ] `sip-signaling-service` artık MD5 hash hesaplaması yapmamalı.
        -   [ ] `REGISTER` isteğindeki `Authorization` başlığının içeriği, yeni `user-service` RPC'sine gönderilmeli ve dönen `true/false` yanıtına göre işlem yapılmalıdır.
