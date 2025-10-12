# Multi-stage Dockerfile for XZEPR server using Red Hat UBI 9
# Stage 1: Build environment
FROM registry.redhat.io/ubi9/ubi:9.6 AS builder

# Install build dependencies
RUN dnf update -y && \
    dnf install -y \
        gcc \
        gcc-c++ \
        make \
        cmake \
        pkg-config \
        openssl-devel \
        postgresql-devel \
        curl \
        git && \
    dnf clean all

# Install Rust toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
ENV PATH="/root/.cargo/bin:${PATH}"

# Set working directory
WORKDIR /build

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn main() {}" > src/lib.rs

# Build dependencies only
RUN cargo build --release && \
    rm -rf src target/release/deps/xzepr*

# Copy source code
COPY src ./src
COPY benches ./benches
COPY migrations ./migrations
COPY config ./config

# Build the actual application
RUN cargo build --release --bin xzepr

# Stage 2: Runtime environment
FROM registry.redhat.io/ubi9/ubi-minimal:9.6

# Install runtime dependencies
RUN microdnf update -y && \
    microdnf install -y \
        ca-certificates \
        postgresql \
        shadow-utils && \
    microdnf clean all

# Create non-root user for security
RUN groupadd -r xzepr && \
    useradd -r -g xzepr -s /bin/bash -c "XZEPR Server" xzepr

# Set working directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /build/target/release/xzepr /app/xzepr

# Copy configuration files
COPY --from=builder /build/config /app/config

# Create directories for certificates and logs
RUN mkdir -p /app/certs /app/logs && \
    chown -R xzepr:xzepr /app

# Copy admin binary as well (optional)
COPY --from=builder /build/target/release/admin /app/admin

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
    maintainer="XZEPR Team" \
    io.openshift.expose-services="8443:https" \
    io.k8s.description="XZEPR Event Processing Server" \
    io.k8s.display-name="XZEPR Server" \
    io.openshift.tags="rust,event-processing,graphql,rest-api"

# Default command
CMD ["./xzepr"]
