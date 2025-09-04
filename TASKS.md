# 🚦 SIP Signaling Service - Görev Listesi (v2.0 - Dayanıklı Çekirdek)

Bu belge, `sip-signaling-service`'in geliştirme yol haritasını, tamamlanan kritik kilometre taşlarını ve gelecekteki hedeflerini tanımlar.

---

### **FAZ 1: Temel Çağrı Kurulumu (Tamamlandı)**
Bu faz, servisin temel `INVITE`/`BYE` akışını, orkestrasyonunu ve olay yayınlama yeteneklerini oluşturdu.

*   [x] **SIG-CORE-01 - SIG-008**: Çekirdek `INVITE`/`BYE` akışı, orkestrasyon, olay yayınlama ve `REGISTER` kimlik doğrulama.
*   [x] **SIG-BUG-02: Yinelenen INVITE İsteklerine Karşı Dayanıklılık**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Kazanım:** Redis üzerinde atomik bir kilit mekanizması kurularak, telekom operatörlerinden gelen yinelenen `INVITE`'ların sisteme birden fazla çağrı olarak girmesi engellendi.
*   [x] **SIG-FEAT-01: `call.answered` Olayını Yayınlama**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Kazanım:** Doğru çağrı süresi ve maliyet hesaplaması için kritik olan `call.answered` olayı, çağrı cevaplandığı anda yayınlanmaya başlandı.

---

### **FAZ 2: Dayanıklı ve Uyumlu Sinyalleşme (Mevcut Durum - TAMAMLANDI)**
Bu faz, servisi basit bir orkestratörden, gerçek dünya telekom senaryolarının karmaşıklığına ve hatalarına karşı dayanıklı, üretim seviyesinde bir çekirdek bileşen haline getirmeyi hedefliyordu. **Bu faz başarıyla tamamlanmıştır.**

-   [x] **Görev ID: MIMARI-01 (YENİ) - Dayanıklı ve Anında Yanıt Veren Başlangıç Mimarisi**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Öncelik:** **MİMARİ**
    -   **Kazanım:** Servis artık UDP portunu anında dinlemeye başlıyor. Arka planda kritik bağımlılıklara (gRPC, Redis) bağlanmaya çalışırken, gelen `INVITE` isteklerini yanıtsız bırakmak yerine anında `503 Service Unavailable` ile cevaplıyor. Bağımlılıklar hazır olduğunda ise tam işlevsel moda geçiyor. Bu, hem telekom hız beklentisini karşılıyor hem de sistemin kararlılığını garanti ediyor.

-   [x] **Görev ID: SIG-BUG-01 - Telekom Uyumluluğu ve Çağrı Sonlandırma (`BYE`) Akışını Sağlamlaştırma**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Öncelik:** **KRİTİK**
    -   **Stratejik Önem:** Bu karmaşık hata, çağrıların telekom tarafında açık kalmasına neden oluyordu.
    -   **Problem Tanımı:** Telekom operatörünün `Record-Route` başlığında hem standart dışı parametreler (`ftag`) hem de yazım hataları (`trasport`) göndermesi ve giden `BYE` isteğinde bu hataların düzeltilmiş ancak standart dışı beklentilerine uygun bir `Route` başlığı beklemesi.
    -   **Çözüm Stratejisi:** `create_bye_request` fonksiyonu, `Route` başlığını oluştururken artık daha akıllıdır.
        1.  `trasport` gibi bilinen yazım hatalarını proaktif olarak düzeltir.
        2.  `ftag` gibi, karşı tarafın uyumluluk için beklediği standart dışı parametrelere dokunmadan, olduğu gibi geri gönderir.
    -   **Kabul Kriterleri:**
        -   [x] `agent-service` çağrıyı sonlandırdıktan sonra, telekom operatörü `BYE` isteğini kabul eder ve `200 OK` yanıtı döner.
        -   [x] `sip-gateway` loglarında artık `475 Bad URI` hatası görülmez.

---

### **FAZ 3: Gelişmiş Çağrı Özellikleri (Sıradaki Öncelik)**
Bu faz, platformun daha karmaşık ve kullanıcı odaklı çağrı yönetimi senaryolarını desteklemesini sağlayacak özellikleri eklemeyi hedefler.

-   [ ] **Görev ID: SIG-012 - Çağrı Transferi (`REFER`)**
    -   **Durum:** ⬜ **Planlandı**
    -   **Öncelik:** ORTA
    -   **Açıklama:** Bir AI diyaloğunun, çağrıyı bir insana veya başka bir hedefe sorunsuz bir şekilde aktarmasını sağlayan SIP `REFER` metodunu implemente et.

-   [ ] **Görev ID: SIG-013 - Çağrı Bekletme (`HOLD`)**
    -   **Durum:** ⬜ **Planlandı**
    -   **Öncelik:** ORTA
    -   **Açıklama:** Çağrıları beklemeye alma ve geri alma (müzik çalma gibi) yeteneklerini ekle. Bu, `re-INVITE` ve SDP (Session Description Protocol) manipülasyonu gerektirir.