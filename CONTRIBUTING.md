# 🤝 Katkıda Bulunma Rehberi - Sentiric SIP Signaling Service

Bu belge, `sentiric-sip-signaling-service` projesine katkıda bulunmak isteyen geliştiriciler için en iyi pratikleri, geliştirme ortamı kurulumunu ve sık karşılaşılan sorunların çözümlerini içerir.

## 🚀 Geliştirme Ortamı Kurulumu (Önerilen Yöntem)

Bu servis, bir mikroservis mimarisinin parçası olduğu için yüksek derecede bağımlılığa sahiptir. Bu nedenle, servisi tek başına `cargo run` ile çalıştırmak yerine, kritik bağımlılıklarını içeren izole bir Docker Compose ortamında geliştirmek **kesinlikle tavsiye edilir.**

### Gerekli Adımlar:

1.  **Bağımlı Repoları Klonla:**
    Bu projenin, aşağıdaki repolarla aynı dizin seviyesinde (`../`) olduğundan emin ol:
    ```
    /workspace
    |-- /sentiric-sip-signaling-service  (bu repo)
    |-- /sentiric-config
    `-- /sentiric-certificates
    ```

2.  **Environment Dosyasını Oluştur:**
    Projenin `Makefile`'ı, gerekli `.env.generated` dosyasını sizin için oluşturabilir. Projenin ana dizinindeyken şu komutu çalıştırın:
    ```bash
    make _generate_env
    ```

3.  **İzole Ortamı Başlat:**
    Tüm bağımlılıkları (`postgres`, `redis`, `rabbitmq`, `user-service` vb.) hazır imajlardan çekecek ve sadece `sip-signaling-service`'i yerel kaynak kodunuzdan derleyecek olan ortamı başlatın:
    ```bash
    make start
    # veya doğrudan:
    # docker compose -f docker-compose.dev.yml up --build
    ```

4.  **Değişiklikleri Uygula:**
    Kodunuzda bir değişiklik yaptıktan sonra, sadece `sip-signaling` servisini yeniden derleyip başlatmak için:
    ```bash
    make restart
    # veya
    # docker compose -f docker-compose.dev.yml restart sip-signaling
    ```

## 🏛️ Mimari ve Geliştirme Prensipleri

Bu prensipler, geçmişte karşılaşılan zorlu hata ayıklama süreçlerinden çıkarılan dersler üzerine kurulmuştur ve projenin sürdürülebilirliği için kritik öneme sahiptir.

### 1. İzole Entegrasyon Testlerini Tercih Et

`sip-signaling-service` gibi bir orkestratör servis, tek başına anlamsızdır. Bu nedenle, `cargo run` ile yapılan testler yanıltıcı olabilir. Geliştirme ve test için her zaman projenin kendi `docker-compose.dev.yml` dosyasını kullanarak, servisi kendi "mini-ekosistemi" içinde çalıştırın.

### 2. Sıralı ve Basit Başlangıç (Bootstrap) Paterni

Servisler, kilitlenme (deadlock) riskini en aza indirmek için sıralı bir başlangıç mantığı izlemelidir:

-   **ÖNCE:** Tüm kritik bağımlılıklar (`config` yükleme, veritabanı bağlantısı, gRPC istemcileri) ana görevde (`main` fonksiyonu içinde) senkron olarak başlatılır.
-   **SONRA:** Tüm bağımlılıkların hazır olduğundan emin olunduktan sonra ağ dinleyicileri (UDP, TCP, gRPC sunucusu) ve diğer asenkron döngüler başlatılır.

*❌ Kaçınılması Gereken: Ana döngü çalışırken arka planda `AppState` gibi kritik bir durumu asenkron olarak başlatan ve `Mutex` ile paylaşılan karmaşık yapılardan kaçının.*

### 3. Gözlemlenebilir Asenkron Görevler (`tokio::spawn`)

"Ateşle ve unut" (`fire and forget`) şeklinde `tokio::spawn` ile başlatılan görevler, panik durumunda **sessizce çökebilir** ve hata ayıklamayı imkansız hale getirebilir.

-   Bir görevin çökmesi kabul edilemezse, sonucunu bir `JoinHandle` veya `JoinSet` ile yönetin.
-   Eğer "ateşle ve unut" zorunluysa, görevin kendi içinde `std::panic::catch_unwind` gibi bir mekanizma ile olası panikleri yakalayıp logladığından emin olun.

**Hedefimiz: Sessizce çöken hiçbir görev olmamalıdır.**

---

Bu prensiplere bağlı kalmak, daha dayanıklı, test edilebilir ve bakımı kolay servisler geliştirmemize yardımcı olacaktır.