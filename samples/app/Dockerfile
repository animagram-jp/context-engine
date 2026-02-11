FROM rust:1-slim AS builder

WORKDIR /build

# Copy workspace root files
COPY Cargo.toml Cargo.lock ./

# Copy library source
COPY src/ ./src/

# Copy sample adapters (shared)
COPY samples/adapters/ ./samples/adapters/

# Copy sample app source
COPY samples/app/Cargo.toml ./samples/app/
COPY samples/app/src/ ./samples/app/src/

# Build sample app (release mode)
RUN cargo build --release --manifest-path samples/app/Cargo.toml

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/samples/app/target/release/state-engine-sample ./

# Copy manifest files (runtime data)
COPY samples/manifest/ ./manifest/

CMD ["./state-engine-sample"]
