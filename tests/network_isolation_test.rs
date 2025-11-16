//! Integration tests to verify network isolation
//!
//! These tests ensure that gunicorn, when bound to a Unix Domain Socket,
//! does NOT open any TCP, UDP, or other network sockets.
//!
//! This is a critical security requirement for the eddi project.

use std::fs;
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::Duration;

/// Represents a network socket entry from /proc/net/*
#[derive(Debug)]
#[allow(dead_code)]
struct NetworkSocket {
    local_address: String,
    local_port: u16,
    state: String,
}

/// Parse /proc/<pid>/net/tcp or /proc/<pid>/net/tcp6
fn parse_tcp_sockets(content: &str) -> Vec<NetworkSocket> {
    let mut sockets = Vec::new();

    for line in content.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        // Parse local address (format: "0100007F:1F90" for 127.0.0.1:8080)
        let local_addr_parts: Vec<&str> = parts[1].split(':').collect();
        if local_addr_parts.len() != 2 {
            continue;
        }

        let port_hex = local_addr_parts[1];
        let port = u16::from_str_radix(port_hex, 16).unwrap_or(0);

        // Parse state (0A = LISTEN in hex)
        let state = parts[3];
        let state_str = if state == "0A" {
            "LISTEN"
        } else {
            "OTHER"
        };

        sockets.push(NetworkSocket {
            local_address: local_addr_parts[0].to_string(),
            local_port: port,
            state: state_str.to_string(),
        });
    }

    sockets
}

/// Parse /proc/<pid>/net/udp or /proc/<pid>/net/udp6
fn parse_udp_sockets(content: &str) -> Vec<NetworkSocket> {
    let mut sockets = Vec::new();

    for line in content.lines().skip(1) {
        // Skip header
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }

        // Parse local address
        let local_addr_parts: Vec<&str> = parts[1].split(':').collect();
        if local_addr_parts.len() != 2 {
            continue;
        }

        let port_hex = local_addr_parts[1];
        let port = u16::from_str_radix(port_hex, 16).unwrap_or(0);

        sockets.push(NetworkSocket {
            local_address: local_addr_parts[0].to_string(),
            local_port: port,
            state: "UDP".to_string(),
        });
    }

    sockets
}

/// Check if a process has any listening TCP sockets
fn check_tcp_sockets(pid: u32) -> Result<Vec<NetworkSocket>, std::io::Error> {
    let mut all_sockets = Vec::new();

    // Check IPv4 TCP
    let tcp_path = format!("/proc/{}/net/tcp", pid);
    if let Ok(content) = fs::read_to_string(&tcp_path) {
        let sockets = parse_tcp_sockets(&content);
        let listening: Vec<_> = sockets
            .into_iter()
            .filter(|s| s.state == "LISTEN")
            .collect();
        all_sockets.extend(listening);
    }

    // Check IPv6 TCP
    let tcp6_path = format!("/proc/{}/net/tcp6", pid);
    if let Ok(content) = fs::read_to_string(&tcp6_path) {
        let sockets = parse_tcp_sockets(&content);
        let listening: Vec<_> = sockets
            .into_iter()
            .filter(|s| s.state == "LISTEN")
            .collect();
        all_sockets.extend(listening);
    }

    Ok(all_sockets)
}

/// Check if a process has any UDP sockets
fn check_udp_sockets(pid: u32) -> Result<Vec<NetworkSocket>, std::io::Error> {
    let mut all_sockets = Vec::new();

    // Check IPv4 UDP
    let udp_path = format!("/proc/{}/net/udp", pid);
    if let Ok(content) = fs::read_to_string(&udp_path) {
        all_sockets.extend(parse_udp_sockets(&content));
    }

    // Check IPv6 UDP
    let udp6_path = format!("/proc/{}/net/udp6", pid);
    if let Ok(content) = fs::read_to_string(&udp6_path) {
        all_sockets.extend(parse_udp_sockets(&content));
    }

    Ok(all_sockets)
}

