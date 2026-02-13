---
name: libp2p-protocols
description: Reference for Elohim custom libp2p protocols, request-response codecs, message serialization (MessagePack), and wire format specification. Use when someone asks "add a new protocol", "implement a codec", "debug message serialization", "wire format", or works with sync/shard protocol handlers.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# libp2p Protocols Reference

The Elohim Protocol defines three custom libp2p protocols for P2P communication.

## Protocol Identifiers

| Protocol | ID | Purpose |
|----------|----|---------|
| Sync | `/elohim/sync/1.0.0` | Content sync (stream positions, doc changes) |
| Shard | `/elohim/shard/1.0.0` | Blob shard transfer (Get/Have/Push) |
| Cluster | `/elohim/cluster/1.0.0` | Cluster coordination (future) |

---

## Wire Format

All protocols use **length-prefixed MessagePack**:

```
┌──────────────────────────────────────────┐
│  4 bytes (big-endian u32)  │  Payload    │
│  Length of payload         │  MessagePack │
│                            │  encoded     │
└──────────────────────────────────────────┘
```

- **Length prefix**: 4 bytes, big-endian `u32`
- **Payload**: MessagePack (via `rmp-serde`)
- **Max message size**: 10 MB (`10 * 1024 * 1024` bytes)

---

## SyncCodec Implementation

The codec implements `libp2p::request_response::Codec`:

```rust
#[derive(Debug, Clone)]
pub struct SyncCodec;

#[async_trait]
impl request_response::Codec for SyncCodec {
    type Protocol = StreamProtocol;
    type Request = SyncMessage;
    type Response = SyncMessage;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where T: AsyncRead + Unpin + Send {
        read_message(io).await
    }

    async fn write_request<T>(&mut self, _: &Self::Protocol, io: &mut T, req: Self::Request) -> io::Result<()>
    where T: AsyncWrite + Unpin + Send {
        write_message(io, &req).await
    }

    // read_response and write_response follow same pattern
}
```

### Read/Write Implementation

```rust
async fn read_message<T>(io: &mut T) -> io::Result<SyncMessage>
where T: AsyncRead + Unpin + Send {
    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    io.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    // Validate size
    if len > MAX_MESSAGE_SIZE {
        return Err(io::Error::new(io::ErrorKind::InvalidData,
            format!("message too large: {} bytes", len)));
    }

    // Read and decode MessagePack payload
    let mut buf = vec![0u8; len];
    io.read_exact(&mut buf).await?;
    rmp_serde::from_slice(&buf).map_err(|e|
        io::Error::new(io::ErrorKind::InvalidData, format!("msgpack: {}", e)))
}

async fn write_message<T>(io: &mut T, msg: &SyncMessage) -> io::Result<()>
where T: AsyncWrite + Unpin + Send {
    let payload = rmp_serde::to_vec(msg)?;
    let len = (payload.len() as u32).to_be_bytes();
    io.write_all(&len).await?;
    io.write_all(&payload).await?;
    io.flush().await
}
```

---

## Sync Protocol Messages (elohim-node)

```rust
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncMessage {
    /// Request events since a stream position
    SyncRequest {
        since: u64,
        limit: Option<u32>,
    },

    /// Response with sync events
    SyncResponse {
        events: Vec<SyncEvent>,
        has_more: bool,
    },

    /// Request document changes
    DocRequest {
        doc_id: String,
        heads: Vec<String>,     // Automerge heads we have
    },

    /// Response with document changes
    DocResponse {
        doc_id: String,
        changes: Vec<Vec<u8>>,  // Automerge change blobs
    },

    /// Announce new local event to peers
    Announce {
        event: SyncEvent,
    },
}
```

### SyncEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncEvent {
    pub position: u64,
    pub doc_id: String,
    pub change_hash: String,
    pub kind: EventKind,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    Local,      // I created this change
    New,        // Just received from peer
    Backfill,   // Historical sync (catching up)
    Outlier,    // DAG gap (reference before content)
}
```

---

## Shard Protocol Messages (elohim-storage)

```rust
pub enum ShardRequest {
    /// Request shard data by hash
    Get { hash: String },

    /// Check if peer has a shard
    Have { hash: String },

    /// Push a shard to peer for replication
    Push { hash: String, data: Vec<u8> },
}

pub enum ShardResponse {
    /// Shard data
    Data(Vec<u8>),

    /// Boolean: peer has shard
    Have(bool),

    /// Replication acknowledged
    PushAck,

    /// Shard not found
    NotFound,

    /// Error
    Error(String),
}
```

---

## Request-Response Lifecycle

### Inbound Request

```
Peer A sends request  -->  Your node receives via swarm event
                           |
                           v
                    ElohimBehaviourEvent::RequestResponse(
                        Event::Message { peer, message: Message::Request { request, channel } }
                    )
                           |
                           v
                    Process request, send response:
                    swarm.behaviour_mut().request_response
                        .send_response(channel, response)?;
```

### Outbound Request

```
Your node sends request:
    let request_id = swarm.behaviour_mut().request_response
        .send_request(&peer_id, request);
                           |
                           v (later)
                    ElohimBehaviourEvent::RequestResponse(
                        Event::Message { peer, message: Message::Response { request_id, response } }
                    )
                           |
                    -- OR --
                    ElohimBehaviourEvent::RequestResponse(
                        Event::OutboundFailure { peer, request_id, error }
                    )
```

---

## MessagePack Serialization

Using `rmp-serde` crate:

```rust
// Serialize
let bytes: Vec<u8> = rmp_serde::to_vec(&message)?;

// Deserialize
let message: SyncMessage = rmp_serde::from_slice(&bytes)?;
```

All message types must derive `Serialize` and `Deserialize` from serde.

---

## Adding a New Protocol

1. Define protocol ID string: `/elohim/your-protocol/1.0.0`
2. Define request/response message types with `Serialize` + `Deserialize`
3. Create codec implementing `request_response::Codec`
4. Add to `ElohimBehaviour` struct as new `request_response::Behaviour<YourCodec>`
5. Handle events in the swarm event loop

---

## Key Files

| File | Purpose |
|------|---------|
| `elohim-node/src/p2p/protocols.rs` | SyncCodec, protocol constants, wire format |
| `elohim-node/src/sync/protocol.rs` | SyncMessage types, SyncEvent, EventKind |
| `holochain/elohim-storage/src/p2p/shard_protocol.rs` | Shard protocol codec |
| `holochain/elohim-storage/src/p2p/sync_protocol.rs` | Storage sync protocol |

## External References

- libp2p request-response: `https://docs.rs/libp2p-request-response/`
- rmp-serde: `https://docs.rs/rmp-serde/`
- MessagePack spec: `https://msgpack.org/`
