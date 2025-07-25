# --- AŞAMA 1: Derleme (Builder) ---
# DÜZELTME: Stabil bir temel (1.82) kullanıyoruz ama içine nightly derleyiciyi kuruyoruz.
FROM rust:1.82 AS builder

# Gerekli sistem kütüphanelerini kur
RUN apt-get update && apt-get install -y protobuf-compiler clang libclang-dev

# Nightly araç zincirini (toolchain) kur ve varsayılan yap
RUN rustup toolchain install nightly
RUN rustup default nightly

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
# Artık tüm 'cargo' komutları 'nightly' derleyicisini kullanacak
RUN cargo build --release
RUN rm -f target/release/deps/sentiric_sip_signaling_service*

COPY src ./src
COPY build.rs ./
COPY ./core-interfaces ./core-interfaces
RUN cargo build --release

# --- AŞAMA 2: Çalıştırma (Runtime) ---
FROM gcr.io/distroless/cc-debian12
WORKDIR /app
# DÜZELTME: Alt çizgileri (-) tire ile değiştiriyoruz.
COPY --from=builder /app/target/release/sentiric-sip-signaling-service .
EXPOSE 5060/udp
# DÜZELTME: Alt çizgileri (-) tire ile değiştiriyoruz.
ENTRYPOINT ["/app/sentiric-sip-signaling-service"]