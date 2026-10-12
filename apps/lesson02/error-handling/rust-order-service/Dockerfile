# --- build aşaması ---
# Not: Cargo.toml'daki rust-version (MSRV) ile aynı sürüm kullanılır. Cargo.lock bu sürümle
# uyumlu bağımlılıklarla oluşturulmuştur. Sürümü yükseltirseniz ikisini birlikte güncelleyin.
FROM rust:1.88-slim-bookworm AS build
WORKDIR /src
# Önce sadece manifest dosyalarını kopyalayıp bağımlılıkları derlemek, Docker katman önbelleğinden yararlanmamızı sağlar.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && cargo build --release && rm -rf src
COPY src ./src
RUN touch src/main.rs && cargo build --release

# --- çalışma aşaması ---
FROM debian:bookworm-slim
COPY --from=build /src/target/release/rust-order-service /usr/local/bin/rust-order-service
ENV APP_ENV=production \
    RUST_LOG=info \
    BIND_ADDR=0.0.0.0:8081
EXPOSE 8081
CMD ["rust-order-service"]
