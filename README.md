# Mini-Redis — Redis Clone in Rust

A Redis-compatible distributed store built from first principles in Rust, built through the [CodeCrafters Redis challenge](https://codecrafters.io/challenges/redis), using low-level primitives throughout: direct epoll/kqueue via `mio` (no async runtime), custom RESP parser, and leader-follower replication with RDB snapshot transfer.

## Features

- **RESP protocol parser** — Byte-level, sliding-window parser. Handles all RESP data types: simple strings, bulk strings, arrays, integers, booleans, doubles, big numbers, nulls, and errors. Serializes responses back to RESP format.
- **Event-driven I/O with `mio`** — Single-threaded non-blocking architecture. Handles multiple concurrent clients without thread-per-connection overhead. Same model as Redis itself.
- **Core commands** — `PING`, `ECHO`, `SET`/`GET` with `PX`/`EX` expiry, `KEYS`, `CONFIG`, `INFO`, `WAIT`
- **Replication commands** — `REPLCONF`, `PSYNC` (full resync handshake)
- **Leader-follower replication** — Full handshake protocol, RDB snapshot transfer to new replicas, command propagation to multiple replicas, `master_repl_offset` tracking on both sides, `WAIT` with poll timeout

## Architecture

### Event Loop

Single-threaded event loop using `mio::Poll`. All socket operations are non-blocking — reads return `WouldBlock` when no data is available, writes buffer and retry. No `Arc<Mutex<>>` needed: single-threaded means there's nothing to share across threads, so there's nothing to synchronize.

```
  ┌──────────────────────────────────┐ 
  │          mio::Poll.poll()        │◄───────────┐
  └─────────────────┬────────────────┘            │
                    │ OS notifies readiness       │
                    ▼                             │
             ┌─────────────┐                      │
             │ Events loop │                      │
             └──────┬──────┘                      │
                    │                             │
         ┌──────────┴──────────┐                  │
         │                     │                  │
     Token(0)?             Token(n)?             Poll 
   New connection          Data ready             │
         │                     │                  │
         ▼                     ▼                  │
  Accept + register       tcp_handler             │
   new Token(n)                │                  │
         │               RESP parser              │
         │                     │                  │
  replica handshake?    command processing        │
  (REPLCONF/PSYNC)      (cache reads/writes)      │
         │                     │                  │
  register Token(1..n)   Write response ──────────┘
  before client conns
```

> Loop repeats on every `poll()` call — `WouldBlock` on any socket operation registers interest and yields back to the loop.

The critical constraint: `mio` requires non-blocking streams. Blocking anywhere in the event loop stalls all clients. This shaped every design decision.

### RESP Parsing

Sliding-window byte parser that handles TCP's non-determinism — messages can arrive split across multiple reads, or multiple messages can arrive in a single read. The parser maintains position state across partial reads rather than assuming message boundaries align with `read()` calls.

This was the hardest part to get right. TCP delivers a byte stream, not messages. The parser has to work correctly whether a command arrives in one chunk, split at the `\r\n`, or split mid-bulk-string.

### Replication Design

Leader-follower replication with several non-obvious implementation decisions:

```
        mio::Poll registry
              │
   ┌──────────┼──────────────┐
   │          │              │
Token(0)   Token(n)      Token(m)
TcpListener  Client       Replica
             conn          conn
              │              │
         tcp_handler    tcp_handler
         (commands +   (REPLCONF/PSYNC
          cache r/w)    → propagation)
              │              │
              └──── propagate writes ──►
                        offset tracking
```

**Replica tokens, not streams** — Replicas are tracked by their `mio` token rather than storing stream references. This avoids lifetime issues when iterating replicas while also holding a mutable reference to the cache — a constraint the borrow checker surfaces immediately if you try the naive approach.

**`master_repl_offset` on both sides** — The leader tracks bytes propagated; each replica tracks bytes acknowledged. `WAIT` uses this to determine when replicas have caught up. Without tracking on both sides, `WAIT` has no reliable way to determine sync state.

**`WAIT` uses `mio`'s poll timeout** — non-blocking by design, loop stays alive for other events while waiting for replicas to acknowledge.

**RDB transfer** — New replicas receive a full RDB snapshot before the replication stream begins. This is the standard Redis approach: snapshot establishes baseline state, replication stream handles changes from that point forward.

## Key Technical Decisions

**Rust over Python** — Chose Rust deliberately to stay close to the metal. Higher-level languages abstract memory, I/O, and concurrency in ways that obscure what's actually happening. The goal was to feel the constraints directly — ownership, lifetimes, non-blocking I/O — not work around them.

**`mio` over Tokio** — Chose `mio` to understand what event loops actually do before using a framework that abstracts them. Tokio's async/await is the right production choice; `mio` is the right learning choice. They solve the same problem at different abstraction levels.

**`mio` over threads** — Thread-per-connection was the first implementation (still on the `multi-threaded` branch). For I/O-bound workloads an event loop is the natural fit — one thread handles thousands of connections without per-connection thread overhead, using OS-level notification (epoll/kqueue) instead of blocking on each socket.

## Lessons Learned

**TCP is a byte stream, not a message stream.** Initial implementation assumed `read()` would return complete commands — it doesn't. Fixed by tracking a byte offset with a sliding window approach so the parser correctly handles commands split across multiple reads.

**The borrow checker enforces better architecture.** As the codebase grew more complex, ownership errors didn't just flag bugs — they flagged design problems. The tcp_handler structure: parsing, command processing, and propagation, had to be carefully decomposed into functions with clear ownership boundaries. Where lifetimes got complicated, owned values reduced the friction for now. Zero-copy I/O is the natural next step once the structure is solid.

**Non-blocking means non-blocking everywhere — except when it isn't.** One blocking call anywhere in the event loop stalls all clients. `WAIT` was initially implemented as a busy-loop on replica offsets, which tests caught. Fixed by using `mio`'s poll timeout so the loop stays alive for other events while waiting for acknowledgement. The replica-master handshake and RDB transfer are the deliberate exceptions — blocking is acceptable there since it happens once at connection time.

## Running It

```bash
# Start server
cargo run

# Basic usage
redis-cli -p 6379 SET foo bar
redis-cli -p 6379 GET foo  # bar
redis-cli -p 6379 SET temp "expires" PX 5000  # expires after 5s
```

```bash
# Start leader + replica
cargo run
cargo run -- --port 6380 --replicaof localhost 6379

# Verify replication
redis-cli -p 6379 SET foo bar
redis-cli -p 6380 GET foo  # bar
```

## Status

✅ Core server (RESP protocol, event loop, commands)  
✅ Replication (handshake, RDB transfer, multi-replica propagation)  
🔄 Persistence (RDB)  
⏭️ Persistence (AOF)  
⏭️ Performance optimization (zero-copy I/O)

---

Built to understand database and distributed systems internals. The goal was genuine understanding — not just a working demo, but knowing why it works.