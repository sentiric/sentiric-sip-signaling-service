# --- AŞAMA 1: Derleme (Builder) ---
FROM rust:1.79 as builder
RUN apt-get update && apt-get install -y protobuf-compiler clang libclang-dev
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -f target/release/deps/sentiric_sip_signaling_service*
COPY src ./src
COPY build.rs ./
COPY ../sentiric-core-interfaces ./sentiric-core-interfaces
RUN cargo build --release

# --- AŞAMA 2: Çalıştırma (Runtime) ---
FROM gcr.io/distroless/cc-debian12
WORKDIR /app
COPY --from=builder /app/target/release/sentiric-sip-signaling-service .
EXPOSE 5060/udp
ENTRYPOINT ["/app/sentiric-sip-signaling-service"]