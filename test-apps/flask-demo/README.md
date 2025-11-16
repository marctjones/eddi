# Flask Demo App - Task 1

This is the test web application specified in GEMINI.md section 6.

## Setup

```bash
cd test-apps/flask-demo
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

## Running Manually (for testing)

```bash
# Start gunicorn bound to a Unix Domain Socket
gunicorn --workers 1 --bind unix:/tmp/eddi-test.sock app:app
```

## Testing the Socket

```bash
# In another terminal, use curl with --unix-socket
curl --unix-socket /tmp/eddi-test.sock http://localhost/
curl --unix-socket /tmp/eddi-test.sock http://localhost/status
```

## Integration with eddi

The eddi Rust application will:
1. Create the UDS at a specified path
2. Spawn gunicorn with the command: `gunicorn --workers 1 --bind unix:<path> app:app`
3. Forward Arti onion service requests to this socket
4. Manage the gunicorn process lifecycle

## Security Verification

When running under eddi, the application should:
- Only listen on the Unix Domain Socket
- Have NO TCP ports open
- Have NO UDP ports open
- Be isolated from the network entirely
