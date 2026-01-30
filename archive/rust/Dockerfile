FROM rust:1-alpine

# ユーザー作成 (UID:GID = 1000:1000)
RUN addgroup -g 1000 appuser && \
    adduser -D -u 1000 -G appuser appuser

WORKDIR /app

RUN apk add --no-cache \
    musl-dev \
    postgresql-dev \
    openssl-dev

# cargoディレクトリの権限設定
RUN mkdir -p /usr/local/cargo && \
    chown -R appuser:appuser /usr/local/cargo

COPY --chown=appuser:appuser library/Cargo.toml library/Cargo.lock* ./

RUN mkdir src && \
    echo "fn main() {}" > src/lib.rs && \
    chown -R appuser:appuser /app && \
    su appuser -c "cargo build --release" && \
    rm -rf src

USER appuser

CMD ["sh"]
