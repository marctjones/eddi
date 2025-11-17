# Security Guide

This document outlines security considerations and best practices for deploying and operating eddi in production.

## Table of Contents

- [Security Model](#security-model)
- [Threat Model](#threat-model)
- [Network Isolation](#network-isolation)
- [System Hardening](#system-hardening)
- [Application Security](#application-security)
- [Tor Security](#tor-security)
- [Operational Security](#operational-security)
- [Incident Response](#incident-response)
- [Security Checklist](#security-checklist)

## Security Model

### What eddi Protects Against

✅ **Network exposure**: No TCP/UDP ports exposed, only Unix Domain Socket
✅ **Clearnet access**: Application accessible ONLY via Tor network
✅ **Direct IP disclosure**: No IP address exposed to visitors
✅ **Port scanning**: No listening ports to discover
✅ **DDoS (partially)**: Tor network provides some inherent DDoS protection

### What eddi Does NOT Protect Against

❌ **Application vulnerabilities**: XSS, SQL injection, etc. in your web app
❌ **Traffic analysis**: Advanced correlation attacks on Tor
❌ **Physical server access**: Attacker with root access can compromise everything
❌ **Social engineering**: Phishing, credential theft, etc.
❌ **Supply chain attacks**: Compromised dependencies

## Threat Model

### Adversary Capabilities

**Level 1: Script Kiddie**
- Port scanning
- Automated vulnerability scanning
- Basic exploitation attempts

**Mitigation**: eddi's network isolation defeats most Level 1 attacks.

**Level 2: Skilled Attacker**
- Application-layer exploits
- Advanced reconnaissance
- Targeted attacks on web framework

**Mitigation**: Requires defense-in-depth (see below).

**Level 3: Nation-State Actor**
- Traffic correlation
- Tor network surveillance
- Zero-day exploits
- Physical access

**Mitigation**: Beyond scope of eddi. Consider additional security layers.

## Network Isolation

### Unix Domain Socket Security

**Verify no network ports are exposed:**

```bash
# Run network isolation tests
cargo test -- --ignored

# Manual verification - check for listening TCP ports
sudo ss -tulpn | grep eddi
# Should return nothing

# Check for listening UDP ports
sudo ss -ulpn | grep eddi
# Should return nothing

# Verify only Unix socket exists
sudo ls -la /tmp/eddi.sock
# Should show: srwxr-xr-x (socket file)
```

**Socket file permissions:**

```bash
# Restrict socket access (only eddi user)
sudo chown eddi:eddi /var/run/eddi/app.sock
sudo chmod 600 /var/run/eddi/app.sock

# Verify
ls -l /var/run/eddi/app.sock
# Should show: srw------- 1 eddi eddi
```

### Firewall Configuration

Even though eddi doesn't expose ports, use a firewall for defense-in-depth:

```bash
# UFW (Ubuntu/Debian)
sudo ufw default deny incoming
sudo ufw default allow outgoing  # Needed for Tor
sudo ufw enable

# iptables
sudo iptables -P INPUT DROP
sudo iptables -P FORWARD DROP
sudo iptables -P OUTPUT ACCEPT
sudo iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
sudo iptables -A INPUT -i lo -j ACCEPT  # Loopback
sudo iptables-save > /etc/iptables/rules.v4
```

## System Hardening

### User and Process Isolation

**Run as dedicated non-root user:**

```bash
# Create restricted user
sudo useradd --system --shell /usr/sbin/nologin eddi

# Never run as root
sudo -u eddi /usr/local/bin/eddi  # Good
sudo /usr/local/bin/eddi           # BAD - don't do this
```

**systemd security features:**

```ini
[Service]
# Prevent privilege escalation
NoNewPrivileges=true

# Filesystem isolation
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/eddi /var/run/eddi

# Capability restrictions
CapabilityBoundingSet=
AmbientCapabilities=

# System call filtering
SystemCallFilter=@system-service
SystemCallErrorNumber=EPERM
SystemCallArchitectures=native

# Namespace isolation
PrivateDevices=true
ProtectKernelModules=true
ProtectKernelTunables=true
ProtectControlGroups=true
```

### File System Security

**Restrict access to sensitive directories:**

```bash
# Arti state (onion service keys!)
sudo chmod 700 /var/lib/eddi
sudo chown eddi:eddi /var/lib/eddi

# Application directory
sudo chmod 755 /opt/eddi/webapp
sudo chown eddi:eddi /opt/eddi/webapp

# No world-readable secrets
sudo find /var/lib/eddi -type f -exec chmod 600 {} \;
```

**SELinux/AppArmor:**

```bash
# SELinux context (RHEL/CentOS)
sudo semanage fcontext -a -t httpd_sys_content_t "/opt/eddi/webapp(/.*)?"
sudo restorecon -R /opt/eddi/webapp

# AppArmor profile (Ubuntu/Debian)
# See deployment/apparmor/eddi.profile
sudo cp deployment/apparmor/eddi.profile /etc/apparmor.d/
sudo apparmor_parser -r /etc/apparmor.d/eddi.profile
```

## Application Security

### Web Application Best Practices

**Your web application should implement:**

1. **Input validation**: Sanitize all user input
2. **Output encoding**: Prevent XSS
3. **CSRF protection**: Use tokens
4. **Content Security Policy**: Restrict resource loading
5. **Rate limiting**: Prevent abuse
6. **Authentication**: If needed, use strong methods
7. **Authorization**: Proper access controls

**Example Flask configuration:**

```python
from flask import Flask
from flask_talisman import Talisman

app = Flask(__name__)

# Security headers
Talisman(app,
    force_https=False,  # We're on Tor, not HTTPS
    content_security_policy={
        'default-src': "'self'",
        'script-src': "'self'",
        'style-src': "'self' 'unsafe-inline'",
    }
)

# Session security
app.config['SESSION_COOKIE_SECURE'] = False  # No HTTPS on Tor
app.config['SESSION_COOKIE_HTTPONLY'] = True
app.config['SESSION_COOKIE_SAMESITE'] = 'Strict'

# Secret key from environment
import os
app.secret_key = os.environ['SECRET_KEY']
```

### Dependency Management

**Keep dependencies updated:**

```bash
# Python
pip list --outdated
pip install --upgrade flask gunicorn

# Rust
cargo update
cargo audit
```

**Use security scanners:**

```bash
# Python
pip install safety
safety check

# Rust
cargo install cargo-audit
cargo audit
```

## Tor Security

### Arti Security Considerations

**Current limitations (as of Arti 0.36):**

⚠️ **No vanguard relay support**: Risk of guard discovery
⚠️ **No DoS protection**: Vulnerable to resource exhaustion
⚠️ **No proof-of-work**: Can't mitigate automated attacks
⚠️ **Experimental status**: Not production-ready per Arti team

**Mitigations:**

1. **Monitor Arti releases**: Update to stable versions when available
2. **Rate limiting**: Implement at application layer
3. **Resource limits**: Use systemd/cgroups
4. **Monitoring**: Watch for unusual traffic patterns

### Onion Service Security

**Protect onion service keys:**

```bash
# Keys stored in Arti state directory
ls /var/lib/eddi/

# Backup and encrypt
tar czf keys-backup.tar.gz /var/lib/eddi/
gpg -c keys-backup.tar.gz
rm keys-backup.tar.gz

# Store encrypted backup off-site
```

**Monitor for key compromise:**

If keys are compromised, attacker can impersonate your service!

**Recovery plan:**
1. Immediately stop eddi
2. Remove compromised state: `rm -rf /var/lib/eddi/`
3. Restart eddi (new onion address will be generated)
4. Communicate new address to users via trusted channel

### Tor Network Attacks

**Timing/correlation attacks:**

Advanced adversaries may correlate:
- Entry guard timing
- Exit node traffic (if connecting to clearnet)
- Application-layer characteristics

**Mitigations:**
- Use Tor Browser for accessing your own service
- Don't reveal identifiable information in responses
- Add random delays to responses (timing obfuscation)
- Use Tor-specific best practices: https://support.torproject.org/

## Operational Security

### Logging

**What to log:**
✅ Service start/stop events
✅ Onion address generation
✅ Connection counts (aggregated)
✅ Error conditions

**What NOT to log:**
❌ IP addresses (Tor provides anonymity)
❌ Request contents
❌ User-identifying information
❌ Detailed timing data

**Log configuration:**

```bash
# Limit log retention
sudo journalctl --vacuum-time=7d
sudo journalctl --vacuum-size=100M

# Redact sensitive data
RUST_LOG=info  # Not 'debug' in production
```

### Monitoring

**Safe metrics to track:**

```bash
# Process health
systemctl is-active eddi

# Resource usage
ps aux | grep eddi

# Connection count (aggregate only)
# Do NOT log individual connections
```

### Update Management

**Establish update procedures:**

```bash
#!/bin/bash
# update-eddi.sh

# Backup keys
sudo tar czf /backup/eddi-$(date +%Y%m%d).tar.gz /var/lib/eddi/

# Update binary
sudo systemctl stop eddi
sudo cp /tmp/eddi-new /usr/local/bin/eddi
sudo systemctl start eddi

# Verify
sleep 5
sudo systemctl status eddi
```

### Incident Detection

**Signs of compromise:**

- Unexpected process crashes
- High CPU/memory usage
- Unknown processes spawned
- Modified binaries (check hashes)
- Suspicious network connections
- Unauthorized file modifications

**Detection tools:**

```bash
# File integrity monitoring
sudo apt install aide
sudo aide --init

# Process monitoring
ps auxf | grep eddi

# Binary integrity
sha256sum /usr/local/bin/eddi
# Compare with known-good hash
```

## Incident Response

### Compromise Response Plan

**If you suspect compromise:**

1. **Isolate**
   ```bash
   sudo systemctl stop eddi
   sudo iptables -P OUTPUT DROP  # Cut network
   ```

2. **Preserve evidence**
   ```bash
   sudo journalctl -u eddi > /forensics/eddi-logs.txt
   sudo tar czf /forensics/eddi-state.tar.gz /var/lib/eddi/
   ```

3. **Investigate**
   - Review logs for suspicious activity
   - Check process tree
   - Examine file modifications
   - Analyze network connections

4. **Recover**
   - Reinstall eddi from trusted source
   - Restore from known-good backup
   - Generate new onion address
   - Notify users

### Vulnerability Disclosure

If you discover a security vulnerability:

1. **Do NOT** disclose publicly
2. Email: security@[your-domain] (create this)
3. Use PGP encryption if possible
4. Provide:
   - Description of vulnerability
   - Steps to reproduce
   - Impact assessment
   - Suggested fix (if any)

## Security Checklist

### Initial Deployment

- [ ] Run as non-root user (eddi)
- [ ] Enable systemd security features
- [ ] Configure firewall (deny all incoming)
- [ ] Set proper file permissions (700 for /var/lib/eddi)
- [ ] Verify no TCP/UDP ports exposed
- [ ] Enable SELinux/AppArmor
- [ ] Configure logging (info level, not debug)
- [ ] Backup onion service keys
- [ ] Test recovery procedures

### Application Security

- [ ] Input validation implemented
- [ ] Output encoding for XSS prevention
- [ ] CSRF protection enabled
- [ ] Content Security Policy configured
- [ ] Rate limiting implemented
- [ ] Session security configured
- [ ] No secrets in code/logs
- [ ] Dependencies audited and updated

### Ongoing Operations

- [ ] Monitor Arti security advisories
- [ ] Update eddi regularly
- [ ] Review logs weekly
- [ ] Test backups monthly
- [ ] Audit dependencies quarterly
- [ ] Update incident response plan
- [ ] Security training for operators

### Before Production

- [ ] Security review completed
- [ ] Penetration testing performed
- [ ] Incident response plan documented
- [ ] Monitoring and alerting configured
- [ ] Backup and recovery tested
- [ ] Documentation reviewed
- [ ] Stakeholders informed of risks

## Additional Resources

- **Tor Security**: https://support.torproject.org/
- **Arti Documentation**: https://gitlab.torproject.org/tpo/core/arti
- **OWASP Top 10**: https://owasp.org/www-project-top-ten/
- **systemd Security**: https://www.freedesktop.org/software/systemd/man/systemd.exec.html
- **Flask Security**: https://flask.palletsprojects.com/en/latest/security/

## Reporting Security Issues

**For eddi itself:**
- Email: [Create a security@... email]
- GitHub Security Advisory: https://github.com/marctjones/eddi/security/advisories
- PGP Key: [Publish PGP public key]

**For dependencies:**
- Arti: https://gitlab.torproject.org/tpo/core/arti/-/issues
- Rust crates: https://rustsec.org/

## Disclaimer

eddi is provided "as-is" without warranty. The maintainers are not responsible for:
- Security of your web application
- Compromises due to misconfiguration
- Tor network-level attacks
- Advanced persistent threats

Use at your own risk. Assess whether eddi's security model meets your threat model before deploying to production.
