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

# Build sample app (release mode)
RUN cargo build --release --manifest-path examples/app/Cargo.toml

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/examples/app/target/release/state-engine-sample ./

# Copy manifest files (runtime data)
COPY examples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
