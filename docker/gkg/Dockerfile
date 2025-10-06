# --- Deps Stage: Build and cache all workspace dependencies ---
FROM rust:1.82-slim AS deps

RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

RUN rustup update stable

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

RUN . /root/.cargo/env && cargo build --release --workspace

# --- Builder Stage: Build the final application binary ---
FROM rust:1.82-slim AS builder

WORKDIR /app

# Copy cached dependencies from the deps stage
COPY --from=deps /app/target ./target
COPY --from=deps /root/.cargo /root/.cargo
COPY --from=deps /usr/local/cargo /usr/local/cargo

# Copy source code again to build the final binary
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/

# Build the specific binary. This will be fast because dependencies are cached.
RUN . /root/.cargo/env && cargo build --release --bin gkg

# --- Runtime Stage ---
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/gkg /usr/local/bin/