FROM rust:1-slim AS builder
WORKDIR /build

COPY core/Cargo.toml ./core/Cargo.toml
COPY rust/Cargo.toml ./rust/Cargo.toml
COPY rust/examples/app/Cargo.toml ./rust/examples/app/Cargo.toml

RUN mkdir -p core/src rust/src rust/examples/adapters rust/examples/app/src && \
    echo "pub fn placeholder() {}" > core/src/lib.rs && \
    echo "pub fn placeholder() {}" > rust/src/lib.rs && \
    touch rust/examples/adapters/mod.rs && \
    touch rust/examples/adapters/in_memory.rs && \
    touch rust/examples/adapters/env_client.rs && \
    touch rust/examples/adapters/kvs_client.rs && \
    touch rust/examples/adapters/db_client.rs && \
    echo "fn main() {}" > rust/examples/app/src/main.rs && \
    touch rust/examples/app/src/adapters.rs && \
    cargo build --release --features state-engine/logging \
        --manifest-path rust/examples/app/Cargo.toml ; \
    rm -rf core/src rust/src rust/examples/adapters rust/examples/app/src && \
    rm -rf rust/examples/app/target/release/.fingerprint/state-engine-* \
           rust/examples/app/target/release/.fingerprint/state-engine-core-* \
           rust/examples/app/target/release/state-engine-sample \
           rust/examples/app/target/release/deps/state_engine-* \
           rust/examples/app/target/release/deps/state_engine_core-* \
           rust/examples/app/target/release/deps/state_engine_sample-*

COPY core/ ./core/
COPY rust/src/ ./rust/src/
COPY rust/examples/adapters/ ./rust/examples/adapters/
COPY rust/examples/app/src/ ./rust/examples/app/src/
RUN cargo build --release --features state-engine/logging \
    --manifest-path rust/examples/app/Cargo.toml

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /build/rust/examples/app/target/release/state-engine-sample ./
COPY rust/examples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
