FROM rust:1-slim AS builder
WORKDIR /build

COPY Cargo.toml ./Cargo.toml
COPY core/Cargo.toml ./core/Cargo.toml
COPY crate/Cargo.toml ./crate/Cargo.toml
COPY wasi/Cargo.toml ./wasi/Cargo.toml
COPY js/Cargo.toml ./js/Cargo.toml
COPY php/Cargo.toml ./php/Cargo.toml
COPY crate/examples/app/Cargo.toml ./crate/examples/app/Cargo.toml

RUN mkdir -p core/src crate/src wasi/src js/src php/src \
             crate/examples/adapters crate/examples/app/src && \
    echo "pub fn placeholder() {}" > core/src/lib.rs && \
    echo "pub fn placeholder() {}" > crate/src/lib.rs && \
    echo "pub fn placeholder() {}" > wasi/src/lib.rs && \
    echo "pub fn placeholder() {}" > js/src/lib.rs && \
    echo "pub fn placeholder() {}" > php/src/lib.rs && \
    touch crate/examples/adapters/mod.rs && \
    touch crate/examples/adapters/in_memory.rs && \
    touch crate/examples/adapters/env_client.rs && \
    touch crate/examples/adapters/kvs_client.rs && \
    touch crate/examples/adapters/db_client.rs && \
    echo "fn main() {}" > crate/examples/app/src/main.rs && \
    touch crate/examples/app/src/adapters.rs && \
    cargo build --release --features state-engine/logging \
        --manifest-path crate/examples/app/Cargo.toml ; \
    rm -rf core/src crate/src wasi/src js/src php/src \
           crate/examples/adapters crate/examples/app/src && \
    rm -rf target/release/.fingerprint/state-engine-* \
           target/release/.fingerprint/core-* \
           target/release/state-engine-sample \
           target/release/deps/state_engine-* \
           target/release/deps/core-* \
           target/release/deps/state_engine_sample-*

COPY core/ ./core/
COPY crate/src/ ./crate/src/
COPY crate/examples/adapters/ ./crate/examples/adapters/
COPY crate/examples/app/src/ ./crate/examples/app/src/
RUN cargo build --release --features state-engine/logging \
    --manifest-path crate/examples/app/Cargo.toml

FROM debian:bookworm-slim

WORKDIR /app

COPY --from=builder /build/target/release/state-engine-sample ./
COPY crate/examples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
