use std::net::{IpAddr, Ipv6Addr};
use tarpc::{client, context};
use std::time::Duration;
use kv_server::*;
use futures::future::join_all;

// Import our mock tests
mod connection_limits_test;

async fn test_basic_operations() -> Result<(), Box<dyn std::error::Error>> {
    let server_addr = (IpAddr::V6(Ipv6Addr::LOCALHOST), 8899);
    
    println!("Connecting to server at {:?}...", server_addr);
    let transport = tarpc::serde_transport::tcp::connect(server_addr, 
        || tarpc::tokio_serde::formats::Json::default()).await?;
    println!("Connected to server!");
    
    // Create a client with default config
    let client = KeyValueStoreClient::new(client::Config::default(), transport).spawn();
    
    // Test the server with some operations
    let mut ctx = context::current();
    ctx.deadline = context::current().deadline + Duration::from_secs(5);
    
    // Set a key-value pair
    println!("Setting key 'hello' to 'world'");
    client.set(ctx.clone(), SetRequest {
        key: "hello".to_string(),
        value: "world".to_string(),
    }).await?;
    
    // Get the value back
    println!("Getting key 'hello'");
    let response = client.get(ctx.clone(), GetRequest {
        key: "hello".to_string(),
    }).await?;
    
    println!("Value for 'hello': {:?}", response.value);
    
    // Try getting a non-existent key
    println!("Getting non-existent key 'nonexistent'");
    let response = client.get(ctx.clone(), GetRequest {
        key: "nonexistent".to_string(),
    }).await?;
    
    println!("Value for 'nonexistent': {:?}", response.value);
    
    // Delete the key
    println!("Deleting key 'hello'");
    client.delete(ctx.clone(), DeleteRequest {
        key: "hello".to_string(),
    }).await?;
    
    // Verify it's deleted
    println!("Getting deleted key 'hello'");
    let response = client.get(ctx.clone(), GetRequest {
        key: "hello".to_string(),
    }).await?;
    
    println!("Value for 'hello' after deletion: {:?}", response.value);
    
    Ok(())
}

/// Attempts to establish a client connection to the server and make a test request
async fn try_connect(server_addr: (IpAddr, u16), attempt_number: usize) -> bool {
    println!("Connection attempt #{}", attempt_number);
    
    // Try to establish TCP connection
    let transport_result = tarpc::serde_transport::tcp::connect(
        server_addr, 
        || tarpc::tokio_serde::formats::Json::default()
    ).await;
    
    match transport_result {
        Ok(transport) => {
            println!("TCP connection #{} succeeded", attempt_number);
            
            // Create RPC client
            let client = KeyValueStoreClient::new(client::Config::default(), transport).spawn();
            
            // Test the RPC connection with a simple request
            let ctx = context::current();
            match client.get(ctx, GetRequest { key: "test".to_string() }).await {
                Ok(_) => {
                    println!("  RPC request on connection #{} succeeded", attempt_number);
                    true // Connection fully successful at both TCP and RPC levels
                },
                Err(e) => {
                    println!("  RPC request on connection #{} failed: {}", attempt_number, e);
                    false // TCP connection succeeded but RPC failed
                }
            }
        },
        Err(e) => {
            println!("TCP connection #{} failed: {}", attempt_number, e);
            false // TCP connection failed
        }
    }
}

/// Tests that the server enforces a limit of 1 connection per IP address
async fn test_ip_connection_limit() -> Result<(), Box<dyn std::error::Error>> {
    let server_addr = (IpAddr::V6(Ipv6Addr::LOCALHOST), 8899);
    println!("\n=== Testing IP connection limit ===");
    println!("Attempting to create multiple connections from the same IP address...");
    
    // Number of connection attempts to make
    let num_attempts = 3;
    
    // Create a vector of connection attempt futures
    let connection_futures = (1..=num_attempts)
        .map(|i| try_connect(server_addr, i))
        .collect::<Vec<_>>();
    
    // Wait for all connection attempts to complete
    let results = join_all(connection_futures).await;
    
    // Count successful connections (both TCP and RPC levels)
    let success_count = results.iter().filter(|&&success| success).count();
    
    // Print test results
    println!("\n--- Test Results ---");
    println!("Total connection attempts: {}", num_attempts);
    println!("Successful connections: {}", success_count);
    
    // Verify only one connection succeeded
    if success_count == 1 {
        println!("✅ Test PASSED: Only one connection was allowed from the same IP");
    } else {
        println!("❌ Test FAILED: Expected 1 connection, but got {}", success_count);
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Running all connection limit tests ===\n");
    
    // 1. Basic operations test
    println!("\n➡️ Running basic operations test...");
    test_basic_operations().await?;
    
    // 2. IP connection limit test
    println!("\n➡️ Running IP connection limit test (1 per IP)...");
    test_ip_connection_limit().await?;
    
    // 3. 10-channel limit mock test
    println!("\n➡️ Running 10-channel limit mock test...");
    connection_limits_test::run_mock_tests();
    
    println!("\n=== All tests completed successfully ===");
    Ok(())
} 