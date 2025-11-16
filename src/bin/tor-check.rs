//! Tor Connectivity Check Tool
//!
//! This tool comprehensively tests Tor functionality:
//! 1. Bootstrap connection to Tor network
//! 2. Access remote websites over Tor
//! 3. Access existing Tor hidden services
//! 4. Publish ephemeral Tor hidden services
//! 5. Connect to own hidden service and verify round-trip communication

use anyhow::Result;
use std::time::Duration;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber;

use arti_client::{TorClient, TorClientConfig};
use tor_rtcompat::PreferredRuntime;
use tor_hsservice::config::OnionServiceConfigBuilder;
use tor_hsservice::{RunningOnionService, handle_rend_requests};
use tor_cell::relaycell::msg::Connected;
use safelog::DisplayRedacted;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use tokio::sync::oneshot;
use futures::StreamExt;

/// Check 1: Bootstrap connection to Tor network
async fn check_tor_bootstrap() -> Result<TorClient<PreferredRuntime>> {
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║ CHECK 1: Tor Network Bootstrap                           ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");
    info!("Purpose: Verify we can connect to the Tor network");
    info!("  → Connects to Tor directory authorities");
    info!("  → Downloads consensus documents");
    info!("  → Builds circuit through Tor relays");
    info!("");
    info!("Status: Bootstrapping Tor client (may take 10-30 seconds)...");

    let start = std::time::Instant::now();

    match tokio::time::timeout(
        Duration::from_secs(60),
        TorClient::create_bootstrapped(TorClientConfig::default())
    ).await {
        Ok(Ok(tor_client)) => {
            let elapsed = start.elapsed();
            info!("");
            info!("✅ CHECK 1 PASSED: Successfully connected to Tor network!");
            info!("   Bootstrap time: {:.2}s", elapsed.as_secs_f64());
            info!("");
            Ok(tor_client)
        }
        Ok(Err(e)) => {
            error!("");
            error!("❌ CHECK 1 FAILED: Could not bootstrap Tor connection");
            error!("   Error: {}", e);
            error!("");
            error!("Possible causes:");
            error!("  • No internet connectivity");
            error!("  • Firewall blocking outbound connections");
            error!("  • Tor directory authorities unreachable");
            error!("  • DNS resolution issues");
            error!("");
            anyhow::bail!("Tor bootstrap failed: {}", e);
        }
        Err(_) => {
            error!("");
            error!("❌ CHECK 1 FAILED: Tor bootstrap timeout (>60 seconds)");
            error!("");
            error!("This usually indicates:");
            error!("  • Slow/unstable internet connection");
            error!("  • Network filtering/throttling");
            error!("  • Tor directory authorities overloaded");
            error!("");
            anyhow::bail!("Tor bootstrap timeout");
        }
    }
}

