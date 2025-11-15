# app.py (Flask Demo for Task 1)
# This is the test web application that will be exposed via Arti onion service.
# It demonstrates that eddi can work with standard WSGI applications.

from flask import Flask

app = Flask(__name__)

@app.route('/')
def hello_world():
    return 'Hello, this is a secure hidden service!'

@app.route('/status')
def status():
    return {
        'status': 'ok',
        'message': 'Flask app running on Unix Domain Socket',
        'framework': 'Flask + Gunicorn'
    }

# When run directly (not recommended - use gunicorn)
if __name__ == '__main__':
    print("WARNING: Do not run this directly.")
    print("Use: gunicorn --workers 1 --bind unix:/tmp/eddi-test.sock app:app")
