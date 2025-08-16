# Multi-stage build for production
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /usr/src/mev_relay

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies
RUN cargo build --release

# Remove dummy main.rs and copy source
RUN rm src/main.rs
COPY src ./src

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash mev_relay

# Copy binary from builder stage
COPY --from=builder /usr/src/mev_relay/target/release/mev_relay /usr/local/bin/

# Copy configuration files
COPY config /etc/mev_relay/config

# Set ownership
RUN chown -R mev_relay:mev_relay /usr/local/bin/mev_relay /etc/mev_relay

# Switch to non-root user
USER mev_relay

# Set working directory
WORKDIR /home/mev_relay

# Expose metrics port
EXPOSE 9090

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD /usr/local/bin/mev_relay --health-check || exit 1

# Run the application
ENTRYPOINT ["/usr/local/bin/mev_relay"]
CMD ["--config", "/etc/mev_relay/config/production.toml"] 