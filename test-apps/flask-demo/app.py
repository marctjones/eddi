"""
TorPaste - A simple pastebin for the Tor network
Demonstrates a functional web application running as a hidden service via eddi
"""

from flask import Flask, render_template, request, redirect, url_for, abort
import sqlite3
import hashlib
import time
from datetime import datetime
import os

app = Flask(__name__)

# Database setup
DB_PATH = 'pastes.db'

def get_db():
    """Get database connection"""
    db = sqlite3.connect(DB_PATH)
    db.row_factory = sqlite3.Row
    return db

def init_db():
    """Initialize the database"""
    db = get_db()
    db.execute('''
        CREATE TABLE IF NOT EXISTS pastes (
            id TEXT PRIMARY KEY,
            title TEXT,
            content TEXT NOT NULL,
            language TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        )
    ''')
    db.commit()
    db.close()

def generate_paste_id(content):
    """Generate a unique ID for a paste"""
    timestamp = str(time.time())
    hash_input = (content + timestamp).encode('utf-8')
    return hashlib.sha256(hash_input).hexdigest()[:8]

@app.route('/')
def index():
    """Home page with paste creation form and recent pastes"""
    db = get_db()
    recent_pastes = db.execute(
        'SELECT id, title, created_at FROM pastes ORDER BY created_at DESC LIMIT 10'
    ).fetchall()
    db.close()
    return render_template('index.html', recent_pastes=recent_pastes)

@app.route('/create', methods=['POST'])
def create_paste():
    """Create a new paste"""
    content = request.form.get('content', '').strip()
    title = request.form.get('title', 'Untitled').strip()
    language = request.form.get('language', 'text')

    if not content:
        return "Content cannot be empty", 400

    paste_id = generate_paste_id(content)

    db = get_db()
    db.execute(
        'INSERT INTO pastes (id, title, content, language) VALUES (?, ?, ?, ?)',
        (paste_id, title, content, language)
    )
    db.commit()
    db.close()

    return redirect(url_for('view_paste', paste_id=paste_id))

@app.route('/paste/<paste_id>')
def view_paste(paste_id):
    """View a specific paste"""
    db = get_db()
    paste = db.execute(
        'SELECT * FROM pastes WHERE id = ?', (paste_id,)
    ).fetchone()
    db.close()

    if paste is None:
        abort(404)

    return render_template('view.html', paste=paste)

@app.route('/raw/<paste_id>')
def raw_paste(paste_id):
    """View raw paste content"""
    db = get_db()
    paste = db.execute(
        'SELECT content FROM pastes WHERE id = ?', (paste_id,)
    ).fetchone()
    db.close()

    if paste is None:
        abort(404)

    return paste['content'], 200, {'Content-Type': 'text/plain; charset=utf-8'}

@app.route('/about')
def about():
    """About page"""
    return render_template('about.html')

@app.route('/status')
def status():
    """API status endpoint"""
    db = get_db()
    count = db.execute('SELECT COUNT(*) as count FROM pastes').fetchone()['count']
    db.close()

    return {
        'status': 'ok',
        'message': 'TorPaste running on Tor Hidden Service',
        'total_pastes': count,
        'framework': 'Flask + Gunicorn + SQLite'
    }

# Initialize database on startup
if not os.path.exists(DB_PATH):
    init_db()

if __name__ == '__main__':
    print("WARNING: Do not run this directly.")
    print("Use: gunicorn --workers 1 --bind unix:/tmp/eddi.sock app:app")
