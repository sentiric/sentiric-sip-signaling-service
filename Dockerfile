# --- AŞAMA 1: Derleme (Builder) ---
# En güncel ve 'slim' bir temel imaj kullanarak başlıyoruz.
FROM rust:1.88-slim-bookworm AS builder

# Gerekli derleme araçlarını kuruyoruz.
RUN apt-get update && apt-get install -y protobuf-compiler clang libclang-dev pkg-config

WORKDIR /app

# Bağımlılıkları önbelleğe almak için önce sadece Cargo dosyalarını kopyala
COPY Cargo.toml Cargo.lock ./
# Sahte bir src dizini oluşturarak sadece bağımlılıkları derle
RUN mkdir src && echo "fn main() {}" > src/main.rs
# --release olmadan derlemek, sadece bağımlılıkları çekmeyi hızlandırır.
RUN cargo build

# Kaynak kodunu kopyala ve asıl derlemeyi yap
COPY src ./src
# Artık production için optimize edilmiş tam bir build yapıyoruz.
RUN cargo build --release

# --- AŞAMA 2: Çalıştırma (Runtime) ---
# Mümkün olan en küçük ve en güvenli imajlardan birini kullanıyoruz.
FROM gcr.io/distroless/cc-debian12

WORKDIR /app
# 'sentiric-sip-signaling-service' adını doğru şekilde kullanıyoruz.
COPY --from=builder /app/target/release/sentiric-sip-signaling-service .

# Servisin dış dünyaya açtığı portları belirtmek iyi bir dokümantasyondur.
# Bu port, ana docker-compose dosyasında gateway tarafından kullanılır.
EXPOSE 5060/udp 

# Uygulamayı çalıştır
ENTRYPOINT ["./sentiric-sip-signaling-service"]