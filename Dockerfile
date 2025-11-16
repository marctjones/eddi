# Multi-stage build for eddi

# Stage 1: Build Rust binary
FROM rust:1.75-slim as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy dependency manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src
COPY test-apps ./test-apps

# Build release binary
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        python3 \
        python3-pip \
        python3-venv \
        ca-certificates \
        && \
    rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --shell /bin/bash eddi

# Copy binary from builder
COPY --from=builder /build/target/release/eddi /usr/local/bin/eddi
RUN chmod +x /usr/local/bin/eddi

# Setup application directories
RUN mkdir -p /opt/eddi/webapp /var/lib/eddi && \
    chown -R eddi:eddi /opt/eddi /var/lib/eddi

WORKDIR /opt/eddi

# Copy and install Flask demo app
COPY test-apps/flask-demo/app.py /opt/eddi/webapp/
COPY test-apps/flask-demo/requirements.txt /opt/eddi/webapp/

RUN cd /opt/eddi/webapp && \
    python3 -m venv venv && \
    . venv/bin/activate && \
    pip install --no-cache-dir --upgrade pip && \
    pip install --no-cache-dir -r requirements.txt && \
    chown -R eddi:eddi /opt/eddi/webapp

# Switch to non-root user
USER eddi

# Environment variables
ENV RUST_LOG=info
ENV PATH="/opt/eddi/webapp/venv/bin:${PATH}"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=60s --retries=3 \
    CMD pgrep -x eddi > /dev/null || exit 1

# No ports exposed - Tor only!
EXPOSE 0

# Run eddi
CMD ["/usr/local/bin/eddi"]
