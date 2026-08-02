# elohim-sdk

The Rust client SDK for the Elohim Protocol — a distributed, peer-to-peer content
substrate. Use it to read and write protocol content from your own Rust program
instead of speaking HTTP to a storage peer by hand.

**Who this is for:** a Rust developer building an application, service, or peer runtime
that consumes Elohim Protocol content. You do not need to know Holochain, and you do
not run a DHT to use this crate — you point it at a peer that already does.

**What it is not:** not a server, not a peer implementation, and not a local database.
This crate is the *client half* of the boundary. Something else — a doorway gateway or
an `elohim-storage` peer — holds the content and answers your requests.

**The one idea to hold:** *the same API, several backends.* You construct a
`ContentClient` with a `ClientMode` that says where content lives, and every call after
that is mode-independent. Changing deployment shape changes the constructor, not your
code.

This README is written under the rule the SDK itself enforces: **a published surface may
not claim more than it serves** (concern class C7, advertise/serve symmetry). Every "not
yet" below is deliberate — see [Honest limits](#honest-limits-read-before-you-design-against-this).

## Prerequisites

1. **Rust** 1.83 or newer. This crate declares no `rust-version` of its own; 1.83 is
   the floor its `elohim-seam-contracts` dependency declares, and therefore the
   effective floor for the family.
