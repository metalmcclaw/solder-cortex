# Build stage
FROM rust:latest as builder

WORKDIR /app

# Copy workspace manifests
COPY Cargo.toml Cargo.lock* ./
COPY crates/cortex-server/Cargo.toml ./crates/cortex-server/
COPY crates/cortex-mcp/Cargo.toml ./crates/cortex-mcp/
COPY crates/cortex-prediction-mcp/Cargo.toml ./crates/cortex-prediction-mcp/

# Create dummy sources to cache dependencies
RUN mkdir -p crates/cortex-server/src && \
    echo "fn main() {}" > crates/cortex-server/src/main.rs && \
    mkdir -p crates/cortex-mcp/src && \
    echo "fn main() {}" > crates/cortex-mcp/src/main.rs && \
    mkdir -p crates/cortex-prediction-mcp/src && \
    echo "fn main() {}" > crates/cortex-prediction-mcp/src/main.rs && \
    cargo build --release && \
    rm -rf crates/cortex-server/src crates/cortex-mcp/src crates/cortex-prediction-mcp/src

# Copy actual source
COPY crates ./crates
COPY config ./config
COPY migrations ./migrations

# Build the actual binaries
RUN touch crates/cortex-server/src/main.rs crates/cortex-mcp/src/main.rs crates/cortex-prediction-mcp/src/main.rs && \
    cargo build --release

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
