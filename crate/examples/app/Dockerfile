FROM rust:1-slim AS builder
WORKDIR /build

COPY Cargo.toml ./Cargo.toml
COPY crate/src/ ./crate/src/
COPY crate/Cargo.toml ./crate/Cargo.toml
COPY crate/examples/adapters/ ./crate/examples/adapters/
COPY crate/examples/app/ ./crate/examples/app/

RUN cargo build --release --features state-engine/logging \
    --manifest-path crate/examples/app/Cargo.toml

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /build/crate/examples/app/target/release/state-engine-sample ./
COPY crate/examples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
