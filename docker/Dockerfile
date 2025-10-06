# Base image with all build dependencies installed in a single layer
FROM rust:1.90-slim AS base
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    pkg-config \
    libssl-dev \
    cmake \
    clang \
    libclang-dev \
    unzip \
    && curl -fsSL https://deb.nodesource.com/setup_22.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*
RUN rustup update stable

# Stage to download and prepare the Kuzu dynamic library
FROM base AS kuzu-downloader
ARG KUZU_VERSION=0.11.2
WORKDIR /kuzu
RUN curl -L -o libkuzu.tar.gz "https://github.com/kuzudb/kuzu/releases/download/v${KUZU_VERSION}/libkuzu-linux-x86_64.tar.gz" && \
    tar -xzf libkuzu.tar.gz && \
    rm libkuzu.tar.gz

# Deps stage to build and cache all workspace dependencies
FROM base AS deps
ENV KUZU_SHARED=1
ENV KUZU_INCLUDE_DIR=/kuzu
ENV KUZU_LIBRARY_DIR=/kuzu
WORKDIR /app

COPY --from=kuzu-downloader /kuzu /kuzu
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY package.json package-lock.json* ./
COPY packages/ ./packages/

RUN npm ci
RUN cargo build --release --workspace

# Builder stage to build the final application binaries
FROM base AS builder
ENV KUZU_SHARED=1
ENV KUZU_INCLUDE_DIR=/kuzu
ENV KUZU_LIBRARY_DIR=/kuzu
ENV LD_LIBRARY_PATH=/kuzu
WORKDIR /app

COPY --from=kuzu-downloader /kuzu /kuzu
COPY --from=deps /app/target ./target
COPY --from=deps /usr/local/cargo /usr/local/cargo
COPY --from=deps /app/node_modules ./node_modules

COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY package.json package-lock.json* ./
COPY packages/ ./packages/

# Generate TypeScript bindings and build frontend
RUN cargo test export_bindings --features no-frontend
RUN npm ci && npm run build --workspace=@gitlab-org/gkg-frontend

# Build the specific binaries
RUN cargo build --release --bin gkg
RUN cargo build --release --bin http-server-deployed
RUN cargo build --release --bin dev-server

# --- Runtime Stages ---

# gkg runtime stage
FROM debian:trixie-slim AS gkg
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/gkg /usr/local/bin/
COPY --from=kuzu-downloader /kuzu/libkuzu.so /usr/lib/

# http-server-deployed runtime stage
FROM debian:trixie-slim AS http-server-deployed
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/http-server-deployed /usr/local/bin/
COPY --from=kuzu-downloader /kuzu/libkuzu.so /usr/lib/

# http-server-desktop runtime stage
FROM debian:trixie-slim AS http-server-desktop
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/dev-server /usr/local/bin/
COPY --from=kuzu-downloader /kuzu/libkuzu.so /usr/lib/
