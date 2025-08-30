### **`sentiric-sip-signaling-service/TASKS.md` (Stratejik Revizyon v5.1)**

# 🚦 Sentiric SIP Signaling Service - Geliştirme Yol Haritası (v5.1)

Bu belge, `sip-signaling-service`'in, Sentiric Anayasası'nda tanımlanan **"Senkron Çağrı Kurulum Orkestratörü"** rolünden, tam teşekküllü bir **"İletişim Yönlendiricisi"** rolüne evrilmesi için gereken tüm görevleri, stratejik öncelik sırasına göre listeler.

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

**Amaç:** Platformu, AI ve insan ajanların bir arada çalışabildiği hibrit bir sisteme dönüştürmek ve teknik gözlem yeteneklerini en üst düzeye çıkarmak. Bu faz, platformun "ürünleşmesi" için kritik öneme sahiptir.

-   [ ] **Görev ID: SIG-012 - Çağrı Transferi (`REFER`)**
    -   **Durum:** ⬜ **Planlandı (SIRADAKİ EN YÜKSEK ÖNCELİK)**
    -   **Stratejik Önem:** Bu görev, AI'ın çağrıyı bir insana devredebilmesinin ("escape hatch") teknik temelidir. Bu olmadan, `web-agent-ui` gibi insan odaklı arayüzler işlevsiz kalır. Platformun hibrit bir yapıya kavuşması için **zorunludur**.
    -   **Tahmini Süre:** ~2-3 gün
    -   **Kabul Kriterleri:**
        -   [ ] Aktif bir çağrı sırasında gelen `REFER` isteği doğru bir şekilde parse edilmeli.
        -   [ ] `Refer-To` başlığındaki hedefe (örn: `sip:2001@sentiric.com`) yeni bir `INVITE` isteği gönderilerek "kör transfer" (blind transfer) başlatılmalı.
        -   [ ] Transferin durumu (`100 Trying`, `200 OK`, `503 Service Unavailable` vb.) standartlara uygun `NOTIFY` mesajları ile `REFER`'ı başlatan tarafa bildirilmelidir.
        -   [ ] **İlişkili Görev:** `agent-service`, "operatöre bağlan" niyeti algıladığında bu `REFER` mekanizmasını tetikleyecek mantığı içermelidir.

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
