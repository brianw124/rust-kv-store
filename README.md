# Key-Value Store Server

A simple key-value store server using tarpc (RPC framework for Rust).

## Project Structure

- `src/shared_types.rs` - Data structures and service definitions shared between client and server
- `src/server.rs` - Server implementation with connection limiting
- `src/client.rs` - Client program with tests for server functionality
- `src/connection_limits_test.rs` - Mock tests for connection limiting

## Features

- In-memory key-value storage (get, set, delete operations)
- Connection limiting (max 10 concurrent connections)
- Per-IP connection limiting (1 per IP)
- JSON serialization for RPC requests and responses
- Thread-safe concurrent client access

## Connection Limiting

The server implements two connection limits:
1. Maximum 1 connection per IP address
2. Maximum 10 concurrent connections across all clients

## Tests

The client includes tests that verify server functionality:

1. **Basic Operations**: Tests the key-value store operations
2. **IP Connection Limit**: Verifies the 1-per-IP connection limit
3. **Mock Connection Limits**: Uses mock server logic to simulate and test the 10-channel limit

## Building and Running

### Build

```
cargo build
```

### Run Server

```
cargo run --bin kv-server
```

### Run Client (with tests)

Open a new terminal and run:

```
cargo run --bin client
```

## API Operations

The key-value store supports the following operations:

1. **Set** - Store a key-value pair
   ```rust
   client.set(ctx, SetRequest { key: "hello", value: "world" }).await?;
   ```

2. **Get** - Retrieve a value by its key
   ```rust
   client.get(ctx, GetRequest { key: "hello" }).await?;
   ```

3. **Delete** - Remove a key-value pair
   ```rust
   client.delete(ctx, DeleteRequest { key: "hello" }).await?;
   ```