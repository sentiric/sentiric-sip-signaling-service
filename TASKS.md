# 🚦 Sentiric SIP Signaling Service - Geliştirme Yol Haritası (v4.1)

Bu belge, servisin geliştirme görevlerini projenin genel fazlarına uygun olarak listeler.

---

### **FAZ 1: Stabilizasyon ve Çekirdek Çağrı Akışı**

**Amaç:** Canlı çağrı akışının çalışmasını engelleyen temel sorunları gidermek ve platformun temel çağrı kurulum/sonlandırma yeteneklerini sağlamlaştırmak.

-   [x] **Görev ID: SIG-000 - Çekirdek `INVITE`/`BYE` Akışı**
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-000B - Senkron Orkestrasyon**
    -   **Durum:** ✅ **Tamamlandı**

-   [x] **Görev ID: SIG-000C - Asenkron Olay Yayınlama**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Güncelleme:** Bu görev kapsamında, RabbitMQ mimarisi `fanout` exchange'den, hedefe yönelik mesajlaşmayı (`call.terminate.request`) destekleyen ve daha esnek olan `topic` exchange'e geçirildi.

-   [x] **Görev ID: SIG-004 - Gözlemlenebilirlik Standardı Uyumu**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Açıklama:** `tracing` yapılandırması `OBSERVABILITY_STANDARD.md` ile uyumlu hale getirildi.

-   [x] **Görev ID: SIG-005 - Uzaktan Çağrı Sonlandırma**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Açıklama:** `call.terminate.request` olaylarını dinleyerek, `agent-service` gibi diğer servislerden gelen komutlarla çağrıları proaktif olarak sonlandırma yeteneği eklendi.

-   [x] **Görev ID: SIG-006 - Kodun Modülerleştirilmesi**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Açıklama:** `src/main.rs` dosyası, sorumlulukların ayrı modüllere taşınmasıyla yeniden yapılandırıldı.

-   [x] **Görev ID: SIG-007 - Yinelenen INVITE Yönetimi (YENİ GÖREV)**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Açıklama:** Telekom sağlayıcılarından gelebilen ve aynı `Call-ID`'ye sahip yinelenen `INVITE` isteklerinin, platformda birden fazla çağrı süreci başlatmasını engelleyen bir kilit mekanizması eklendi.
    -   **Kabul Kriterleri:**
        -   [x] Bir `Call-ID` için ilk `INVITE` işleme alındığında, bu `Call-ID` aktif çağrılar listesine eklenir.
        -   [x] Aynı `Call-ID` ile gelen sonraki `INVITE` istekleri, loglanır ve görmezden gelinir.
        -   [x] Bu, `media-service` üzerinde gereksiz port tahsis edilmesini ve `agent-service`'te yinelenen diyalog döngüleri oluşmasını engeller.

---

### **FAZ 2: Gelişmiş SIP Yetenekleri (Sıradaki Öncelik)**

**Amaç:** Platformu, standart SIP istemcilerinin bağlanabildiği ve daha karmaşık çağrı senaryolarını yönetebilen tam teşekküllü bir SIP sunucusuna dönüştürmek.

-   [ ] **Görev ID: SIG-001 - `REGISTER` Metodu Desteği**
    -   **Açıklama:** SIP istemcilerinin (softphone, mobil uygulama) platforma kayıt olmasını (`REGISTER`) ve `user-service` üzerinden kimlik doğrulaması yapmasını sağla. Bu, platformdan dışarıya doğru arama yapmanın ilk adımıdır.

-   [ ] **Görev ID: SIG-002 - `CANCEL` Metodu Desteği**
    -   **Açıklama:** Bir `INVITE` isteği gönderildikten sonra, ancak `200 OK` yanıtı alınmadan önce çağrının iptal edilmesini sağlayan `CANCEL` isteğini işle.

-   [ ] **Görev ID: SIG-003 - Çağrı Transferi (`REFER`)**
    -   **Açıklama:** Aktif bir çağrıyı başka bir SIP kullanıcısına veya harici bir numaraya yönlendirmeyi sağlayan `REFER` metodunu implemente et.
    -   **Durum:** ⬜ Planlandı.