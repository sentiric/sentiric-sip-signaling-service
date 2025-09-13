# 🚦 SIP Signaling Service - Görev Listesi (v2.1 - Strateji B+ Mimarisi)

Bu belge, `sip-signaling-service`'in geliştirme yol haritasını, tamamlanan kritik kilometre taşlarını ve gelecekteki hedeflerini tanımlar.

---

### **FAZ 1: Temel Çağrı Kurulumu (Arşivlendi)**
Bu faz, servisin temel `INVITE`/`BYE` akışını, orkestrasyonunu ve olay yayınlama yeteneklerini oluşturdu.

---

### **FAZ 2: Strateji B+ ve Mimari Sağlamlaştırma (Mevcut Durum - TAMAMLANDI)**
Bu faz, servisi basit bir orkestratörden, `sip-gateway` ile sorumlulukları net bir şekilde ayrılmış, dayanıklı ve mimari olarak temiz bir çekirdek bileşen haline getirmeyi hedefliyordu. **Bu faz başarıyla tamamlanmıştır.**

-   **Görev ID: MIMARI-02 (YENİ) - Strateji B+ ile Sorumlulukların Ayrıştırılması**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Öncelik:** **MİMARİ**
    -   **Kazanım:** Servisin SIP işleme mantığı, artık dış dünyanın karmaşıklıklarıyla (çoklu `Via`, `Record-Route`, NAT sorunları) ilgilenmeyecek şekilde kökten basitleştirilmiştir. Bu sorumluluklar tamamen `sip-gateway-service`'e devredilmiştir.
    -   **Teknik Detaylar:**
        -   `sip/utils.rs`: `parse_complex_headers` fonksiyonu, artık sadece tek `Via` başlığı bekleyecek şekilde basitleştirildi.
        -   `rabbitmq/terminate.rs`: `create_bye_request` fonksiyonu, artık karmaşık `Route` başlıkları oluşturmak yerine, basit bir `BYE` isteğini doğrudan `gateway`'e yönlendirir.
        -   `sip/call_context.rs`: Operatör kaynaklı `trasport` yazım hatasını düzeltme mantığı gibi dış dünyaya özel kodlar temizlendi.
    -   **Stratejik Önem:** Bu değişiklik, servisin kod tabanını daha temiz, daha odaklı ve bakımı daha kolay hale getirmiştir.

-   **Görev ID: MIMARI-01 - Dayanıklı ve Anında Yanıt Veren Başlangıç Mimarisi**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Kazanım:** Servis, kritik bağımlılıkları (gRPC, Redis) hazır olmasa bile `503 Service Unavailable` yanıtı vererek telekom standartlarına uygun bir şekilde çalışır.

-   **Görev ID: SIG-BUG-02 - Yinelenen INVITE İsteklerine Karşı Dayanıklılık**
    -   **Durum:** ✅ **Tamamlandı**
    -   **Kazanım:** Redis üzerinde atomik bir kilit mekanizması kurularak, yinelenen `INVITE`'ların sisteme birden fazla çağrı olarak girmesi engellendi.

---

### **FAZ 3: Zenginleştirilmiş Olaylar ve Temizlik (Sıradaki Öncelik)**

**Amaç:** Platformun geri kalanına daha zengin ve temiz veri sağlayarak asenkron iş akışlarının doğru çalışmasını garanti altına almak.

-   **Görev ID: SIG-FEAT-01 - `call.started` Olayını Kullanıcı Bilgileriyle Zenginleştirme**
    -   **Durum:** ⬜ **Yapılacak (Öncelik 1 - KRİTİK)**
    -   **Bağımlılık:** `sentiric-contracts`'teki `CT-FEAT-01` görevinin tamamlanmış olması.
    -   **Açıklama:** `dialplan-service`'ten alınan `ResolveDialplanResponse` nesnesinin tamamını, yeni kontratlara uygun olarak `call.started` olayının `dialplan_resolution` alanına eklemek. Bu, `agent-service`'in arayanı doğru bir şekilde tanımasını sağlayacaktır.
    -   **Kabul Kriterleri:**
        -   [ ] `sip/invite/orchestrator.rs` içindeki `publish_call_event` fonksiyonu, `dialplan_res` parametresini almalı ve `serde_json` kullanarak `event_payload`'a eklemelidir.
        -   [ ] Yapılan bir test aramasında, RabbitMQ'ya giden `call.started` mesajının içinde `dialplan` anahtarının ve altında `matchedUser` bilgilerinin olduğu doğrulanmalıdır.

-   **Görev ID: SIG-CLEANUP-01 - Gereksiz `call.answered` Olayını Kaldırma**
    -   **Durum:** ⬜ **Yapılacak (Öncelik 2)**
    -   **Açıklama:** Mevcut akışta `agent-service` tarafından görmezden gelinen ve `call.started` ile aynı anda yayınlanan `call.answered` olayını kaldırmak. Bu, sistemdeki gereksiz gürültüyü azaltacak ve mimariyi basitleştirecektir.
    -   **Kabul Kriterleri:**
        -   [ ] `sip/invite/orchestrator.rs` içindeki `setup_and_finalize_call` fonksiyonundan `call.answered` olayını yayınlayan kod satırı kaldırılmalıdır.

-   [ ] **Görev ID: SIG-012 - Çağrı Transferi (`REFER`)**
-   [ ] **Görev ID: SIG-013 - Çağrı Bekletme (`HOLD`)**