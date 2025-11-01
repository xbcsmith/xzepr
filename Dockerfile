# Multi-stage Dockerfile for XZEPR server using Debian
# Stage 1: Build environment
FROM debian:bookworm AS builder

USER root

# Install Rust and build dependencies
RUN apt-get update && \
    apt-get install -y \
        pkg-config \
        libssl-dev \
        libpq-dev \
        cmake \
        build-essential \
        curl \
        git \
        ca-certificates && \
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
    rm -rf /var/lib/apt/lists/*

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Install sqlx-cli with only postgres support (smaller binary)
RUN cargo install sqlx-cli --no-default-features --features postgres

# Set working directory
WORKDIR /build

# Copy dependency files first for better caching
COPY Cargo.toml ./

# Copy source code
COPY src ./src
COPY migrations ./migrations
COPY config ./config

# Build the actual application
RUN cargo build --release --bin xzepr --bin admin

# Stage 2: Runtime environment
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        ca-certificates \
        libpq5 \
        libssl3 \
        curl && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN groupadd -r xzepr && \
    useradd -r -g xzepr -s /bin/bash -c "XZEPR Server" xzepr

# Set working directory
WORKDIR /app

# Copy the binaries from builder stage
COPY --from=builder /build/target/release/xzepr /app/xzepr
COPY --from=builder /build/target/release/admin /app/admin
COPY --from=builder /root/.cargo/bin/sqlx /app/sqlx

# Copy configuration and migration files
COPY --from=builder /build/config /app/config
COPY --from=builder /build/migrations /app/migrations

# Create directories for certificates and logs
RUN mkdir -p /app/certs /app/logs && \
    chown -R xzepr:xzepr /app

# Switch to non-root user
USER xzepr

# Expose the application port
EXPOSE 8443

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f -k https://localhost:8443/health || exit 1

# Environment variables with defaults
ENV RUST_LOG=info,xzepr=debug
ENV XZEPR__SERVER__HOST=0.0.0.0
ENV XZEPR__SERVER__PORT=8443
ENV XZEPR__SERVER__ENABLE_HTTPS=true
ENV XZEPR__TLS__CERT_PATH=/app/certs/cert.pem
ENV XZEPR__TLS__KEY_PATH=/app/certs/key.pem

# Add labels for metadata
LABEL \
    name="xzepr" \
    description="XZEPR Event Processing Server" \
    version="0.1.0" \
    maintainer="XZEPR Team"

# Default command
CMD ["./xzepr"]
