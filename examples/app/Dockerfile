FROM rust:1-slim AS builder
WORKDIR /build

COPY Cargo.toml ./Cargo.toml
COPY src/ ./src/
COPY examples/adapters/ ./examples/adapters/
COPY examples/app/ ./examples/app/

RUN cargo build --release --features state-engine/logging \
    --manifest-path examples/app/Cargo.toml

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /build/examples/app/target/release/state-engine-sample ./
COPY examples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
