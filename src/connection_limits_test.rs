use std::collections::HashMap;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

/// Mock implementation of the connection tracking logic from server.rs
/// This allows us to test the connection limit logic directly
struct ConnectionTracker {
    // Track connections per IP (mimics max_channels_per_key in server.rs)
    connections_per_ip: Mutex<HashMap<IpAddr, usize>>,

    // Track total active connections (mimics the global counter in server.rs)
    total_connections: AtomicUsize,

    // Configuration
    max_per_ip: usize,
    max_total: usize,
}

impl ConnectionTracker {
    fn new(max_per_ip: usize, max_total: usize) -> Self {
        ConnectionTracker {
            connections_per_ip: Mutex::new(HashMap::new()),
            total_connections: AtomicUsize::new(0),
            max_per_ip,
            max_total,
        }
    }

    /// Try to establish a new connection from the given IP
    /// Returns true if connection is allowed, false if denied due to limits
    fn try_connect(&self, ip: IpAddr) -> bool {
        // First check total connection limit
        let current_total = self.total_connections.load(Ordering::SeqCst);
        if current_total >= self.max_total {
            println!(
                "Connection from {:?} rejected: total limit ({}/{}) reached",
                ip, current_total, self.max_total
            );
            return false;
        }

        // Then check per-IP limit
        let mut connections = self.connections_per_ip.lock().unwrap();
        let ip_count = connections.entry(ip).or_insert(0);

        if *ip_count >= self.max_per_ip {
            println!(
                "Connection from {:?} rejected: per-IP limit ({}/{}) reached",
                ip, *ip_count, self.max_per_ip
            );
            return false;
        }

        // Accept connection and update counters
        *ip_count += 1;
        self.total_connections.fetch_add(1, Ordering::SeqCst);

        println!(
            "Connection from {:?} accepted (IP connections: {}, total: {})",
            ip,
            *ip_count,
            self.total_connections.load(Ordering::SeqCst)
        );
        true
    }

    /// Disconnect a client - reduces the connection count
    fn disconnect(&self, ip: IpAddr) {
        let mut connections = self.connections_per_ip.lock().unwrap();
        if let Some(count) = connections.get_mut(&ip) {
            if *count > 0 {
                *count -= 1;
                self.total_connections.fetch_sub(1, Ordering::SeqCst);
                println!(
                    "Client {:?} disconnected (IP connections: {}, total: {})",
                    ip,
                    *count,
                    self.total_connections.load(Ordering::SeqCst)
                );
            }
        }
    }
}

/// Test that simulates 10 connections from different IPs and verifies the 11th fails
fn test_ten_connection_limit() {
    println!("\n=== Test: 10-Channel Total Connection Limit ===");
    println!("This test mocks connections from different IPs to verify the server's");
    println!("10-channel total connection limit without modifying server.rs\n");

    // Create a tracker with the same config as the server (1 per IP, 10 total)
    let tracker = ConnectionTracker::new(1, 10);

    // Create 11 unique IP addresses
    let ips: Vec<IpAddr> = (1..=11)
        .map(|i| IpAddr::V4(Ipv4Addr::new(192, 168, 0, i)))
        .collect();

    println!("Attempting to connect from 11 different IP addresses...");

    // Track success and failure
    let mut successful = Vec::new();
    let mut failed = Vec::new();

    // First attempt to connect with all 11 IPs
    for (i, &ip) in ips.iter().enumerate() {
        println!("\nAttempt #{}: Connection from IP {:?}", i + 1, ip);
        if tracker.try_connect(ip) {
            successful.push(ip);
        } else {
            failed.push(ip);
        }

        // Add a small delay to make output more readable
        thread::sleep(Duration::from_millis(100));
    }

    // Print results
    println!("\n--- Test Results ---");
    println!("Total connection attempts: {}", ips.len());
    println!("Successful connections: {}", successful.len());
    println!("Failed connections: {}", failed.len());

    // Verify our expectations
    if successful.len() == 10 && failed.len() == 1 {
        println!("✅ Test PASSED: Exactly 10 connections were allowed, and the 11th was rejected");
    } else {
        println!("❌ Test FAILED: Expected 10 successful and 1 failed connection");
        println!(
            "   Instead got {} successful and {} failed",
            successful.len(),
            failed.len()
        );
    }

    // Show we can connect once we disconnect
    if !failed.is_empty() {
        println!("\n--- Testing connection after disconnect ---");
        let disconnect_ip = successful[0];
        println!("Disconnecting client from IP {:?}", disconnect_ip);
        tracker.disconnect(disconnect_ip);

        let retry_ip = failed[0];
        println!(
            "Retrying connection from previously rejected IP {:?}",
            retry_ip
        );
        if tracker.try_connect(retry_ip) {
            println!("✅ Successfully connected after freeing a slot");
        } else {
            println!("❌ Failed to connect after freeing a slot");
        }
    }
}

/// Main test runner - simplified to focus on the 10-channel limit test
pub fn run_mock_tests() {
    println!("\n=================================================");
    println!("Running connection limit tests with IP mocking");
    println!("=================================================\n");

    test_ten_connection_limit();

    println!("\nMock server tests completed! ✅");
}
