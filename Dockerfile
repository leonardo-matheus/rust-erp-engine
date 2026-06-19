FROM rust:slim-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    pkg-config \
    libssl-dev \
    ca-certificates \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

COPY backend/Cargo.toml backend/Cargo.lock ./backend/

RUN mkdir -p backend/src && \
    echo "fn main() {}" > backend/src/main.rs && \
    echo "pub mod config; pub mod crypto; pub mod db; pub mod error; pub mod middleware; pub mod services; pub mod store; pub mod validation; pub mod auth; pub const VERSION: &str = \"0.0.0\"; pub use config::AppConfig; pub use error::AppError;" > backend/src/lib.rs && \
    mkdir -p backend/proto && touch backend/proto/erp.proto

RUN cd backend && cargo build --release 2>/dev/null || true

COPY backend/ ./backend/

RUN cd backend && cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libsqlite3-0 \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash appuser

COPY --from=builder /app/backend/target/release/server /app/server

RUN chown appuser:appuser /app/server

USER appuser

WORKDIR /app

EXPOSE 10000

CMD ["/app/server"]