2. **A reachable peer.** Either a doorway URL (e.g. `https://doorway-alpha.elohim.host`)
   or an `elohim-storage` service URL (e.g. `http://127.0.0.1:8090`). Without one, this
   crate has nothing to talk to — see [Honest limits](#honest-limits-read-before-you-design-against-this).
3. **Registry access.** This crate publishes to the Nexus hosted registry `elohim`, not
   to crates.io. Add it to your `~/.cargo/config.toml` (anonymous read is enabled, so no
   token is needed to build):

   ```toml
   [registries.elohim]
   index = "sparse+https://nexus.ethosengine.com/repository/cargo-internal/"
   ```

   Inside this monorepo, use a path dependency instead:
   `elohim-sdk = { path = "../elohim-sdk" }`.
4. **An async runtime.** Every content call is `async`; the examples assume `tokio`.

## First run

```toml
[dependencies]
elohim-sdk = { version = "0.1.0", registry = "elohim" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

```rust,ignore
use elohim_sdk::{ClientMode, ContentClient, ContentReadable};
use serde::{Deserialize, Serialize};

// 1. Describe the content you want. `content_type` selects the endpoint family;
//    `content_id` is how an instance names itself.
#[derive(Debug, Serialize, Deserialize)]
struct Article {
    id: String,
    title: String,
}

impl ContentReadable for Article {
    fn content_type() -> &'static str { "content" }
    fn content_id(&self) -> &str { &self.id }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 2. Say where content lives. Nothing below this line is mode-specific.
    let client = ContentClient::new(ClientMode::Browser {
        doorway_url: "https://doorway-alpha.elohim.host".into(),
        api_key: None,
    });

    // 3. Read it. `Ok(None)` means the request succeeded and returned nothing —
    //    see the note on two-provenance absence in Honest limits.
    match client.get::<Article>("manifesto").await? {
        Some(article) => println!("{}", article.title),
        None => println!("no article returned"),
    }
    Ok(())
}
```

**Then:** to write content, implement `ContentWriteable` and call `client.save(&article)`
— writes are batched in a `WriteBuffer` and go out on `client.flush()`. To move off the
doorway and onto a peer you run yourself, swap `ClientMode::Browser` for
`ClientMode::Node`; nothing else in the code above changes.

## API surface

| Surface | What it is |
|---|---|
| `ContentClient` / `ClientMode` | mode-aware content access (Browser · Native · Node) |
| `views` | re-export of `elohim-views` — the ts-rs-anchored HTTP wire types |
| `contracts` | re-export of `elohim-seam-contracts` — the concern canon as compile shapes |
| `WriteBuffer` / `WritePriority` / `WriteOp` | write batching with the `for_interactive` / `for_seeding` / `for_recovery` presets |
| `ReachLevel` / `ReachEnforcer` | the notarized reach vocabulary and a local access check |
| `ContentReadable` / `ContentWriteable` / `Syncable` | the traits your content types implement |
| `StorageClient` / `AutomergeSync` (feature `client`) | re-exported from `elohim-storage-client` |
| `Cacheable` / `CacheSignal` / `CacheRule` | re-exported from `doorway-client` |

### Features

| Feature | Effect | Builds? |
|---|---|---|
| `client` (default) | HTTP access to elohim-storage / doorway (`reqwest`, `elohim-storage-client`) | yes |
| `native` | `client` + `rusqlite` for local storage | yes |
| `wasm` | `client`, for the browser target | yes (same code as `client`) |
| `sync` | `client` + `automerge` CRDT sync | **no — does not compile** |
| `full` | `native` + `sync` | **no — inherits `sync`** |

`sync` and `full` are declared but broken: `src/sync/mod.rs` declares a module whose
file has never existed, and `src/traits/syncable.rs` writes single-parameter `Result<T>`
without importing the crate's `Result` alias, so it resolves to `std::result::Result`
and fails to compile. Do not enable either feature; use the `AutomergeSync` re-export
from `elohim-storage-client` (feature `client`) if you need CRDT sync today. This is
listed rather than quietly omitted for the same reason as everything else in this
README.

## Modes

```rust,ignore
use elohim_sdk::{ContentClient, ClientMode};

// Browser — reads the doorway projection. No local storage, no offline.
let client = ContentClient::new(ClientMode::Browser {
    doorway_url: "https://doorway.example.com".into(),
    api_key: None,
});

// Node — local storage that also serves doorways.
let client = ContentClient::new(ClientMode::Node {
    storage_path: "/data/elohim".into(),
    storage_url: "http://127.0.0.1:8090".into(),
    public_url: None,
});
```

## Honest limits (read before you design against this)

- **`ClientMode::Native { sync_url: None }` does not read or write locally yet.**
  `get()` returns `SdkError::InvalidMode` and `flush()` logs a warning and returns
  `Ok(())`. Every mode that currently *works* goes over HTTP to elohim-storage or a
  doorway. Local-SQLite-as-authority is not wired here.
- **No DHT.** This crate's authority is the SQLite/projection plane. Holochain DHT
  participation (attestations, identity, consent) is not in this surface; it lives
  behind the storage service.
- **`ReachEnforcer` is a local check, not enforcement.** It compares a caller's
  declared reach against content reach in-process. The authoritative reach gate is
  the serving peer's, not the client's — do not treat a local `can_access` as
  permission.
- **`get()` still returns `Result<Option<T>>`.** That `None` carries two provenances
  (the item is absent / we never got an answer), which is exactly the collapse
  `contracts::Answer<T>` exists to undo. Migrating these signatures is a later task;
  until then, treat a `None` from a remote mode as *unresolved*, not as absence.

## `contracts` — the inheritance surface

`elohim_sdk::contracts` re-exports [`elohim-seam-contracts`](../seam-contracts), a leaf
crate (zero first-party dependencies, std-only default, WASM-buildable) carrying the
protocol's concern contracts so an external peer runtime **receives** them instead of
re-deriving them one production incident at a time:

- `Answer<T>` — `Present` / `Absent` / `Unreachable`. On a full-arc fleet a local
  `get` miss is `Unreachable` (gossip has not delivered it), never `Absent`.
- `ReasonLabel` — a closed, countable outcome vocabulary, so a decision increments a
  labeled counter through a typed reason instead of a raw string.
- `Arbitrated` / `Quiescent` property harnesses, behind that crate's default-off
  `harness` feature. Enable it in your own `[dev-dependencies]`, not through the SDK,
  so nothing links a test harness at runtime:

```toml
[dev-dependencies]
elohim-seam-contracts = { version = "0.1.0", registry = "elohim", features = ["harness"] }
```

Design:
`genesis/docs/superpowers/plans/2026-08-02-seam-concern-contract-architecture-plan.md`.

## Publishing

The crate family publishes to the Nexus hosted registry `elohim`, declared in the
repo's `.cargo/config.toml`. Every path dependency carries a `version` alongside its
`path` — cargo strips `path` on publish and resolves the version from the registry, so
a bare path dep makes the family unpublishable. Publish order (dependencies first):

```
elohim-views → elohim-seam-contracts → doorway-client → elohim-storage-client → elohim-sdk
```

## License

AGPL-3.0
