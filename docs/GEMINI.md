You are absolutely right, and my apologies. Yes, that is almost certainly the problem. The UI is likely interpreting the ` ```python ` block inside the main ` ```markdown ` block as the *end* of the file, which is why it's cutting off everything after section 6.

Let's try a workaround.

I am going to provide the *entire* `GEMINI.md` file again, but this time I will **not** wrap it in the outer ` ```markdown ... ``` ` code block. I will just start the response *with* the file's content. This should allow the UI to render the whole thing as one continuous Markdown document, including the Python code block in its proper place.

Here is the complete file:

-----

# GEMINI.md: Project "eddi"

This document outlines the design and implementation tasks for a secure, self-contained application launcher. The goal is to make *any* web application (Python, PHP, Dotnet, etc.) accessible *only* as a Tor hidden service, with no TCP ports exposed.

All AI-assisted development (via `gemini cli`, `copilot cli`, `claude cli`) should refer to this document as the single source of truth for goals, constraints, and tasks.

## 1\. Project Metadata

  * **Owner:** marctjones
  * **Project Name:** `eddi`
  * **GitHub Repo:** `https://github.com/marctjones/eddi`

## 2\. Core Mission

To design and build a command-line tool (written in Rust) that:

1.  Is given a command to run a target web application (e.g., `gunicorn ...` or `php-fpm ...`).
2.  Natively embeds the **Arti** (Tor) library and initializes a Tor hidden service.
3.  Creates a **Unix Domain Socket (UDS)** for communication.
4.  Spawns the target web application as a **child process**, configured to *only* listen on that UDS.
5.  Acts as the "bridge," forwarding all HTTP requests from the Arti hidden service to the UDS, and sending responses back.
6.  Manages the entire lifecycle of the child process.

The primary goal is to make it **easy and safe** to take an existing web application and expose it *only* as a Tor hidden service.

## 3\. Development Principles & Methodology

This project will adhere to the following principles to ensure quality, security, and maintainability.

  * **Iterative & Test-Driven Development (TDD):** We will follow TDD practices. The project will be built in small, verifiable parts. We will write unit and integration tests *before* or *along with* new features to minimize "thrashing" and ensure each component works as expected.
  * **Git Workflow:**
      * All new features or bug fixes must be developed in separate branches.
      * Work will be merged into the `main` branch via **Pull Requests (PRs)**.
      * We will commit working, incremental progress frequently with clear messages.
  * **Working Demos:** We will create simple, working demos for each significant architectural feature (e.g., the Arti "hello world", the UDS child process, the final bridge). This is to provide clear, human-verifiable proof that the components are working.
  * **Phased Language Support:** The project will be implemented in phases:
    1.  **Phase 1: Python:** The initial focus is to create a production-grade tool for Python web applications (e.g., Flask, FastAPI via Gunicorn/Uvicorn).
    2.  **Phase 2: PHP & Dotnet:** After Python support is stable, we will add tests and demos for PHP (e.g., `php-fpm`) and Dotnet (Kestrel on UDS).
    3.  **Phase 3: TypeScript:** Support for TypeScript/Node.js backends is a long-term goal, to be addressed only after the first two phases are production-ready.
  * **Language Best Practices:** We will adhere to Rust best practices (`clippy`, `rustfmt`) and standard TDD methodologies. When writing test apps or demos in other languages (Python, PHP), we will use their respective best practices.
  * **Dependency & Licensing:**
      * **Minimize Dependencies:** We will be mindful of not creating "dependency hell" and will only add new dependencies when they provide significant value over rewriting.
      * **Open Source Only:** All dependencies must be open source.
      * **Permissive Licenses Preferred:** We will **strongly prefer** dependencies with permissive licenses (e.g., MIT, Apache 2.0, BSD).
      * **NO COPYLEFT (Default):** Copyleft licenses (GPL, LGPL, AGPL) **must be avoided**. If a copyleft-licensed library is deemed *absolutely critical* and no alternative exists, work must pause until you give explicit approval.
  * **Guard Against Scope Creep:**
      * We will remain focused on the **Core Mission**. The tool does one thing: bridge an Arti hidden service to a UDS child process.
      * Features like building a web server *into* the tool, complex configuration file management, or anything not directly related to this bridge are **out of scope**.
      * Out-of-scope ideas will be noted in the Appendix for future consideration.

