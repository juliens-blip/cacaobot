# =============================================================================
# Palm Oil Trading Bot - Multi-stage Dockerfile for Railway deployment
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Builder
# -----------------------------------------------------------------------------
FROM rust:1.75-slim-bookworm as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock* ./

# Copy proto files and build.rs first (needed for prost-build)
COPY proto ./proto
COPY build.rs ./build.rs

# Create dummy source structure for dependency caching
RUN mkdir -p src/modules/scraper src/modules/trading src/modules/monitoring src/modules/utils src/bin \
    && echo "fn main() { println!(\"dummy\"); }" > src/main.rs \
    && echo "fn main() {}" > src/bin/test_connection.rs \
    && echo "fn main() {}" > src/bin/backtest.rs \
    && touch src/lib.rs src/config.rs src/error.rs src/bot.rs \
    && echo "pub mod scraper; pub mod trading; pub mod monitoring; pub mod utils;" > src/modules/mod.rs \
    && touch src/modules/scraper/mod.rs src/modules/trading/mod.rs \
    && touch src/modules/monitoring/mod.rs src/modules/utils/mod.rs

# Build dependencies only (cached layer)
RUN cargo build --release 2>/dev/null || true
RUN rm -rf src build.rs

# Copy actual source code
COPY build.rs ./build.rs
COPY src ./src

# Build the application
RUN cargo build --release --bin palm-oil-bot

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim as runtime

# Labels
LABEL maintainer="Palm Oil Bot Team"
LABEL description="Automated trading bot for FCPO Palm Oil CFDs"
LABEL version="0.1.0"

# Install runtime dependencies only
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -m -u 1000 -s /bin/bash botuser

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/palm-oil-bot /usr/local/bin/palm-oil-bot

# Copy test binary if it exists
COPY --from=builder /app/target/release/test-connection /usr/local/bin/test-connection 2>/dev/null || true

# Set ownership
RUN chown -R botuser:botuser /app

# Switch to non-root user for security
USER botuser

# Environment defaults
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Health check - verify process is running
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD pgrep -x palm-oil-bot > /dev/null || exit 1

# Default command
CMD ["palm-oil-bot"]
