FROM rust:1-slim AS builder

WORKDIR /build

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy library source
COPY src/ ./src/

# Copy sample adapters (shared)
COPY examples/adapters/ ./examples/adapters/

# Copy sample app source
COPY examples/app/Cargo.toml ./examples/app/
COPY examples/app/src/ ./examples/app/src/

# Build both binaries (release mode, with logging)
RUN cargo build --release --features state-engine/logging --manifest-path examples/app/Cargo.toml

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /build/examples/app/target/release/state-engine-sample ./
COPY --from=builder /build/examples/app/target/release/state-engine-test ./

# Copy manifest files (runtime data)
COPY examples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