/// Check 2: Access remote websites over Tor
async fn check_clearnet_over_tor(tor_client: &TorClient<PreferredRuntime>) -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║ CHECK 2: Access Remote Websites Over Tor                 ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");
    info!("Purpose: Verify we can browse the regular internet through Tor");
    info!("  → Tests TCP connections through Tor circuits");
    info!("  → Validates we can reach clearnet sites anonymously");
    info!("  → Target: www.torproject.org:80");
    info!("");
    info!("Status: Connecting to www.torproject.org...");

    let start = std::time::Instant::now();

    match tokio::time::timeout(
        Duration::from_secs(30),
        tor_client.connect(("www.torproject.org", 80))
    ).await {
        Ok(Ok(mut stream)) => {
            let elapsed = start.elapsed();

            // Send a simple HTTP HEAD request
            let request = "HEAD / HTTP/1.0\r\nHost: www.torproject.org\r\n\r\n";

            match tokio::time::timeout(
                Duration::from_secs(10),
                stream.write_all(request.as_bytes())
            ).await {
                Ok(Ok(_)) => {
                    // Try to read response
                    let mut buf = vec![0u8; 1024];
                    match tokio::time::timeout(
                        Duration::from_secs(10),
                        stream.read(&mut buf)
                    ).await {
                        Ok(Ok(n)) if n > 0 => {
                            let response = String::from_utf8_lossy(&buf[..n]);
                            if response.contains("HTTP/1") {
                                info!("");
                                info!("✅ CHECK 2 PASSED: Successfully accessed website over Tor!");
                                info!("   Connected to: www.torproject.org:80");
                                info!("   Request time: {:.2}s", elapsed.as_secs_f64());
                                info!("");
                                Ok(())
                            } else {
                                error!("");
                                error!("❌ CHECK 2 FAILED: Unexpected response format");
                                error!("");
                                anyhow::bail!("Invalid HTTP response");
                            }
                        }
                        Ok(Ok(_)) => {
                            error!("");
                            error!("❌ CHECK 2 FAILED: Empty response from server");
                            error!("");
                            anyhow::bail!("Empty response");
                        }
                        Ok(Err(e)) => {
                            error!("");
                            error!("❌ CHECK 2 FAILED: Read error: {}", e);
                            error!("");
                            anyhow::bail!("Read failed: {}", e);
                        }
                        Err(_) => {
                            error!("");
                            error!("❌ CHECK 2 FAILED: Read timeout");
                            error!("");
                            anyhow::bail!("Read timeout");
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("");
                    error!("❌ CHECK 2 FAILED: Write error: {}", e);
                    error!("");
                    anyhow::bail!("Write failed: {}", e);
                }
                Err(_) => {
                    error!("");
                    error!("❌ CHECK 2 FAILED: Write timeout");
                    error!("");
                    anyhow::bail!("Write timeout");
                }
            }
        }
        Ok(Err(e)) => {
            error!("");
            error!("❌ CHECK 2 FAILED: Could not connect to website over Tor");
            error!("   Error: {}", e);
            error!("");
            anyhow::bail!("Clearnet connection failed: {}", e);
        }
        Err(_) => {
            error!("");
            error!("❌ CHECK 2 FAILED: Connection timeout (>30 seconds)");
            error!("");
            anyhow::bail!("Clearnet connection timeout");
        }
    }
}

/// Check 3: Access existing Tor hidden service
async fn check_hidden_service_access(tor_client: &TorClient<PreferredRuntime>) -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║ CHECK 3: Access Existing Tor Hidden Service              ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");
    info!("Purpose: Verify we can connect to .onion hidden services");
    info!("  → Tests rendezvous circuit creation");
    info!("  → Validates hidden service protocol functionality");
    info!("  → Target: DuckDuckGo onion service");
    info!("");
    info!("Status: Connecting to DuckDuckGo hidden service...");

    // DuckDuckGo's well-known onion address
    let onion_host = "duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion";

    let start = std::time::Instant::now();

    match tokio::time::timeout(
        Duration::from_secs(60),
        tor_client.connect((onion_host, 80))
    ).await {
        Ok(Ok(mut stream)) => {
            let elapsed = start.elapsed();

            // Send a simple HTTP HEAD request
            let request = format!("HEAD / HTTP/1.0\r\nHost: {}\r\n\r\n", onion_host);

            match tokio::time::timeout(
                Duration::from_secs(15),
                stream.write_all(request.as_bytes())
            ).await {
                Ok(Ok(_)) => {
                    // Try to read response
                    let mut buf = vec![0u8; 1024];
                    match tokio::time::timeout(
                        Duration::from_secs(15),
                        stream.read(&mut buf)
                    ).await {
                        Ok(Ok(n)) if n > 0 => {
                            let response = String::from_utf8_lossy(&buf[..n]);
                            if response.contains("HTTP/1") {
                                info!("");
                                info!("✅ CHECK 3 PASSED: Successfully accessed hidden service!");
                                info!("   Connected to: {}", onion_host);
                                info!("   Request time: {:.2}s", elapsed.as_secs_f64());
                                info!("");
                                Ok(())
                            } else {
                                error!("");
                                error!("❌ CHECK 3 FAILED: Unexpected response format");
                                error!("");
                                anyhow::bail!("Invalid HTTP response from hidden service");
                            }
                        }
                        Ok(Ok(_)) => {
                            error!("");
                            error!("❌ CHECK 3 FAILED: Empty response from hidden service");
                            error!("");
                            anyhow::bail!("Empty response");
                        }
                        Ok(Err(e)) => {
                            error!("");
                            error!("❌ CHECK 3 FAILED: Read error: {}", e);
                            error!("");
                            anyhow::bail!("Read failed: {}", e);
                        }
                        Err(_) => {
                            error!("");
                            error!("❌ CHECK 3 FAILED: Read timeout");
                            error!("");
                            anyhow::bail!("Read timeout");
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("");
                    error!("❌ CHECK 3 FAILED: Write error: {}", e);
                    error!("");
                    anyhow::bail!("Write failed: {}", e);
                }
                Err(_) => {
                    error!("");
                    error!("❌ CHECK 3 FAILED: Write timeout");
                    error!("");
                    anyhow::bail!("Write timeout");
                }
            }
        }
        Ok(Err(e)) => {
            error!("");
            error!("❌ CHECK 3 FAILED: Could not connect to hidden service");
            error!("   Error: {}", e);
            error!("");
            error!("Possible causes:");
            error!("  • Hidden service is down");
            error!("  • Rendezvous circuit creation failed");
            error!("  • Network congestion");
            error!("");
            anyhow::bail!("Hidden service connection failed: {}", e);
        }
        Err(_) => {
            error!("");
            error!("❌ CHECK 3 FAILED: Connection timeout (>60 seconds)");
            error!("   Note: Hidden service connections can be slow");
            error!("");
            anyhow::bail!("Hidden service timeout");
        }
    }
}

const TEST_MESSAGE: &str = "TOR_CHECK_PING_v1";
const TEST_RESPONSE: &str = "TOR_CHECK_PONG_v1";

/// Check 4: Publish ephemeral Tor hidden service and handle requests
async fn check_hidden_service_publish(
    tor_client: &TorClient<PreferredRuntime>,
) -> Result<(Arc<RunningOnionService>, String, oneshot::Receiver<bool>)> {
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║ CHECK 4: Publish Ephemeral Tor Hidden Service            ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");
    info!("Purpose: Verify we can create and publish hidden services");
    info!("  → Tests ability to register .onion addresses");
    info!("  → Validates we can act as a hidden service");
    info!("  → Creates temporary ephemeral service");
    info!("  → Listens for incoming connections");
    info!("");
    info!("Status: Creating ephemeral hidden service...");

    let start = std::time::Instant::now();

    // Create an ephemeral onion service configuration
    let svc_config = OnionServiceConfigBuilder::default()
        .nickname("tor-check-test".parse()?)
        .build()?;

    // Launch the onion service
    let (onion_service, request_stream) = tor_client
        .launch_onion_service(svc_config)
        .map_err(|e| {
            error!("");
            error!("❌ CHECK 4 FAILED: Could not launch onion service");
            error!("   Error: {}", e);
            error!("");
            error!("Possible causes:");
            error!("  • Tor consensus not fully downloaded");
            error!("  • Insufficient circuit resources");
            error!("  • Hidden service directory upload failed");
            error!("");
            e
        })?;

    info!("  → Onion service launched successfully");

    // Wait for the onion address to be available
    info!("  → Waiting for .onion address registration...");

    let addr = tokio::time::timeout(
        Duration::from_secs(30),
        async {
            loop {
                if let Some(addr) = onion_service.onion_address() {
                    return Ok::<_, anyhow::Error>(addr);
                }
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }
    )
    .await
    .map_err(|_| {
        error!("");
        error!("❌ CHECK 4 FAILED: Timeout waiting for onion address");
        error!("   The service was launched but address registration timed out");
        error!("");
        anyhow::anyhow!("Onion address registration timeout")
    })??;

    let elapsed = start.elapsed();
    let full_address = format!("{}", addr.display_unredacted());

    info!("");
    info!("✅ CHECK 4 PASSED: Successfully published hidden service!");
    info!("");
    info!("   ┌─────────────────────────────────────────────────────────────┐");
    info!("   │ .onion address (copy this):                                 │");
    info!("   │ {}                                │", full_address);
    info!("   └─────────────────────────────────────────────────────────────┘");
    info!("");
    info!("   Registration time: {:.2}s", elapsed.as_secs_f64());
    info!("");
    info!("Note: Keeping service running for CHECK 5 verification...");
    info!("");

    // Spawn a task to handle incoming requests
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    tokio::spawn(async move {
        let mut stream_requests = handle_rend_requests(request_stream);

        while let Some(request) = stream_requests.next().await {
            info!("  → CHECK 4: Received connection from CHECK 5");

            // Accept the connection
            let mut stream = match request.accept(Connected::new_empty()).await {
                Ok(stream) => stream,
                Err(e) => {
                    error!("  → CHECK 4: Failed to accept stream: {}", e);
                    continue;
                }
            };

            // Read the message
            let mut buf = vec![0u8; 1024];
            match stream.read(&mut buf).await {
                Ok(n) if n > 0 => {
                    let message = String::from_utf8_lossy(&buf[..n]);
                    info!("  → CHECK 4: Received message: {}", message.trim());

                    if message.trim() == TEST_MESSAGE {
                        info!("  → CHECK 4: Message verified! Sending response...");

                        // Send response
                        if let Err(e) = stream.write_all(TEST_RESPONSE.as_bytes()).await {
                            error!("  → CHECK 4: Failed to send response: {}", e);
                            continue;
                        }

                        info!("  ✅ CHECK 4: Successfully exchanged messages with CHECK 5!");

                        // Signal success
                        if let Some(tx) = tx.lock().await.take() {
                            let _ = tx.send(true);
                        }
                        break;
                    } else {
                        warn!("  → CHECK 4: Unexpected message: {}", message.trim());
                    }
                }
                Ok(_) => {
                    warn!("  → CHECK 4: Received empty message");
                }
                Err(e) => {
                    error!("  → CHECK 4: Read error: {}", e);
                }
            }
        }
    });

    Ok((onion_service, full_address, rx))
}

/// Check 5: Connect to our own hidden service and verify round-trip
async fn check_hidden_service_roundtrip(
    tor_client: &TorClient<PreferredRuntime>,
    onion_address: &str,
) -> Result<()> {
    info!("╔═══════════════════════════════════════════════════════════╗");
    info!("║ CHECK 5: Connect to Own Hidden Service (Round-Trip)      ║");
    info!("╚═══════════════════════════════════════════════════════════╝");
    info!("");
    info!("Purpose: Verify complete hidden service functionality");
    info!("  → Tests connecting to our own .onion service");
    info!("  → Sends test message to CHECK 4");
    info!("  → Verifies response from CHECK 4");
    info!("  → Validates full publish + access workflow");
    info!("");
    info!("Status: Connecting to {}...", onion_address);

    // Parse the onion address to extract hostname and port
    let onion_host = if onion_address.ends_with(".onion") {
        onion_address.to_string()
    } else {
        format!("{}.onion", onion_address)
    };

    let start = std::time::Instant::now();

    // Connect to the hidden service on port 80 (default)
    match tokio::time::timeout(
        Duration::from_secs(60),
        tor_client.connect((onion_host.as_str(), 80))
    ).await {
        Ok(Ok(mut stream)) => {
            info!("  → CHECK 5: Connected to hidden service!");
            info!("  → CHECK 5: Sending test message: '{}'", TEST_MESSAGE);

            // Send the test message
            match tokio::time::timeout(
                Duration::from_secs(10),
                stream.write_all(TEST_MESSAGE.as_bytes())
            ).await {
                Ok(Ok(_)) => {
                    info!("  → CHECK 5: Message sent, waiting for response...");

                    // Read the response
                    let mut buf = vec![0u8; 1024];
                    match tokio::time::timeout(
                        Duration::from_secs(10),
                        stream.read(&mut buf)
                    ).await {
                        Ok(Ok(n)) if n > 0 => {
                            let response = String::from_utf8_lossy(&buf[..n]);
                            info!("  → CHECK 5: Received response: '{}'", response.trim());

                            if response.trim() == TEST_RESPONSE {
                                let elapsed = start.elapsed();
                                info!("");
                                info!("✅ CHECK 5 PASSED: Successfully verified round-trip communication!");
                                info!("   Connected to: {}", onion_host);
                                info!("   Round-trip time: {:.2}s", elapsed.as_secs_f64());
                                info!("   Message sent: '{}'", TEST_MESSAGE);
                                info!("   Response received: '{}'", TEST_RESPONSE);
                                info!("");
                                Ok(())
                            } else {
                                error!("");
                                error!("❌ CHECK 5 FAILED: Unexpected response");
                                error!("   Expected: '{}'", TEST_RESPONSE);
                                error!("   Received: '{}'", response.trim());
                                error!("");
                                anyhow::bail!("Unexpected response from hidden service");
                            }
                        }
                        Ok(Ok(_)) => {
                            error!("");
                            error!("❌ CHECK 5 FAILED: Empty response from hidden service");
                            error!("");
                            anyhow::bail!("Empty response");
                        }
                        Ok(Err(e)) => {
                            error!("");
                            error!("❌ CHECK 5 FAILED: Read error: {}", e);
                            error!("");
                            anyhow::bail!("Read failed: {}", e);
                        }
                        Err(_) => {
                            error!("");
                            error!("❌ CHECK 5 FAILED: Response timeout");
                            error!("");
                            anyhow::bail!("Read timeout");
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("");
                    error!("❌ CHECK 5 FAILED: Write error: {}", e);
                    error!("");
                    anyhow::bail!("Write failed: {}", e);
                }
                Err(_) => {
                    error!("");
                    error!("❌ CHECK 5 FAILED: Write timeout");
                    error!("");
                    anyhow::bail!("Write timeout");
                }
            }
        }
        Ok(Err(e)) => {
            error!("");
            error!("❌ CHECK 5 FAILED: Could not connect to hidden service");
            error!("   Error: {}", e);
            error!("");
            error!("Possible causes:");
            error!("  • Hidden service not yet published to HSDir");
            error!("  • Rendezvous circuit creation failed");
            error!("  • Service port not accepting connections");
            error!("");
            anyhow::bail!("Connection to own hidden service failed: {}", e);
        }
        Err(_) => {
            error!("");
            error!("❌ CHECK 5 FAILED: Connection timeout (>60 seconds)");
            error!("   Note: First connection to new hidden service can be slow");
            error!("");
            anyhow::bail!("Connection timeout");
        }
    }
}

/// Test if we're in a sandboxed/restricted environment
fn check_environment() {
    info!("Environment diagnostics:");

    // Check if we can resolve DNS
    match std::net::ToSocketAddrs::to_socket_addrs(&("torproject.org", 443)) {
        Ok(mut addrs) => {
            if let Some(addr) = addrs.next() {
                info!("  ✅ DNS resolution: OK (torproject.org -> {})", addr.ip());
            } else {
                warn!("  ⚠️  DNS resolution: No addresses returned");
            }
        }
        Err(e) => {
            warn!("  ⚠️  DNS resolution: FAILED ({})", e);
        }
    }

    // Check home directory (for Arti state)
    match std::env::var("HOME") {
        Ok(home) => {
            info!("  ✅ HOME directory: {}", home);
            let arti_dir = format!("{}/.local/share/arti", home);
            if std::path::Path::new(&arti_dir).exists() {
                info!("  ✅ Arti state directory exists: {}", arti_dir);
            } else {
                info!("  ℹ️  Arti state directory will be created: {}", arti_dir);
            }
        }
        Err(_) => {
            warn!("  ⚠️  HOME environment variable not set");
        }
    }

    info!("");
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

    info!("═══════════════════════════════════════════════════════════════");
    info!("         Tor Connectivity Comprehensive Diagnostic Tool        ");
    info!("═══════════════════════════════════════════════════════════════");
    info!("Version: {}", env!("CARGO_PKG_VERSION"));
    info!("");
    info!("This tool validates all aspects of Tor functionality:");
    info!("  1. Bootstrap connection to Tor network");
    info!("  2. Access remote websites over Tor (clearnet)");
    info!("  3. Access existing Tor hidden services (.onion)");
    info!("  4. Publish Tor hidden services");
    info!("  5. Verify round-trip communication with own hidden service");
    info!("");

    // Check environment first
    check_environment();

    let mut checks_passed = 0;
    let mut checks_failed = 0;
    let checks_skipped = 0;

    // Check 1: Bootstrap Tor
    let tor_client = match check_tor_bootstrap().await {
        Ok(client) => {
            checks_passed += 1;
            client
        }
        Err(_e) => {
            checks_failed += 1;
            error!("Cannot proceed with further checks without Tor connection.");
            print_summary(checks_passed, checks_failed, checks_skipped);
            std::process::exit(1);
        }
    };

    // Check 2: Access clearnet over Tor
    match check_clearnet_over_tor(&tor_client).await {
        Ok(_) => checks_passed += 1,
        Err(_e) => {
            checks_failed += 1;
            warn!("Continuing with remaining checks...");
            info!("");
        }
    }

    // Check 3: Access hidden service
    match check_hidden_service_access(&tor_client).await {
        Ok(_) => checks_passed += 1,
        Err(_e) => {
            checks_failed += 1;
            warn!("Continuing with remaining checks...");
            info!("");
        }
    }

    // Check 4 & 5: Publish hidden service and verify round-trip
    let (onion_service, onion_address, check4_rx) = match check_hidden_service_publish(&tor_client).await {
        Ok(result) => {
            checks_passed += 1;
            result
        }
        Err(_e) => {
            checks_failed += 1;
            warn!("Skipping CHECK 5 (requires CHECK 4 to pass)...");
            info!("");
            print_summary(checks_passed, checks_failed, checks_skipped);
            if checks_failed == 0 {
                std::process::exit(0);
            } else {
                std::process::exit(1);
            }
        }
    };

    // Check 5: Connect to own hidden service
    match check_hidden_service_roundtrip(&tor_client, &onion_address).await {
        Ok(_) => {
            checks_passed += 1;

            // Wait for CHECK 4 to confirm it received the message
            match tokio::time::timeout(Duration::from_secs(5), check4_rx).await {
                Ok(Ok(true)) => {
                    info!("  ✅ CHECK 4 & 5: Full round-trip verified!");
                    info!("");
                }
                _ => {
                    warn!("  ⚠️  CHECK 4 did not confirm message receipt (but CHECK 5 passed)");
                    info!("");
                }
            }
        }
        Err(_e) => {
            checks_failed += 1;
            warn!("Continuing to summary...");
            info!("");
        }
    }

    // Clean up the hidden service
    drop(onion_service);
    info!("Cleaned up ephemeral hidden service.");
    info!("");

    print_summary(checks_passed, checks_failed, checks_skipped);

    if checks_failed == 0 {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

fn print_summary(passed: u32, failed: u32, skipped: u32) {
    info!("═══════════════════════════════════════════════════════════════");
    info!("                        TEST SUMMARY                           ");
    info!("═══════════════════════════════════════════════════════════════");
    info!("");
    info!("  ✅ Passed:  {}", passed);
    info!("  ❌ Failed:  {}", failed);
    info!("  ⚠️  Skipped: {}", skipped);
    info!("");

    if failed == 0 && passed >= 5 {
        info!("╔═══════════════════════════════════════════════════════════╗");
        info!("║          ALL CHECKS PASSED ✅                             ║");
        info!("╚═══════════════════════════════════════════════════════════╝");
        info!("");
        info!("Your Tor setup is fully functional!");
        info!("All capabilities verified:");
        info!("  ✅ Tor network connectivity");
        info!("  ✅ Browse the internet anonymously over Tor");
        info!("  ✅ Access .onion hidden services");
        info!("  ✅ Publish your own .onion hidden services");
        info!("  ✅ Full round-trip communication verified");
        info!("");
        info!("You are ready to run 'eddi' to launch persistent hidden services!");
    } else if failed == 0 && passed >= 3 {
        info!("╔═══════════════════════════════════════════════════════════╗");
        info!("║          CRITICAL CHECKS PASSED ✅                        ║");
        info!("╚═══════════════════════════════════════════════════════════╝");
        info!("");
        info!("Your Tor setup is mostly functional.");
        info!("You can now:");
        info!("  • Browse the internet anonymously over Tor");
        info!("  • Access .onion hidden services");
        info!("");
        info!("Note: Some checks did not pass, but core functionality works.");
    } else if failed > 0 {
        error!("╔═══════════════════════════════════════════════════════════╗");
        error!("║          SOME CHECKS FAILED ❌                            ║");
        error!("╚═══════════════════════════════════════════════════════════╝");
        error!("");
        error!("Review the errors above for troubleshooting guidance.");
    }

    info!("");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_check() {
        // This should not panic
        check_environment();
    }
}
