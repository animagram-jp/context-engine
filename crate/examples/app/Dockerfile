FROM rust:1-slim AS builder
WORKDIR /build

COPY core/Cargo.toml ./core/Cargo.toml
COPY crate/Cargo.toml ./crate/Cargo.toml
COPY crate/examples/app/Cargo.toml ./crate/examples/app/Cargo.toml

RUN mkdir -p core/src crate/src crate/examples/adapters crate/examples/app/src && \
    echo "pub fn placeholder() {}" > core/src/lib.rs && \
    echo "pub fn placeholder() {}" > crate/src/lib.rs && \
    touch crate/examples/adapters/mod.rs && \
    touch crate/examples/adapters/in_memory.rs && \
    touch crate/examples/adapters/env_client.rs && \
    touch crate/examples/adapters/kvs_client.rs && \
    touch crate/examples/adapters/db_client.rs && \
    echo "fn main() {}" > crate/examples/app/src/main.rs && \
    touch crate/examples/app/src/adapters.rs && \
    cargo build --release --features state-engine/logging \
        --manifest-path crate/examples/app/Cargo.toml ; \
    rm -rf core/src crate/src crate/examples/adapters crate/examples/app/src && \
    rm -rf crate/examples/app/target/release/.fingerprint/state-engine-* \
           crate/examples/app/target/release/.fingerprint/state-engine-core-* \
           crate/examples/app/target/release/state-engine-sample \
           crate/examples/app/target/release/deps/state_engine-* \
           crate/examples/app/target/release/deps/state_engine_core-* \
           crate/examples/app/target/release/deps/state_engine_sample-*

COPY core/ ./core/
COPY crate/src/ ./crate/src/
COPY crate/examples/adapters/ ./crate/examples/adapters/
COPY crate/examples/app/src/ ./crate/examples/app/src/
RUN cargo build --release --features state-engine/logging \
    --manifest-path crate/examples/app/Cargo.toml

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /build/crate/examples/app/target/release/state-engine-sample ./
COPY crate/examples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
