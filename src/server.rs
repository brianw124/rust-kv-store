use std::net::{IpAddr, Ipv6Addr};
use futures::future;
use futures_util::StreamExt;
use tarpc::context;
use tarpc::tokio_serde::formats::Json;
use tarpc::server::{self, Channel, incoming::Incoming};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::error::Error;
use kv_server::*;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
struct Server {
    store: Arc<Mutex<HashMap<String, String>>>,
    // Connection counter
    connection_count: Arc<AtomicUsize>,
    // Maximum allowed concurrent connections
    max_connections: usize,
}

impl KeyValueStore for Server {
    // Need to define the future types for tarpc
    type SetFut = future::Ready<()>;
    type GetFut = future::Ready<GetResponse>;
    type DeleteFut = future::Ready<()>;

    fn set(self, _: context::Context, req: SetRequest) -> Self::SetFut {
        let mut store = self.store.lock().unwrap();
        store.insert(req.key, req.value);
        future::ready(())
    }

    fn get(self, _: context::Context, req: GetRequest) -> Self::GetFut {
        let store = self.store.lock().unwrap();
        let response = GetResponse {
            value: store.get(&req.key).cloned(),
        };
        future::ready(response)
    }

    fn delete(self, _: context::Context, req: DeleteRequest) -> Self::DeleteFut {
        let mut store = self.store.lock().unwrap();
        store.remove(&req.key);
        future::ready(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create server with a connection counter
    let server = Server {
        store: Arc::new(Mutex::new(HashMap::new())),
        connection_count: Arc::new(AtomicUsize::new(0)),
        max_connections: 10, // Set maximum connections to 10
    };
    
    let server_addr = (IpAddr::V6(Ipv6Addr::LOCALHOST), 8899);
    
    // JSON transport is provided by the json_transport tarpc module. It makes it easy
    // to start up a serde-powered json serialization strategy over TCP.
    let mut listener = tarpc::serde_transport::tcp::listen(&server_addr, Json::default).await?;
    listener.config_mut().max_frame_length(usize::MAX);
    
    println!("Server listening on {:?}", server_addr);
    println!("Maximum client connections: {}", server.max_connections);
    
    listener
        // Ignore accept errors.
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        // Limit channels to 1 per IP.
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        // For each channel, create a future that serves it.
        .for_each(|channel| {
            let server = server.clone();
            async move {
                // Check if we're at the connection limit
                let current_count = server.connection_count.load(Ordering::SeqCst);
                if current_count >= server.max_connections {
                    println!("Connection limit reached ({}/{}). Rejecting new connection.",
                             current_count, server.max_connections);
                    return;
                }
                
                // Increment connection count
                server.connection_count.fetch_add(1, Ordering::SeqCst);
                let peer_addr = channel.transport().peer_addr().unwrap();
                let count = server.connection_count.load(Ordering::SeqCst);
                println!("New connection from {:?} ({}/{})", peer_addr, count, server.max_connections);
                
                // Clone for drop handler
                let counter = server.connection_count.clone();
                let max_connections = server.max_connections;
                
                // Execute the channel - note that server is cloned because serve() takes ownership
                let fut = channel.execute(server.clone().serve());
                
                // Spawn task to handle this client
                tokio::spawn(async move {
                    // Process client requests
                    fut.await;
                    
                    // Decrement counter when client disconnects
                    let new_count = counter.fetch_sub(1, Ordering::SeqCst) - 1;
                    println!("Client {:?} disconnected. Active connections: {}/{}",
                             peer_addr, new_count, max_connections);
                });
            }
        })
        .await;
    
    Ok(())
} 