/// Get all child process IDs of a given PID
fn get_child_pids(parent_pid: u32) -> Result<Vec<u32>, std::io::Error> {
    let mut child_pids = Vec::new();

    // Read /proc to find all processes
    for entry in fs::read_dir("/proc")? {
        let entry = entry?;
        let path = entry.path();

        // Check if this is a PID directory
        if let Some(file_name) = path.file_name() {
            if let Some(name_str) = file_name.to_str() {
                if let Ok(pid) = name_str.parse::<u32>() {
                    // Read the status file to check parent PID
                    let status_path = path.join("status");
                    if let Ok(status) = fs::read_to_string(status_path) {
                        for line in status.lines() {
                            if line.starts_with("PPid:") {
                                if let Some(ppid_str) = line.split_whitespace().nth(1) {
                                    if let Ok(ppid) = ppid_str.parse::<u32>() {
                                        if ppid == parent_pid {
                                            child_pids.push(pid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(child_pids)
}

/// Spawn gunicorn for testing
struct TestGunicorn {
    child: Child,
    socket_path: PathBuf,
}

impl TestGunicorn {
    fn spawn() -> Result<Self, std::io::Error> {
        let socket_path = PathBuf::from("/tmp/eddi-network-test.sock");

        // Remove existing socket
        let _ = fs::remove_file(&socket_path);

        let bind_addr = format!("unix:{}", socket_path.display());

        let child = Command::new("gunicorn")
            .current_dir("test-apps/flask-demo")
            .arg("--workers")
            .arg("2") // Use 2 workers to test multiple processes
            .arg("--bind")
            .arg(&bind_addr)
            .arg("app:app")
            .spawn()?;

        Ok(Self {
            child,
            socket_path,
        })
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }

    fn wait_for_socket(&self, timeout_secs: u64) -> bool {
        let start = std::time::Instant::now();

        while start.elapsed().as_secs() < timeout_secs {
            if self.socket_path.exists() {
                thread::sleep(Duration::from_millis(500));
                return true;
            }
            thread::sleep(Duration::from_millis(100));
        }

        false
    }
}

impl Drop for TestGunicorn {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.socket_path);
    }
}

#[test]
#[ignore] // Only run with: cargo test -- --ignored
fn test_no_tcp_sockets_opened() {
    // Spawn gunicorn
    let gunicorn = TestGunicorn::spawn()
        .expect("Failed to spawn gunicorn. Is it installed?");

    // Wait for socket
    assert!(
        gunicorn.wait_for_socket(10),
        "Socket file was not created"
    );

    println!("Gunicorn master PID: {}", gunicorn.pid());

    // Get all child PIDs (worker processes)
    let child_pids = get_child_pids(gunicorn.pid()).expect("Failed to get child PIDs");
    println!("Worker PIDs: {:?}", child_pids);

    // Check master process
    let master_tcp = check_tcp_sockets(gunicorn.pid()).expect("Failed to check TCP sockets");
    assert!(
        master_tcp.is_empty(),
        "Master process has listening TCP sockets: {:?}",
        master_tcp
    );

    // Check all worker processes
    for worker_pid in child_pids {
        let worker_tcp = check_tcp_sockets(worker_pid).expect("Failed to check TCP sockets");
        assert!(
            worker_tcp.is_empty(),
            "Worker process {} has listening TCP sockets: {:?}",
            worker_pid,
            worker_tcp
        );
    }

    println!("✓ No TCP sockets found");
}

#[test]
#[ignore] // Only run with: cargo test -- --ignored
fn test_no_udp_sockets_opened() {
    // Spawn gunicorn
    let gunicorn = TestGunicorn::spawn()
        .expect("Failed to spawn gunicorn. Is it installed?");

    // Wait for socket
    assert!(
        gunicorn.wait_for_socket(10),
        "Socket file was not created"
    );

    println!("Gunicorn master PID: {}", gunicorn.pid());

    // Get all child PIDs (worker processes)
    let child_pids = get_child_pids(gunicorn.pid()).expect("Failed to get child PIDs");
    println!("Worker PIDs: {:?}", child_pids);

    // Check master process
    let master_udp = check_udp_sockets(gunicorn.pid()).expect("Failed to check UDP sockets");
    assert!(
        master_udp.is_empty(),
        "Master process has UDP sockets: {:?}",
        master_udp
    );

    // Check all worker processes
    for worker_pid in child_pids {
        let worker_udp = check_udp_sockets(worker_pid).expect("Failed to check UDP sockets");
        assert!(
            worker_udp.is_empty(),
            "Worker process {} has UDP sockets: {:?}",
            worker_pid,
            worker_udp
        );
    }

    println!("✓ No UDP sockets found");
}

#[test]
#[ignore] // Only run with: cargo test -- --ignored
fn test_unix_socket_works() {
    // Spawn gunicorn
    let gunicorn = TestGunicorn::spawn()
        .expect("Failed to spawn gunicorn. Is it installed?");

    // Wait for socket
    assert!(
        gunicorn.wait_for_socket(10),
        "Socket file was not created"
    );

    // Connect and send a request
    let mut stream = UnixStream::connect(&gunicorn.socket_path)
        .expect("Failed to connect to Unix socket");

    let request = "GET / HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n";
    std::io::Write::write_all(&mut stream, request.as_bytes())
        .expect("Failed to write request");

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .expect("Failed to read response");

    assert!(
        response.contains("HTTP/1.1 200 OK") || response.contains("HTTP/1.0 200 OK"),
        "Expected 200 OK response, got: {}",
        response
    );
    assert!(
        response.contains("Hello, this is a secure hidden service!"),
        "Expected correct body, got: {}",
        response
    );

    println!("✓ Unix socket communication works");
}

#[test]
fn test_parse_tcp_sockets() {
    let sample = "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
                      0: 0100007F:1F90 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 12345";

    let sockets = parse_tcp_sockets(sample);
    assert_eq!(sockets.len(), 1);
    assert_eq!(sockets[0].state, "LISTEN");
}
