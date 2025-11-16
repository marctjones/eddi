# Deployment Guide

This guide covers deploying eddi in production environments.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Installation](#installation)
- [Configuration](#configuration)
- [systemd Service](#systemd-service)
- [Docker Deployment](#docker-deployment)
- [Monitoring](#monitoring)
- [Troubleshooting](#troubleshooting)

## Prerequisites

### System Requirements

- **OS**: Linux (recommended: Ubuntu 20.04+, Debian 11+, or RHEL 8+)
- **Architecture**: x86_64 or aarch64
- **RAM**: Minimum 512MB, recommended 1GB+
- **Disk**: 1GB for Arti state and logs
- **Network**: Outbound internet access for Tor network

### Software Dependencies

- **Rust**: 1.70+ (for building from source)
- **Python**: 3.8+ (if using the Flask demo or Python web apps)
- **gunicorn**: 21.0+ (for WSGI apps)
- **Tor**: Not required (Arti is embedded)

## Installation

### Option 1: Build from Source

```bash
# Clone the repository
git clone https://github.com/marctjones/eddi.git
cd eddi

# Build release binary
cargo build --release

# Install to system
sudo cp target/release/eddi /usr/local/bin/
sudo chmod +x /usr/local/bin/eddi
```

### Option 2: Download Pre-built Binary

```bash
# Download latest release
wget https://github.com/marctjones/eddi/releases/latest/download/eddi-linux-x86_64

# Install
sudo mv eddi-linux-x86_64 /usr/local/bin/eddi
sudo chmod +x /usr/local/bin/eddi
```

## Configuration

### Application Structure

Create a deployment directory:

```bash
sudo mkdir -p /opt/eddi
sudo mkdir -p /opt/eddi/webapp
sudo mkdir -p /var/lib/eddi  # For Arti state
sudo mkdir -p /var/log/eddi  # For logs
```

### Web Application Setup

#### Flask Example

```bash
cd /opt/eddi/webapp

# Create your Flask app
cat > app.py <<'EOF'
from flask import Flask
app = Flask(__name__)

@app.route('/')
def index():
    return 'Hello from Tor!'

@app.route('/status')
def status():
    return {'status': 'ok'}
EOF

# Install dependencies
python3 -m venv venv
source venv/bin/activate
pip install flask gunicorn

# Test locally
gunicorn --workers 2 --bind unix:/tmp/test.sock app:app
```

### Environment Configuration

Create `/etc/eddi/config.env`:

```bash
# Logging
RUST_LOG=info

# Application settings
EDDI_SOCKET_PATH=/var/run/eddi/app.sock
EDDI_APP_DIR=/opt/eddi/webapp
EDDI_APP_MODULE=app:app
EDDI_WORKERS=2

# Arti settings
EDDI_ONION_NICKNAME=my-service
```

## systemd Service

### Create Service File

Create `/etc/systemd/system/eddi.service`:

```ini
[Unit]
Description=eddi - Tor Hidden Service Bridge
Documentation=https://github.com/marctjones/eddi
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=eddi
Group=eddi
WorkingDirectory=/opt/eddi

# Environment
EnvironmentFile=/etc/eddi/config.env
Environment="RUST_LOG=info"

# Process
ExecStart=/usr/local/bin/eddi
Restart=always
RestartSec=10s

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/eddi /var/run/eddi /var/log/eddi
CapabilityBoundingSet=
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM

# Resource limits
LimitNOFILE=4096
MemoryMax=1G
TasksMax=256

# Logging
StandardOutput=journal
StandardError=journal
SyslogIdentifier=eddi

[Install]
WantedBy=multi-user.target
```

### Create User and Directories

```bash
# Create dedicated user
sudo useradd --system --shell /usr/sbin/nologin --home /opt/eddi eddi

# Set permissions
sudo chown -R eddi:eddi /opt/eddi
sudo chown -R eddi:eddi /var/lib/eddi
sudo chown -R eddi:eddi /var/log/eddi

# Create runtime directory
sudo mkdir -p /var/run/eddi
sudo chown eddi:eddi /var/run/eddi
```

### Manage Service

```bash
# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable eddi
sudo systemctl start eddi

# Check status
sudo systemctl status eddi

# View logs
sudo journalctl -u eddi -f

# Restart
sudo systemctl restart eddi

# Stop
sudo systemctl stop eddi
```

## Docker Deployment

### Dockerfile

Create `Dockerfile`:

```dockerfile
FROM rust:1.75-slim as builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY test-apps ./test-apps

RUN cargo build --release

FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y \
        python3 \
        python3-pip \
        python3-venv \
        ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create user
RUN useradd --create-home --shell /bin/bash eddi

# Copy binary
COPY --from=builder /build/target/release/eddi /usr/local/bin/

# Setup directories
RUN mkdir -p /opt/eddi/webapp /var/lib/eddi
WORKDIR /opt/eddi

# Install Python app
COPY test-apps/flask-demo/app.py /opt/eddi/webapp/
COPY test-apps/flask-demo/requirements.txt /opt/eddi/webapp/

RUN cd /opt/eddi/webapp && \
    python3 -m venv venv && \
    . venv/bin/activate && \
    pip install --no-cache-dir -r requirements.txt

# Set ownership
RUN chown -R eddi:eddi /opt/eddi /var/lib/eddi

USER eddi

ENV RUST_LOG=info
EXPOSE 0

CMD ["/usr/local/bin/eddi"]
```

### docker-compose.yml

Create `docker-compose.yml`:

```yaml
version: '3.8'

services:
  eddi:
    build: .
    container_name: eddi
    restart: unless-stopped

    environment:
      - RUST_LOG=info

    volumes:
      # Persist Arti state (onion service keys)
      - eddi-data:/var/lib/eddi

      # Mount your application (optional)
      # - ./my-webapp:/opt/eddi/webapp

    # No ports exposed - only accessible via Tor!
    # ports: []

    security_opt:
      - no-new-privileges:true

    cap_drop:
      - ALL

    read_only: true
    tmpfs:
      - /tmp

    logging:
      driver: "json-file"
      options:
        max-size: "10m"
        max-file: "3"

volumes:
  eddi-data:
```

### Docker Deployment

```bash
# Build
docker-compose build

# Start
docker-compose up -d

# View logs to get .onion address
docker-compose logs -f eddi

# Stop
docker-compose down

# Update and restart
docker-compose pull
docker-compose up -d
```

## Monitoring

### Health Checks

Create a health check script `/opt/eddi/health_check.sh`:

```bash
#!/bin/bash
# Check if the Unix socket exists
if [ ! -S /var/run/eddi/app.sock ]; then
    echo "Socket not found"
    exit 1
fi

# Check if eddi process is running
if ! pgrep -x eddi > /dev/null; then
    echo "eddi process not running"
    exit 1
fi

# Check if gunicorn is running
if ! pgrep -x gunicorn > /dev/null; then
    echo "gunicorn process not running"
    exit 1
fi

echo "Healthy"
exit 0
```

### Logging

View logs in real-time:

```bash
# systemd
sudo journalctl -u eddi -f

# Docker
docker-compose logs -f eddi

# Extract .onion address from logs
sudo journalctl -u eddi | grep "Onion Service Address"
```

### Metrics (Future)

Integrate with Prometheus:

```bash
# Add metrics endpoint (future enhancement)
curl http://localhost:9090/metrics
```

## Troubleshooting

### Common Issues

#### Issue: "Failed to bootstrap Tor client"

**Cause**: No internet connectivity or Tor network unreachable

**Solution**:
```bash
# Check network connectivity
ping 8.8.8.8

# Check DNS
nslookup torproject.org

# Check if Tor directory authorities are reachable
curl -I https://www.torproject.org
```

#### Issue: "Failed to spawn gunicorn"

**Cause**: gunicorn not installed or not in PATH

**Solution**:
```bash
# Install gunicorn
pip install gunicorn

# Or in virtual environment
cd /opt/eddi/webapp
source venv/bin/activate
pip install gunicorn
```

#### Issue: "Socket file was not created"

**Cause**: Application failed to start or permissions issue

**Solution**:
```bash
# Check application logs
sudo journalctl -u eddi -n 50

# Check permissions
ls -l /var/run/eddi/

# Verify app directory exists
ls -la /opt/eddi/webapp/
```

#### Issue: "Onion service not reachable"

**Cause**: Tor network issues or service still propagating

**Solution**:
- Wait 1-2 minutes for onion service to fully propagate
- Check Tor network status: https://status.torproject.org
- Verify Arti can connect to Tor directory authorities

### Debug Mode

Enable verbose logging:

```bash
# systemd
sudo systemctl stop eddi
RUST_LOG=debug /usr/local/bin/eddi

# Docker
docker-compose down
docker-compose run -e RUST_LOG=debug eddi
```

### Performance Tuning

#### Increase Worker Count

For high-traffic services:

```bash
# Edit config
EDDI_WORKERS=4

# Adjust systemd resource limits
sudo systemctl edit eddi
```

Add:
```ini
[Service]
LimitNOFILE=8192
MemoryMax=2G
```

#### Tor Circuit Optimization

Arti will automatically manage circuits. For custom tuning, consider:
- Adjusting `max_concurrent_streams_per_circuit` in Arti config
- Tuning connection pooling in your web application

## Security Considerations

See [SECURITY.md](SECURITY.md) for comprehensive security best practices.

### Key Points

1. **Run as non-root user**: Always use the `eddi` user
2. **Filesystem permissions**: Restrict access to `/var/lib/eddi` (Arti state)
3. **Network isolation**: Verify no TCP ports are exposed
4. **Application security**: Keep your web framework updated
5. **Tor security**: Monitor Tor Project security advisories

## Backup and Recovery

### Backup Onion Service Keys

```bash
# Backup Arti state (includes onion service keys)
sudo tar czf eddi-backup-$(date +%Y%m%d).tar.gz \
    /var/lib/eddi/

# Store securely off-site
```

### Restore

```bash
# Stop service
sudo systemctl stop eddi

# Restore state
sudo tar xzf eddi-backup-20240101.tar.gz -C /

# Restart
sudo systemctl start eddi
```

## Scaling

### Horizontal Scaling

Run multiple eddi instances with load balancing:

1. Use shared Arti state directory (NFS/GlusterFS)
2. Run separate instances with different onion addresses
3. Use Tor-aware load balancer (future enhancement)

### Vertical Scaling

Increase resources on single instance:

```bash
# More workers
EDDI_WORKERS=8

# More memory
MemoryMax=4G

# More file descriptors
LimitNOFILE=16384
```

## Production Checklist

- [ ] eddi installed and running as system service
- [ ] Dedicated non-root user created
- [ ] systemd service configured and enabled
- [ ] Logging configured and working
- [ ] Health checks in place
- [ ] Onion service keys backed up
- [ ] Security hardening applied
- [ ] Monitoring set up
- [ ] Documentation for operators
- [ ] Incident response plan

## Support

- **Documentation**: [TASK4.md](TASK4.md)
- **Security**: [SECURITY.md](SECURITY.md)
- **Issues**: https://github.com/marctjones/eddi/issues
- **Discussions**: https://github.com/marctjones/eddi/discussions
