[![progress-banner](https://backend.codecrafters.io/progress/redis/e065b20a-7134-4c04-b00b-92a7cf28eb4d)](https://app.codecrafters.io/users/codecrafters-bot?r=2qF)

# Redis Clone in Rust

A lightweight, event-driven Redis implementation in Rust featuring non-blocking I/O and RESP protocol parsing. Built as part of the [CodeCrafters "Build Your Own Redis" Challenge](https://codecrafters.io/challenges/redis) to explore systems programming and distributed systems concepts.

## Features

- **Event-driven I/O with mio**: Single-threaded, non-blocking architecture using Rust's `mio` library for efficient connection handling
- **Core Redis commands**:
  - `PING` - Server health check
  - `ECHO` - Echo back messages
  - `SET` - Store key-value pairs with optional expiry (`PX` for milliseconds, `EX` for seconds)
  - `GET` - Retrieve values with automatic expiry handling
- **RESP Protocol**: Full implementation of Redis Serialization Protocol (RESP) parser supporting multiple data types (simple strings, bulk strings, arrays, integers, booleans, doubles, big numbers, null values, and errors)
- **Expiration Support**: Time-based key expiry with automatic cleanup on access
- **Custom CLI**: Configurable port binding using `clap`

## Architecture

### Event Loop Design

The application uses a single-threaded event loop powered by `mio` for asynchronous I/O operations:

```text
┌─────────────────────────────────────────┐
│         Event Loop (mio::Poll)          │
│                                         │
│  ┌───────────┐      ┌──────────────┐   │
│  │  Server   │─────▶│  Connection  │   │
│  │  Listener │      │   Registry   │   │
│  └───────────┘      └──────────────┘   │
│                            │            │
│                            ▼            │
│                     ┌─────────────┐    │
│                     │   Handler   │    │
│                     │  (TCP I/O)  │    │
│                     └─────────────┘    │
│                            │            │
│                            ▼            │
│                     ┌─────────────┐    │
│                     │    Cache    │    │
│                     │  (HashMap)  │    │
│                     └─────────────┘    │
└─────────────────────────────────────────┘
```

1. **Poll Loop**: Uses `mio::Poll` to monitor multiple TCP connections for readiness events
2. **Connection Management**: Maintains a `HashMap` of client connections indexed by tokens
3. **Non-blocking I/O**: All socket operations return immediately, preventing thread blocking

### RESP Protocol Parsing

The parser (`src/resp/parser.rs`) implements a zero-copy, byte-level parser for the Redis protocol:

- Recognizes RESP data type prefixes (`+`, `-`, `:`, `$`, `*`, etc.)
- Handles delimiter detection (`\r\n`) for message boundaries
- Converts raw bytes into structured `Value` enum types
- Supports serialization back to RESP format for responses

### Command Execution Flow

1. Client connects → registered with event loop
2. Data arrives → `tcp_handler` reads from socket buffer
3. Parser converts bytes → `Value::Array` of command components
4. Pattern matching dispatches to appropriate command handler
5. Handler executes operation on `Cache` (HashMap wrapper)
6. Response serialized to RESP format and written to socket

## Getting Started

### Prerequisites

- Rust 1.70+ (uses 2021 edition features)
- Optional: `redis-cli` for testing

### Running the Server

```bash
# Default port (6379)
cargo run

# Custom port
cargo run -- --port 8080
```

### Testing

```bash
# Using redis-cli
redis-cli -h localhost -p 6379 PING
# Expected: PONG

redis-cli -h localhost -p 6379 SET mykey "Hello"
# Expected: OK

redis-cli -h localhost -p 6379 GET mykey
# Expected: "Hello"

redis-cli -h localhost -p 6379 SET temp "expires" PX 5000
# Expires after 5 seconds
```

#### Testing Multiple Concurrent Connections

The event loop handles multiple simultaneous clients. Test with multiple terminals:

```bash
# Terminal 1
redis-cli -h localhost -p 6379
> SET user1 "Alice"

# Terminal 2 (simultaneously)
redis-cli -h localhost -p 6379
> SET user2 "Bob"

# Terminal 3 (verify both)
redis-cli -h localhost -p 6379
> GET user1  # "Alice"
> GET user2  # "Bob"
```

## Implementation Details

### Why mio Over Threads?

**Architectural Evolution**: Originally implemented with one thread per connection (preserved in `multi-threaded` branch). Migrated to event-driven architecture for better scalability.

Benefits of `mio` (over both threads and Tokio):

- **Scalability**: Handles thousands of connections without thread-per-connection overhead
- **Memory Efficiency**: No per-connection thread allocation (saves 2-8MB per client)
- **Lower-level Understanding**: Chose `mio` over Tokio to learn event loop fundamentals without async/await abstractions
- **Real-world Learning**: Same architecture as Node.js, Nginx, and Redis itself
- **Trade-off**: Must write non-blocking handlers, but simplifies state management

### Expiry Implementation

Lazy expiration using `HashMap<Vec<u8>, (Vec<u8>, Option<Instant>)>` - keys expire on access rather than via background cleanup. Simple but uses memory until accessed.

### Key Trade-offs

- Single-threaded (simplicity over CPU parallelism)
- In-memory only (speed over persistence)
- Lazy expiry (simplicity over proactive cleanup)
- Basic HashMap (O(1) lookups, no compression)

## What I Learned

This project deepened my understanding of several critical systems programming concepts:

### Event Loops and Non-Blocking I/O

- OS event mechanisms (epoll/kqueue abstraction via `mio`)
- Edge vs level-triggered monitoring
- Handling `WouldBlock` errors in non-blocking sockets
- Token-based multi-client connection management

### Protocol Parsing

- Byte-level parsing without external libraries
- UTF-8 validation and zero-copy techniques
- Enum-based protocol representations

### Distributed Systems

- Protocol design trade-offs (text vs binary)
- Client-server communication patterns
- Time-based expiration and idempotency

## Future Work

Continuing through CodeCrafters challenges:

- **Replication**: Leader-follower architecture
- **RDB Persistence**: Snapshot-based durability
- **Streams**: Append-only log structure
- **Pub/Sub**: Publisher-subscriber messaging

Additional ideas: pipelining, transactions (MULTI/EXEC), clustering, benchmarking vs official Redis

---

*This project demonstrates practical systems programming in Rust, showcasing event-driven architecture, protocol implementation, and performance-conscious design decisions.*
