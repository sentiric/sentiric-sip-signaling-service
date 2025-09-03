# --- AŞAMA 1: Derleme (Builder) ---
FROM rust:1.88-slim-bookworm AS builder

# YENİ: Build argümanlarını tanımla
ARG GIT_COMMIT
ARG BUILD_DATE
ARG SERVICE_VERSION

# Gerekli derleme araçlarını kur
RUN apt-get update && \
    apt-get install -y protobuf-compiler git curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY . .

# YENİ: Build-time environment değişkenlerini ayarla
ENV GIT_COMMIT=${GIT_COMMIT}
ENV BUILD_DATE=${BUILD_DATE}
ENV SERVICE_VERSION=${SERVICE_VERSION}

RUN cargo build --release

# --- AŞAMA 2: Çalıştırma (Runtime) ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y netcat-openbsd ca-certificates && rm -rf /var/lib/apt/lists/*

# YENİ: Build argümanlarını tekrar tanımla ki runtime'da da kullanılabilsin
ARG GIT_COMMIT
ARG BUILD_DATE
ARG SERVICE_VERSION

# YENİ: Argümanları environment değişkenlerine ata
ENV GIT_COMMIT=${GIT_COMMIT}
ENV BUILD_DATE=${BUILD_DATE}
ENV SERVICE_VERSION=${SERVICE_VERSION}

WORKDIR /app

COPY --from=builder /app/target/release/sentiric-sip-signaling-service .

ENTRYPOINT ["./sentiric-sip-signaling-service"]