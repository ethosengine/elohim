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
code — today, that holds across `Browser` and `Node`; `Native` is not yet a working
backend on its own (see [Honest limits](#honest-limits) and the mode × operation
table in [First run](#first-run)).

This README is written under the rule the SDK itself enforces: **a published surface may
not claim more than it serves** (concern class C7, advertise/serve symmetry). Every "not
yet" below is deliberate — see [Honest limits](#honest-limits).

## Prerequisites

1. **Rust** 1.83 or newer. This crate declares no `rust-version` of its own; 1.83 is
   the floor its `elohim-seam-contracts` dependency declares, and therefore the
   effective floor for the family.
2. **A reachable peer.** Either a doorway URL (e.g. `https://doorway-alpha.elohim.host`)
   or an `elohim-storage` service URL (e.g. `http://127.0.0.1:8090`). Without one, this
   crate has nothing to talk to — see [Honest limits](#honest-limits).
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

```bash
cargo new elohim-demo && cd elohim-demo
```

```toml
[dependencies]
elohim-sdk = { version = "0.1.0", registry = "elohim" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

Paste the block below as the entire contents of `src/main.rs`, then
`cargo run`. This part needs no running stack — `ContentClient::new` never
dials out, and `save()` only queues into the write buffer until something
calls `flush()`. It is marked `rust,ignore` because it is written as a
downstream consumer's own binary (it names `elohim-sdk` as an external
dependency), not a doctest runnable from inside this crate — it mirrors a real
in-crate test instead; see the provenance note just below the block:

```rust,ignore
use elohim_sdk::{ClientMode, ContentClient, ContentReadable, ContentWriteable, WritePriority};
use serde::{Deserialize, Serialize};

// 1. Describe the content you want. `content_type` selects the endpoint family
//    for writes; `content_id` is how an instance names itself. Naming the
//    struct's lowercased type name to match `content_type()` matters — see
//    the read/write note below.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Content {
    id: String,
    title: String,
}

impl ContentReadable for Content {
    fn content_type() -> &'static str { "content" }
    fn content_id(&self) -> &str { &self.id }
}

impl ContentWriteable for Content {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 2. Say where content lives. Nothing below this line is mode-specific.
    let client = ContentClient::new(
        ClientMode::Node {
            storage_path: "/tmp/elohim-demo".into(), // accepted, but not yet
                                                      // read by any code path
                                                      // in this crate — see
                                                      // the table below.
            storage_url: "http://127.0.0.1:8090".into(),
            public_url: None,
        },
        // app_id: arbitrary while you only ever queue/save offline (as here).
        // Once you flush() or get() against a real peer, it must name a
        // namespace THAT PEER already serves — unrelated to the `elohim`
        // registry name above.
        "elohim",
    );

    // 3. Queue a write. No network call happens yet — save() only queues.
    let item = Content { id: "manifesto".into(), title: "Draft".into() };
    client.save(&item).await?;

    // 4. Success signal: the op landed in the Normal-priority queue.
    let counts = client.write_buffer().pending_counts().await;
    assert_eq!(counts.get(&WritePriority::Normal), Some(&1));
    println!("queued 1 write, backpressure {}%", client.backpressure().await);

    Ok(())
}
```

**Expected `cargo run` output:** `queued 1 write, backpressure 0%` (`Node`
mode buffers with a 5000-op ceiling, so 1 queued op rounds down to 0%; the
`assert_eq!` passing silently *is* the primary success signal — the `println!`
is only for eyes). This snippet mirrors `test_queue_and_take` in
`crates/elohim-sdk/src/cache/write_buffer.rs` (`mod tests`): that test queues
ops directly on a `WriteBuffer` and asserts on the resulting batch; this walks
the same queue through `ContentClient::save()` and asserts via
`pending_counts()` instead of `take_batch()`, so it holds without a live
stack.

**Against a live stack:** `client.flush().await?` (after `save()`) and
`client.get::<Content>("manifesto").await?` both move real bytes over HTTP —
neither works without a reachable peer, and there is no offline fallback:
without one, both return `Err(SdkError::Network(..))`.

- **Already have a peer?** Point `storage_url` (`Node`/`Native`) or
  `doorway_url` (`Browser`) at it. The public `https://doorway-alpha.elohim.host`
  from Prerequisite 2 is reachable, but this README does not promise it serves
  anonymous reads or has a `manifesto` id present — verify against your own
  peer, or substitute a content id and `app_id` you know exist there.
- **Inside this monorepo:** `pnpm install` once at the repo root, then
  `pnpm run hc:start` from `app/elohim-app/` brings up the Holochain
  conductor + `elohim-storage` + doorway trio (allow it a few tens of
  seconds to become ready); then point at `http://127.0.0.1:8090`, as in the
  snippet above.

**Writes and reads use different rules for `content_type`, and that is a known
asymmetry, not documented behavior:**

- **Writes** (`save()`/`flush()`) route to
  `POST {storage_url}/db/{app_id}/{content_type}/bulk`, where `content_type`
  is exactly the string your `ContentReadable::content_type()` returns.
- **Reads** (`get()`) route to
  `GET {storage_url}/db/{app_id}/{content_type}/{id}`, where `content_type` is
  instead the **lowercased Rust type name** obtained by reflection
  (`get_from_storage`, `src/client/content_client.rs`) — never the string
  `content_type()` returns.

For `Content` above these happen to agree (`"content"` either way, so
`.../db/elohim/content/manifesto` is both the write and the read URL) — by
naming choice, not by guarantee. Name your struct anything else, or return a
different string from `content_type()`, and only one path moves. Until this
is unified, keep the two in sync yourself. The exact rule for the read side:
`std::any::type_name::<T>()` (module-qualified), keep only the last `::`
segment, then `.to_lowercase()` — **no word-boundary splitting**. A struct
named `ManifestoDraft` reads from `.../manifestodraft/{id}`, not
`.../manifesto_draft/{id}`.

A `404` maps to `Ok(None)`; a `200` deserializes the body as `T` (see
[Honest limits](#honest-limits) on why a remote `None` is not the same claim
as an observed absence).

**Mode × operation, in one table** — what each `ClientMode` variant actually
does today for each call:

| Mode | `get()` | `save()` | `flush()` | `storage_path` |
|---|---|---|---|---|
| `Browser` | GET via `doorway_url` (unauthenticated unless `api_key` set — this README does not know which routes require one) | queues (mode-independent) | POST via `doorway_url` | n/a (no such field) |
| `Node` | GET via `storage_url` | queues | POST via `storage_url` | accepted, never read |
| `Native { sync_url: Some(_) }` | GET via `sync_url` (identical to `Node`) | queues | POST via `sync_url` | accepted, never read |
| `Native { sync_url: None }` | `Err(SdkError::InvalidMode)` | queues (buffer fills normally) | **drains and discards** the queue, logs a warning, returns `Ok(())` | accepted, never read |

`storage_path` is accepted by both `Node` and `Native` but is not read by any
code path in this crate today — pass any value; it currently has no effect
(no local SQLite exists — see Honest limits). Whether `Browser` mode supports
`save()`/`flush()` at all against a real doorway, and what that doorway
requires for `api_key`, is not something this README can currently confirm —
verify against your own deployment.

## API surface

| Surface | What it is |
|---|---|
| `ContentClient` / `ClientMode` | mode-aware content access — `Browser` and `Node` are fully wired over HTTP; `Native` has no local-storage implementation: with `sync_url` set it behaves exactly like `Node`, without one `get()`/`flush()` are stubs (see [Honest limits](#honest-limits)) |
| `views` | re-export of `elohim-views` — the ts-rs-anchored HTTP wire types |
| `contracts` | re-export of `elohim-seam-contracts` — the concern canon as compile shapes |
| `WriteBuffer` / `WritePriority` / `WriteOp` | write batching with the `for_interactive` / `for_seeding` / `for_recovery` presets |
| `ReachLevel` / `ReachEnforcer` | the notarized reach vocabulary and a local access check |
| `ContentReadable` / `ContentWriteable` / `Syncable` | the traits your content types implement |
| `StorageClient` / `AutomergeSync` (feature `client`) | re-exported from `elohim-storage-client` |
| `Cacheable` / `CacheSignal` / `CacheRule` | re-exported from `doorway-client` |

`ContentReadable` (`content_type()` + `content_id()`) is what makes `get()`
callable for a type; `ContentWriteable` extends it with a `validate()` that
defaults to `Ok(())` — `impl ContentWriteable for T {}` is valid and means "no
extra validation" — and is what makes `save()`/`flush()` callable.
`Syncable` is the CRDT-merge contract for the `sync` feature, which does not
compile today (see Features below), so it has no working consumer in this
crate yet.

### Features

| Feature | Effect | Builds? |
|---|---|---|
| `client` (default) | HTTP access to elohim-storage / doorway (`reqwest`, `elohim-storage-client`) | yes |
| `native` | `client` + `rusqlite` dependency — added, but unused; no local-storage code path exists in this crate yet | yes |
| `wasm` | `client`, for the browser target | yes (same code as `client`) |
| `sync` | `client` + `automerge` CRDT sync | **no — does not compile** |
| `full` | `native` + `sync` | **no — inherits `sync`** |

`sync` and `full` do not compile — do not enable either. Use the `AutomergeSync`
re-export from `elohim-storage-client` (feature `client`) if you need CRDT sync
today (`src/sync/mod.rs` declares a module whose file has never existed, and
`src/traits/syncable.rs`'s single-parameter `Result<T>` resolves to
`std::result::Result` instead of the crate's alias). This is listed rather than
quietly omitted for the same reason as everything else in this README.

## Modes

`ContentClient::new` always takes two arguments — the mode, and an `app_id`
scope string (`"lamad"`, `"elohim"`, ...) that namespaces the HTTP path (see
the URL shapes in [First run](#first-run)). All three `ClientMode` variants,
every field:

```rust,ignore
use elohim_sdk::{ContentClient, ClientMode};

// Browser — reads the doorway projection. No local storage, no offline.
let client = ContentClient::new(
    ClientMode::Browser {
        doorway_url: "https://doorway.example.com".into(),
        api_key: None,
    },
    "elohim",
);

// Node — local storage that also serves doorways.
let client = ContentClient::new(
    ClientMode::Node {
        storage_path: "/data/elohim".into(),
        storage_url: "http://127.0.0.1:8090".into(),
        public_url: None,
    },
    "elohim",
);

// Native — every field this variant declares. Only the `sync_url: Some(_)`
// arm does anything today, and it then behaves exactly like Node, over HTTP
// — see Honest limits below before you design against this one.
let client = ContentClient::new(
    ClientMode::Native {
        storage_path: "/data/elohim".into(),
        sync_url: Some("http://127.0.0.1:8090".into()),
    },
    "elohim",
);
```

## Honest limits

Read this before you design against the crate.

- **`ClientMode::Native` has no local-storage implementation at all, and one
  branch silently drops writes.** With a `sync_url` set, it behaves exactly
  like `Node` — same HTTP round trip, no SQLite behind it
  (`get_from_storage` / `flush_to_storage`; see
  `src/client/content_client.rs`) — despite the `native` Cargo feature
  declaring a `rusqlite` dependency that nothing in this crate's source
  currently uses. **Without a `sync_url`:** `get()` returns
  `SdkError::InvalidMode`; `flush()` drains the queued ops out of the write
  buffer, discards them, logs a warning, and returns `Ok(())` — the writes
  are gone, not merely deferred to a later `flush()`. Every mode that
  currently *works* goes over HTTP to elohim-storage or a doorway;
  `ClientMode::supports_offline()` reporting `true` for `Native`/`Node` is
  aspirational, not a behavior guarantee.
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
