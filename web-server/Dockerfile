# Multi-stage build: Rust MCP binary + Node.js server

# Stage 1: Build Rust MCP binaries
FROM rust:1.85-bookworm AS rust-builder

WORKDIR /app

# Copy workspace manifests
COPY Cargo.toml Cargo.lock* ./
COPY crates ./crates

# Build the unified MCP binary
RUN cargo build --release -p cortex-unified-mcp

# Stage 2: Node.js runtime with MCP binary
FROM node:22-bookworm-slim

# Install runtime deps for Rust binary
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy MCP binary from rust builder
COPY --from=rust-builder /app/target/release/cortex-mcp /app/cortex-mcp

# Copy Node.js server
COPY web-server/package*.json ./
RUN npm ci --only=production

COPY web-server/server.js ./

# Copy dashboard static files
COPY dashboard ./dashboard

# Update server to use the local paths
ENV MCP_BINARY_PATH=/app/cortex-mcp
ENV DASHBOARD_PATH=/app/dashboard
ENV PORT=8080
ENV NODE_ENV=production

EXPOSE 8080

CMD ["node", "server.js"]
