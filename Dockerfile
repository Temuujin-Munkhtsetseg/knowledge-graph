# Multi-stage build for GitLab Knowledge Graph
# This Dockerfile builds both the deployed and desktop server binaries.
FROM rust:1.82-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    libclang-dev \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace configuration
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/
COPY package.json package-lock.json* ./
COPY packages/ packages/

# Generate TypeScript bindings
RUN cargo test export_bindings_

# Install Node dependencies and build frontend
RUN npm ci && npm run build --workspace=@gitlab-org/gkg-frontend

# Build both binaries in release mode
RUN cargo build --release --bin http-server-deployed --bin dev-server

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binaries from builder stage
COPY --from=builder /app/target/release/http-server-deployed /usr/local/bin/
COPY --from=builder /app/target/release/dev-server /usr/local/bin/