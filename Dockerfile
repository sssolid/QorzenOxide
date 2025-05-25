# Dockerfile

# Multi-stage build for optimal image size
FROM rust:1.75-slim as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 qorzen

# Set working directory
WORKDIR /app

# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./

# Create src directory and dummy main to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Copy source code
COPY src ./src
COPY examples ./examples
COPY benches ./benches
COPY tests ./tests

# Build the application
RUN cargo build --release --bin qorzen

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1001 qorzen

# Create necessary directories
RUN mkdir -p /app/data /app/logs /app/config \
    && chown -R qorzen:qorzen /app

# Copy binary from builder stage
COPY --from=builder /app/target/release/qorzen /usr/local/bin/qorzen

# Copy configuration template
COPY config.example.yaml /app/config/config.yaml

# Set ownership
RUN chown qorzen:qorzen /usr/local/bin/qorzen /app/config/config.yaml

# Switch to app user
USER qorzen

# Set working directory
WORKDIR /app

# Environment variables
ENV RUST_LOG=info
ENV QORZEN_CONFIG=/app/config/config.yaml

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD qorzen health --format json || exit 1

# Expose ports (if API is enabled)
EXPOSE 8000 9090

# Default command
CMD ["qorzen", "run", "--headless"]

# Metadata
LABEL maintainer="Your Name <your.email@example.com>"
LABEL version="0.1.0"
LABEL description="Qorzen Core - A modular plugin-based system"
LABEL org.opencontainers.image.source="https://github.com/yourusername/qorzen-core"
LABEL org.opencontainers.image.licenses="MIT OR Apache-2.0"