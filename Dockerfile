# Build stage
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Copy workspace manifests and all crates
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates

# Build all release binaries
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries from builder
COPY --from=builder /app/target/release/cortex /app/cortex
COPY --from=builder /app/target/release/cortex-mcp /app/cortex-mcp
COPY --from=builder /app/target/release/cortex-prediction-mcp /app/cortex-prediction-mcp

# Copy config files
COPY config ./config
COPY migrations ./migrations

# Expose port
EXPOSE 3000

# Set environment variables
ENV RUST_LOG=cortex=info,tower_http=debug
ENV CORTEX_SERVER_HOST=0.0.0.0
ENV CORTEX_SERVER_PORT=3000

# Run the binary
CMD ["./cortex"]