## 4\. Core Architectural Constraints

  * **No TCP Exposure:** The application must *never* bind to or listen on a TCP port.
  * **Embedded Arti:** We must use the `arti` Rust library.
  * **Process Separation:** The host Rust app and the target web app run as separate processes, communicating *only* via the UDS.
  * **Language Agnostic:** The Rust host should not care what language the target application is written in.

## 5\. Technology Stack

  * **Host/Bridge Language:** Rust
  * **Tor Implementation:** `arti` (Rust crate)
  * **IPC Mechanism:** Unix Domain Sockets
  * **Test Web Apps:**
      * 1.  Python (using Flask + Gunicorn)
      * 2.  (Optional) PHP (using `php-fpm`)

## 6\. Phase 1: Test Python Web Application

This is our first test target. We need a standard WSGI app and a server that can bind to a UDS.

  * **Task:** Write a minimal Flask app.
  * **Task:** Identify the `gunicorn` command to run this app, bound to a UDS.

<!-- end list -->

```python
# app.py (Example using Flask)
from flask import Flask
app = Flask(__name__)

@app.route('/')
def hello_world():
    return 'Hello, this is a secure hidden service!'

# We will run this from the command line, NOT from Rust.
# Example command:
# gunicorn --workers 1 --bind unix:/tmp/project.sock app:app
```

## 7\. Phase 2: Core Design (Finalized)

  * **Architecture:** **Rust-Led** (Confirmed).
  * **IPC Mechanism:** **Unix Domain Socket** (Confirmed).

### New Design Tasks:

1.  **Rust Host: Arti Setup:**
      * Use `arti-client` and `arti_client::TorClient` to bootstrap a connection.
      * Use the `HsClientService` (or equivalent `arti` API) to launch a v3 onion service.
2.  **Rust Host: UDS & Process Management:**
      * Create a secure UDS.
      * Use `std::process::Command` to launch the user-provided command.
      * Monitor the child process.
3.  **Rust Host: The Bridge Logic:**
      * This is the core loop: `(Arti Request) -> (UDS Request) -> (UDS Response) -> (Arti Response)`.

## 8\. Phase 3: Security & Logging

  * **Logging:** Use the `tracing` crate in Rust. Capture `stdout`/`stderr` from the child process.
  * **Filesystem Security:** UDS must have strict file permissions (e.g., `0o600`).
  * **Signal Handling:** The Rust host must correctly handle `SIGINT`/`SIGTERM`.

## 9\. Next Steps (Task List for AI)

1.  [X] **Task:** Write the minimal Flask app (`app.py`) and the exact `gunicorn` command needed to run it on a UDS. *(This is complete)*
2.  [ ] **Task:** Create a "Hello World" Rust project that uses `arti` to start a basic hidden service (that just returns "hello" from Rust). This verifies our Arti setup.
3.  [ ] **Task:** Create a separate "Hello World" Rust project that spawns `gunicorn` as a child process, bound to a UDS. The Rust app will then manually connect to that UDS, send a hardcoded `GET /` request, and print the response. This verifies the UDS/process logic.
4.  [ ] **Task:** (The Big One) Combine tasks 2 and 3 into the final "Arti-to-UDS Bridge."

## 10\. Appendix: Alternative Architectures (Future Exploration)

This section documents alternative designs that were discussed. They are considered **out of scope** for the current project.

  * **A-1: In-Memory (pyo3) Bridge:**
      * **Concept:** A single Rust binary that embeds a Python interpreter using `pyo3`.
      * **Mechanism:** Instead of a socket, Rust would translate an `arti` request into a WSGI/ASGI "environment" object *in memory* and directly call the Python `app` function.
      * **Pros:** True socketless design, potentially very high performance.
      * **Cons:** Highly complex and **Python-specific**.
  * **A-2: Python-Led (Rust Extension) Bridge:**
      * **Concept:** A Python script imports a Rust extension (`.so` file) that contains the `arti` logic.
      * **Cons:** Less secure, as Python manages the environment.

-----

Please let me know if that renders correctly for you. This should give you the full, correct file from section 1 to 10.

If this looks good, we are ready for **Task 2**: Creating the "Hello World" `arti` hidden service in Rust.
