# Multi-stage build for Better Auth Server
# Stage 1: Builder
FROM rust:1.83-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the server binary
RUN cargo build --release -p better_auth_server

# Stage 2: Runtime
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 authuser

# Set working directory
WORKDIR /app

# Copy the binary from builder
COPY --from=builder /app/target/release/better-auth-server /usr/local/bin/better-auth-server

# Create directories for config and data
RUN mkdir -p /app/config /app/data && \
    chown -R authuser:authuser /app

# Switch to non-root user
USER authuser

# Expose default port
EXPOSE 8080

# Set environment variables with defaults
ENV RUST_LOG=info
ENV AUTH_PORT=8080
ENV AUTH_HOST=0.0.0.0

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD [ "sh", "-c", "curl -f http://localhost:${AUTH_PORT}/health || exit 1" ]

# Run the server
ENTRYPOINT ["better-auth-server"]
