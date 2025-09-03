# 🚦 SIP Servisleri - Görev Listesi

Bu belge, `sip-signaling` ve `sip-gateway` servislerinin ortak sorumluluğu olan kritik çağrı kontrol hatalarını gidermek için gereken görevleri tanımlar.

---

### **FAZ 1: Stabil Çağrı Kurulumu (Tamamlanmış Görevler)**
*   [x] **SIG-001 - SIG-008**: Çekirdek `INVITE`/`BYE` akışı, orkestrasyon, olay yayınlama ve `REGISTER` kimlik doğrulama.
*   [x] **SIG-BUG-02 (YENİ): Yinelenen INVITE İsteklerine Karşı Dayanıklılık**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Öncelik:** **KRİTİK**
    -   **Problem Tanımı:** Telekom sağlayıcıları, `200 OK` yanıtını alana kadar aynı `Call-ID` ile defalarca `INVITE` gönderebilir. Bu durum, `sip-signaling`'in aynı çağrı için birden fazla `call.started` olayı yayınlamasına neden olarak `agent-service`'te yarış durumu (race condition) yaratıyordu.
    -   **Çözüm Stratejisi:** Bir `INVITE` başarıyla işlendiğinde, `Call-ID`'si kısa bir TTL (30sn) ile Redis'e "işlendi" olarak kaydedildi. Bu süre içinde gelen aynı `Call-ID`'li tüm `INVITE`'lar artık tamamen görmezden geliniyor.

---

### **FAZ 2: Güvenilir Çağrı Kontrolü ve Veri Bütünlüğü (Mevcut Odak)**

-   **Görev ID: SIG-BUG-01 - Çağrı Sonlandırma (`BYE`) Akışını Sağlamlaştırma**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Öncelik:** **KRİTİK**
    -   **Stratejik Önem:** Bu hata, çağrıların gereksiz yere uzun süre açık kalmasına, yanlış faturalandırmaya ve kötü bir kullanıcı deneyimine neden oluyordu.
    -   **Problem Tanımı:** Sistem `BYE` gönderdiğinde, `sip-gateway`'in `Via` başlıklarını doğru yönetmemesi nedeniyle paket telekom operatörüne ulaşmıyor ve çağrı açık kalıyordu.
    -   **Çözüm Stratejisi:** `sip-gateway` artık bir Session Border Controller (SBC) gibi davranarak gelen ve giden paketlerdeki `Via` başlıklarını modifiye ediyor, böylece yanıtların ve `BYE` gibi sonraki isteklerin doğru rotayı takip etmesini sağlıyor.
    -   **Kabul Kriterleri:**
        -   [x] `agent-service`, çağrıyı sonlandırma komutunu verdikten sonra, kullanıcının softphone'u veya telefon hattı **5 saniye içinde otomatik olarak kapanmalıdır.**
        -   [x] `sip-signaling` loglarında artık tekrarlayan "BYE isteği alınan çağrı aktif çağrılar listesinde bulunamadı" uyarısı görülmemelidir.
    -   **Tahmini Süre:** ~2-3 Gün

-   **Görev ID: SIG-FEAT-01 - `call.answered` Olayını Yayınlama**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Öncelik:** YÜKSEK
    -   **Stratejik Önem:** Doğru çağrı süresi ve maliyet hesaplaması için temel veriyi sağlar. Raporlama doğruluğu için zorunludur.
    -   **Bağımlılıklar:** `CDR-FEAT-01`
    -   **Çözüm Stratejisi:** `sip/invite.rs` içinde, istemciye `200 OK` yanıtı başarıyla gönderildikten hemen sonra, `RabbitMQ`'ya `call.answered` tipinde yeni bir olay yayınlandı.
    -   **Kabul Kriterleri:**
        -   [x] Bir çağrı cevaplandığında, RabbitMQ'da `call.answered` olayı görülmelidir.
        -   [x] `cdr-service` bu olayı işleyerek `calls` tablosundaki `answer_time` sütununu doldurmalıdır.
    -   **Tahmini Süre:** ~3-4 Saat

---

### **FAZ 3: Hibrit Etkileşim (Gelecek Vizyonu)**
-   [ ] **Görev ID: SIG-012 - Çağrı Transferi (`REFER`)**
    -   **Durum:** ⬜ **Planlandı**
    -   **Öncelik:** ORTA
