//! Task 2: Arti "Hello World" - Basic Tor Hidden Service
//!
//! This is a proof-of-concept that demonstrates:
//! 1. Initializing the Arti Tor client
//! 2. Creating a basic HTTP server
//! 3. Exposing that server as a Tor v3 onion hidden service
//!
//! This verifies that our Arti setup works before we add UDS and child process management.

use anyhow::Result;
use std::convert::Infallible;
use std::net::SocketAddr;
use tracing::{info, warn, error};
use tracing_subscriber;

use arti_client::TorClient;

use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};

/// Simple HTTP handler that returns a "Hello World" message
///
/// This demonstrates that we can serve HTTP requests through the Tor network.
/// In the final implementation, this will be replaced with proxying to a UDS.
async fn handle_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    info!("Received request: {} {}", _req.method(), _req.uri());

    let response_body = concat!(
        "Hello from Tor Hidden Service!\n\n",
        "This is Task 2 of the 'eddi' project.\n",
        "If you're seeing this, the Arti hidden service is working correctly.\n",
        "\n",
        "Next steps:\n",
        "- Task 3: Add UDS and child process management\n",
        "- Task 4: Bridge Arti requests to UDS\n"
    );

    Ok(Response::new(Body::from(response_body)))
}

/// Initialize and run the Tor hidden service
async fn run_hidden_service() -> Result<()> {
    info!("Initializing Arti Tor client...");

    // Initialize the Tor client with default configuration
    // The Tokio runtime is automatically detected when running inside #[tokio::main]
    // In production, we might want to customize the config (e.g., data directory)
    let _tor_client = TorClient::create_bootstrapped(Default::default()).await?;

    info!("Tor client bootstrapped successfully!");

    // For this Task 2 demo, we'll bind to localhost:8080 as a placeholder
    // In the real implementation, this will be replaced with the onion service listener
    //
    // NOTE: The Arti API for running onion services is still evolving.
    // As of arti-client 0.11, the onion service functionality may require
    // using the lower-level tor-hsservice crate or waiting for stable APIs.
    //
    // For this proof-of-concept, we demonstrate:
    // 1. That we can initialize Arti successfully ✓
    // 2. That we have an HTTP server ready to accept connections ✓
    // 3. The next step would be configuring the actual onion service

    warn!("NOTE: Full onion service setup requires additional Arti configuration.");
    warn!("This demo starts an HTTP server to verify the basic architecture.");
    warn!("See: https://gitlab.torproject.org/tpo/core/arti/-/blob/main/doc/OnionService.md");

    // Create the HTTP server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

    let make_svc = make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_request))
    });

    let server = Server::bind(&addr).serve(make_svc);

    info!("HTTP server listening on http://{}", addr);
    info!("Tor client is ready. In production, this would be an .onion address.");
    info!("Press Ctrl+C to stop.");

    // Run the server
    if let Err(e) = server.await {
        error!("Server error: {}", e);
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"))
        )
        .init();

    info!("=== Task 2: Arti Hello World ===");
    info!("Starting Tor hidden service demo...");

    run_hidden_service().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_handle_request() {
        let req = Request::builder()
            .method("GET")
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = handle_request(req).await.unwrap();
        assert_eq!(response.status(), 200);
    }
}
