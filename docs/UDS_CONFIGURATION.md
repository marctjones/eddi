# Unix Domain Socket Configuration Guide

This guide explains how to configure eddi to serve web applications over Tor using Unix Domain Sockets (UDS).

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Serving a UDS Application Over Tor](#serving-a-uds-application-over-tor)
- [Onion Address Management](#onion-address-management)
  - [Automatically Generate a New Onion Address](#automatically-generate-a-new-onion-address)
  - [Reuse an Existing Onion Address](#reuse-an-existing-onion-address)
  - [Import a User-Provided Onion Address](#import-a-user-provided-onion-address)
- [Managing Multiple eddi Instances](#managing-multiple-eddi-instances)
- [Testing Connections](#testing-connections)
- [Supported Web Servers](#supported-web-servers)
- [Command Reference](#command-reference)
- [Troubleshooting](#troubleshooting)

---

## Overview

eddi allows you to expose any web application as a Tor hidden service (.onion address) using Unix Domain Sockets for inter-process communication. This provides:

- **Complete network isolation**: No TCP ports exposed on your system
- **Tor-only access**: Your application is only accessible via Tor
- **Persistent onion addresses**: Reuse the same .onion address across restarts
- **Multi-instance support**: Run multiple eddi servers with different onion addresses simultaneously

### Architecture

```
┌─────────────────┐
│   Tor Network   │
│   (.onion)      │
└────────┬────────┘
         │
    ┌────▼─────┐
    │   eddi   │
    │  Server  │
    └────┬─────┘
         │
    Unix Domain Socket
    (/tmp/app.sock)
         │
    ┌────▼─────────────┐
    │  Web Application │
    │ (gunicorn, nginx,│
    │  uvicorn, etc.)  │
    └──────────────────┘
```

---

## Quick Start

### 1. Start with Default Settings (Flask Demo)

```bash
./eddi-server
```

This will:
- Spawn a Flask demo application
- Create a new onion service
- Save keys to `~/.eddi/onion-services/eddi-demo/`
- Listen on `/tmp/eddi.sock`

### 2. Test the Connection

In another terminal:

```bash
# Replace with your .onion address from the server output
./eddi-connect http://your-address.onion
```

---

## Serving a UDS Application Over Tor

### Scenario 1: Let eddi Spawn Your Application (Gunicorn)

If you want eddi to automatically spawn and manage your web application using Gunicorn:

```bash
./eddi-server \
  --socket /tmp/myapp.sock \
  --nickname my-blog \
  --app-dir /path/to/your/app \
  --app-module app:application \
  --workers 4
```

**Parameters:**
- `--socket`: Path where Gunicorn will create the Unix socket
- `--nickname`: Unique identifier for this onion service
- `--app-dir`: Directory containing your web application
- `--app-module`: Python module in format `module:app` (e.g., `app:app` or `wsgi:application`)
- `--workers`: Number of Gunicorn worker processes

### Scenario 2: Connect to an Already Running Application

If your web application is already running and listening on a Unix socket:

```bash
# First, start your web application
gunicorn --bind unix:/var/run/myapp.sock myapp:app

# In another terminal, start eddi
./eddi-server \
  --socket /var/run/myapp.sock \
  --nickname my-app \
  --no-spawn
```

**Important:** Use the `--no-spawn` flag to tell eddi not to spawn a child process.

### Scenario 3: Using with nginx

First, configure nginx to listen on a Unix socket:

```nginx
# /etc/nginx/sites-available/myapp
upstream app {
    server unix:/var/run/myapp.sock;
}

server {
    listen unix:/var/run/nginx-myapp.sock;

    location / {
        proxy_pass http://app;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

Start nginx, then run eddi:

```bash
./eddi-server \
  --socket /var/run/nginx-myapp.sock \
  --nickname my-nginx-app \
  --no-spawn
```

---

## Onion Address Management

### Automatically Generate a New Onion Address

Each unique `--nickname` gets its own onion address:

```bash
./eddi-server --nickname my-blog
```

Keys are saved to `~/.eddi/onion-services/my-blog/`

The first time you use a nickname, a new .onion address is generated. On subsequent runs with the same nickname, the same address is reused.

### Reuse an Existing Onion Address

eddi automatically reuses onion addresses based on the nickname:

```bash
# First run - generates new address
./eddi-server --nickname my-blog
# Onion address: abc123...xyz.onion

# Second run - reuses same address
./eddi-server --nickname my-blog
# Onion address: abc123...xyz.onion (same as before)
```

**Key Storage Location:**

By default, keys are stored in:
```
~/.eddi/onion-services/<nickname>/
```

You can specify a custom key directory:

```bash
./eddi-server \
  --nickname my-blog \
  --key-dir /opt/eddi/keys
```

Keys will be stored in: `/opt/eddi/keys/my-blog/`

### Import a User-Provided Onion Address

If you have an existing Tor hidden service and want to use its keys with eddi:

```bash
# Copy your existing Tor keys to a directory
# Keys needed: hostname, hs_ed25519_public_key, hs_ed25519_secret_key

./eddi-server \
  --nickname my-imported-service \
  --import-keys /path/to/existing/tor/keys
```

This copies the keys to eddi's key storage and uses them for the onion service.

---

## Managing Multiple eddi Instances

You can run multiple eddi instances simultaneously, each serving a different application on a different onion address.

### Example: Running Three Services

**Terminal 1: Blog**
```bash
./eddi-server \
  --nickname blog \
  --socket /tmp/blog.sock \
  --app-dir ~/my-blog \
  --app-module app:app
```

**Terminal 2: API Server**
```bash
./eddi-server \
  --nickname api \
  --socket /tmp/api.sock \
  --app-dir ~/my-api \
  --app-module server:application
```

**Terminal 3: File Server**
```bash
./eddi-server \
  --nickname files \
  --socket /tmp/files.sock \
  --no-spawn
```

Each instance:
- Has its own unique .onion address
- Uses a different Unix socket
- Stores keys in a separate directory
- Runs as an independent process

**Key Management:**
```
~/.eddi/onion-services/
├── blog/          # blog's onion address keys
├── api/           # api's onion address keys
└── files/         # files' onion address keys
```

---

## Testing Connections

### Test Your eddi Server

Use the `eddi-connect` tool to test connections to your onion service:

```bash
# Basic connection
./eddi-connect http://your-address.onion

# Test a specific path
./eddi-connect http://your-address.onion/status

# Quiet mode (only show response)
./eddi-connect --quiet http://your-address.onion

# Verbose mode
./eddi-connect --verbose http://your-address.onion

# Show only headers
./eddi-connect --headers-only http://your-address.onion
```

### Test Generic Onion Services

You can also use `eddi-connect` to test any onion service:

```bash
# Test another onion service
./eddi-connect http://thehiddenwiki.onion

# Test with timeout
./eddi-connect --timeout 60 http://slow-onion-site.onion

# Test clearnet sites via Tor (anonymized)
./eddi-connect https://check.torproject.org
```

### Connection Test Options

```bash
./eddi-connect --help
```

Available options:
- `-q, --quiet`: Only show response body
- `-v, --verbose`: Show detailed connection info
- `-H, --headers-only`: Show only HTTP headers
- `-t, --timeout SECS`: Connection timeout (default: 30)
- `-l, --max-body-size BYTES`: Maximum response size

---

## Supported Web Servers

eddi works with any web server that supports Unix Domain Sockets:

### Gunicorn (Python - WSGI)

**Manual start:**
```bash
gunicorn --bind unix:/tmp/app.sock myapp:app
./eddi-server --socket /tmp/app.sock --no-spawn --nickname myapp
```

**Let eddi spawn:**
```bash
./eddi-server \
  --socket /tmp/app.sock \
  --nickname myapp \
  --app-dir /path/to/app \
  --app-module myapp:app
```

### Uvicorn (Python - ASGI)

```bash
uvicorn --uds /tmp/app.sock myapp:app
./eddi-server --socket /tmp/app.sock --no-spawn --nickname myapp
```

### nginx

Configure nginx to listen on a Unix socket, then:

```bash
./eddi-server --socket /var/run/nginx.sock --no-spawn --nickname nginx-app
```

### Node.js (Express)

```javascript
// server.js
const app = require('express')();
const fs = require('fs');

const SOCKET_PATH = '/tmp/node-app.sock';

// Remove old socket if it exists
if (fs.existsSync(SOCKET_PATH)) {
    fs.unlinkSync(SOCKET_PATH);
}

app.get('/', (req, res) => res.send('Hello from Node!'));

app.listen(SOCKET_PATH, () => {
    console.log(`Listening on ${SOCKET_PATH}`);
});
```

```bash
node server.js &
./eddi-server --socket /tmp/node-app.sock --no-spawn --nickname node-app
```

### Go HTTP Server

```go
package main

import (
    "fmt"
    "net"
    "net/http"
    "os"
)

func main() {
    socketPath := "/tmp/go-app.sock"

    // Remove old socket
    os.Remove(socketPath)

    listener, err := net.Listen("unix", socketPath)
    if err != nil {
        panic(err)
    }

    http.HandleFunc("/", func(w http.ResponseWriter, r *http.Request) {
        fmt.Fprintf(w, "Hello from Go!")
    })

    http.Serve(listener, nil)
}
```

```bash
go run server.go &
./eddi-server --socket /tmp/go-app.sock --no-spawn --nickname go-app
```

---

## Command Reference

### eddi Server Options

```bash
eddi --help
```

**Core Options:**
- `-s, --socket PATH`: Unix Domain Socket path (default: `/tmp/eddi.sock`)
- `-n, --nickname NAME`: Onion service nickname (default: `eddi-demo`)
- `-d, --app-dir PATH`: Web application directory (required if spawning)
- `-m, --app-module MODULE`: WSGI/ASGI module (default: `app:app`)
- `-w, --workers NUM`: Number of Gunicorn workers (default: 2)
- `-k, --key-dir PATH`: Key storage directory (default: `~/.eddi/onion-services`)
- `--no-spawn`: Don't spawn app (assume it's running)
- `--import-keys PATH`: Import existing onion service keys
- `--test-connection BOOL`: Test UDS connection before starting (default: true)

**Wrapper Script Options:**

```bash
./eddi-server --help-launcher
```

- `--force, -f`: Kill existing processes and clean locks
- `--help-launcher`: Show launcher help

### eddi-connect (tor-http-client) Options

```bash
./eddi-connect --help
```

**Options:**
- `URL`: URL to fetch (required)
- `-H, --headers-only`: Show only response headers
- `-l, --max-body-size BYTES`: Maximum response body size (default: 1MB)
- `-t, --timeout SECS`: Connection timeout (default: 30)
- `-v, --verbose`: Show detailed connection information
- `-q, --quiet`: Quiet mode - only show response body

---

## Troubleshooting

### eddi Can't Connect to Unix Socket

**Symptom:** Error message: "Unix socket file does not exist" or "Failed to connect to Unix socket"

**Solutions:**

1. **Verify socket exists:**
   ```bash
   ls -la /tmp/eddi.sock  # or your custom socket path
   ```

2. **Check application is running:**
   ```bash
   ps aux | grep gunicorn  # or your app server
   ```

3. **Verify socket permissions:**
   ```bash
   chmod 666 /tmp/eddi.sock
   ```

4. **Start application manually:**
   ```bash
   gunicorn --bind unix:/tmp/eddi.sock myapp:app
   ```

### Multiple eddi Instances Conflict

**Symptom:** Error about existing processes or lock files

**Solutions:**

1. **Use unique nicknames:**
   ```bash
   ./eddi-server --nickname instance1
   ./eddi-server --nickname instance2
   ```

2. **Use unique socket paths:**
   ```bash
   ./eddi-server --socket /tmp/app1.sock --nickname app1
   ./eddi-server --socket /tmp/app2.sock --nickname app2
   ```

3. **Clean up existing processes:**
   ```bash
   ./eddi-cleanup
   # or
   ./eddi-server --force
   ```

### Onion Address Changed Unexpectedly

**Symptom:** Different .onion address on restart

**Cause:** Changed nickname or deleted key directory

**Solution:**

1. **Always use the same nickname:**
   ```bash
   ./eddi-server --nickname my-stable-service
   ```

2. **Backup your keys:**
   ```bash
   cp -r ~/.eddi/onion-services ~/eddi-keys-backup
   ```

3. **Specify consistent key directory:**
   ```bash
   ./eddi-server --key-dir /opt/eddi-keys --nickname myapp
   ```

### Cannot Connect to Onion Address

**Symptom:** Connection timeout or "unable to connect"

**Solutions:**

1. **Wait for onion service to be fully reachable** (can take 1-2 minutes):
   ```
   Look for: "✓ Onion service is fully reachable!"
   ```

2. **Verify Tor is working:**
   ```bash
   ./eddi-connect https://check.torproject.org
   ```

3. **Check UDS connection is working:**
   - Look for: "✓ Unix Domain Socket is accessible and working"

4. **Test locally first:**
   ```bash
   # In one terminal
   ./eddi-server --nickname test

   # In another terminal (use the .onion address from server output)
   ./eddi-connect http://your-address.onion
   ```

### Permission Denied on Socket

**Symptom:** Error: "Permission denied" when connecting to socket

**Solutions:**

1. **Fix socket permissions:**
   ```bash
   chmod 666 /tmp/your-app.sock
   ```

2. **Run eddi as same user as web application**

3. **Use a socket path both processes can access:**
   ```bash
   ./eddi-server --socket /tmp/eddi.sock
   ```

---

## Advanced Configuration

### Custom Key Storage Location

Store keys in a custom location (useful for backups, shared storage, etc.):

```bash
./eddi-server \
  --nickname production-app \
  --key-dir /mnt/secure-storage/eddi-keys
```

Keys will be stored in: `/mnt/secure-storage/eddi-keys/production-app/`

### Disable Connection Testing

Skip the UDS connection test (useful if socket is created after eddi starts):

```bash
./eddi-server \
  --socket /tmp/app.sock \
  --no-spawn \
  --test-connection=false
```

### Production Deployment

For production deployments, see:
- [DEPLOYMENT.md](DEPLOYMENT.md) - Docker, systemd, and production configurations
- [SECURITY.md](SECURITY.md) - Security model and best practices

---

## Examples

### Example 1: Simple Flask App

```bash
# Start eddi with Flask app
./eddi-server \
  --nickname my-flask-blog \
  --socket /tmp/blog.sock \
  --app-dir ~/my-blog \
  --app-module app:app \
  --workers 4

# Test it
./eddi-connect http://your-address.onion
```

### Example 2: Multiple Services

```bash
# Terminal 1: Blog
./eddi-server --nickname blog --socket /tmp/blog.sock \
  --app-dir ~/blog --app-module app:app

# Terminal 2: API
./eddi-server --nickname api --socket /tmp/api.sock \
  --app-dir ~/api --app-module api:application

# Terminal 3: Admin Panel
./eddi-server --nickname admin --socket /tmp/admin.sock \
  --app-dir ~/admin --app-module admin:app
```

### Example 3: Existing nginx Server

```bash
# nginx already running on unix:/var/run/nginx.sock
./eddi-server \
  --nickname my-nginx-site \
  --socket /var/run/nginx.sock \
  --no-spawn
```

### Example 4: Reusing Existing Onion Address

```bash
# Import keys from existing Tor hidden service
./eddi-server \
  --nickname imported-service \
  --import-keys /var/lib/tor/hidden_service \
  --socket /tmp/app.sock \
  --no-spawn
```

---

## Security Considerations

1. **Unix Socket Permissions**: Ensure socket files have appropriate permissions
2. **Key Storage Security**: Protect `~/.eddi/onion-services/` directory
3. **Process Isolation**: Run eddi with minimal privileges
4. **Application Security**: Secure your web application (CSRF, XSS, etc.)

For detailed security information, see [SECURITY.md](SECURITY.md).

---

## Next Steps

- Read [DEPLOYMENT.md](DEPLOYMENT.md) for production deployment options
- Read [SECURITY.md](SECURITY.md) for security best practices
- Read [TESTING.md](TESTING.md) for running the test suite
- Check [README.md](../README.md) for general project overview
