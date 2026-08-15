# elohim-sdk

Read and write Elohim Protocol content from a Rust program, instead of hand-rolling
HTTP calls against a peer.

The Elohim Protocol is a peer-to-peer content network. Peers hold the content and
answer for it. This crate is the **client half** of that boundary: it builds the
requests, batches your writes, and hands back typed results. It is not a server, not
a peer, and not a local database.

**What the content is.** A shared corpus that no single server owns — today mostly
learning material: concepts, learning paths, assessments, and the executable Gherkin
scenarios that specify them. A program using this crate reads from that corpus and writes
material back to its own peer. The difference from a database is custody: your peer holds a real copy
and answers for it, so the corpus survives any one operator withdrawing.

Be clear about what this crate does and does not do in that picture. **It speaks to one
peer at a time over HTTP.** It does not gossip, replicate, or discover other peers —
those happen inside the `elohim-storage` service you dial, on its own schedule. See
[What happens after `flush()`](#what-happens-after-flush) for the chain from your write to
another peer holding the bytes, and how much of it this crate can show you.

**Who this serves.** A Rust developer writing an application, service, or peer runtime
that consumes protocol content. You do not need to know Holochain's API, and you do not
write DHT code — you point the client at a service that already does that. You will,
however, have to *run* that service yourself today, and standing it up is the heaviest
thing this document asks of you: see [Prerequisites](#prerequisites) item 5 before you
commit.

**Where the crate is.** Early. Version `0.1.0`, a small surface, and nine sharp
edges — three with copy-paste reproductions — in [Rough edges](#rough-edges). Three of them will change
how you write your first working program, and the first is the one that costs data:

- Against the only publicly reachable endpoint, a `Browser`-mode write reports success
  and discards everything (edge 8).
- `flush()` returns `Ok(())` even when the endpoint rejects your batch (edge 2).
- `Browser`-mode reads currently miss the doorway's cached types (edge 1).

Composed, edges 1 and 8 mean **neither reading nor writing works against a public
endpoint today** — both require an `elohim-storage` service you run yourself. Quickstart
step 3 starts you toward that, with the caveat in [Prerequisites](#prerequisites) item 5.
Read the rough-edges section before you design against this crate.

**Licence: AGPL-3.0** — network copyleft. If you expose functionality built on this over
a service, the licence reaches that service's source. Settle that before you add the
dependency.

**How to read this.** To get running, follow the spine in order: [Vocabulary](#vocabulary)
→ [Mental model](#mental-model) → [Prerequisites](#prerequisites) →
[Quickstart](#quickstart) (three steps; the third is the real round trip) →
[What happens after `flush()`](#what-happens-after-flush) →
[Your types vs. `views` types](#your-types-vs-views-types) →
[How URLs are built](#how-urls-are-built) → [Rough edges](#rough-edges). The rest —
*Authentication* through *The contracts surface* — is reference; read it when you hit the
thing it describes.

Do not skip *How URLs are built* even if you are skimming. It contains one rule that
shapes every content type you will ever declare: **reads derive their URL from your Rust
struct's name, writes from `content_type()`, and if the two disagree only your writes
land.** The quickstart names its struct `Content` because of that rule.

---

## Vocabulary

Six words carry the rest of this document.

| Word | What it means here |
|---|---|
| **peer** | A node that holds content and answers for it. A peer runs an **`elohim-storage`** service, and that service is what you dial. |
| **doorway** | A public gateway sitting in front of a fleet of peers. It is *not itself a peer*: it serves a cached, read-oriented **projection** of what those peers hold. It exists for clients that cannot join the network directly. |
| **endpoint** | Whichever of the two you dialled. This crate talks either to a peer's `elohim-storage` service (`/db/…`, full read and write) or to a doorway (`/api/v1/cache/…`, cached reads). Those are different HTTP surfaces, not two addresses for the same thing, and "endpoint" is the word this document uses when the distinction does not matter. |
| **app scope** (`app_id`) | A namespace string. It becomes a path segment on every `/db/…` request, so it must name a scope the endpoint already serves. The doorway's `/api/v1/cache/…` read route does **not** carry it — see [the mode table](#what-each-mode-does). |
| **conductor** | The Holochain runtime process a peer runs beside its `elohim-storage` service. You never call it from this crate, but you do have to *run* one to have a peer at all, and it is what makes [notarization](#what-happens-after-flush) possible. |
| **reach** | The protocol's audience vocabulary — an eight-level ladder, widest audience first: `commons` (public) → `regional` → `bioregional` → `municipal` → `neighborhood` → `local` → `invited` → `private` (owner only). Content carries a level and so does a requester; **more trust reads more**. Concretely: a `commons` item is readable by every requester; a `private` item only by a `private` requester; and a requester cleared to `municipal` reads `municipal`, `bioregional`, `regional`, and `commons`, but not `neighborhood` or anything narrower. `public` is an accepted spelling of `commons`, and the storage side treats `community` as that same widest tier. **This client neither sends nor enforces any of it** — a row's reach comes from a `reach` field in your serialized JSON, defaulting to `commons` at the endpoint. See rough edge 6, and [What happens after `flush()`](#what-happens-after-flush) for why the tier decides whether your content leaves the machine. |

Two app scopes are in use today: `"lamad"` for learning content and `"elohim"` for
shared infrastructure (resources, sensemaking). **The `content` resource lives under
`lamad`** — that is the scope every example here passes.

Getting this wrong is quiet. There is no registry to query for the full set, and an
unrecognised scope is **not rejected** — it simply selects a namespace with nothing in
it, so reads come back `Ok(None)` exactly as if the item did not exist. If a read you
expect to succeed returns `None`, suspect the scope before you suspect the id. Get the
scope from whoever operates the endpoint.

One warning about the name: `app_id` appears in two different places with two different
namespaces behind it. In `ContentClient::new` it scopes `/db/…` content rows (`lamad`).
In `StorageConfig`, which `AutomergeSync` uses, the same-named field scopes *sync
documents*, and the peer projects content documents under `elohim` there. Same word,
different namespace — see [CRDT sync](#crdt-sync).

## Mental model

```
your Rust program
      │
      │   ContentClient  ──  typed calls:  get()  save()  flush()
      ▼
   ClientMode            ──  which endpoint, and which HTTP surface
      │
      ├─ Node, Native+sync_url ─→  a peer's elohim-storage
      │                             GET/POST  /db/{app}/{type}/…
      │
      └─ Browser ──────────────→  a doorway
                                    GET   /api/v1/cache/{type}/{id}
                                    POST  /db/{app}/{type}/bulk
                                          └─ often unserved by a doorway; see edge 8

      (Native WITHOUT sync_url dials nothing at all — see edge 3.)
```

Three consequences, which are most of what you need to hold:

1. **The calls are uniform; the destination is not.** `get`, `save`, and `flush` never
   change signature. `ClientMode` decides which host and which endpoint family they
   resolve to, so changing deployment shape is a constructor argument rather than a
   rewrite. The modes are *not* freely interchangeable against an arbitrary endpoint,
   though — a doorway and an `elohim-storage` service do not serve the same routes.
2. **Every working call goes over the network.** No mode in this crate reads or writes
   local storage. `save()` buffers in memory until `flush()` sends it; that buffer is
   the only local state there is — and because it is only memory, **a client dropped with
   a non-empty buffer loses whatever was in it.** There is no flush-on-drop.
3. **The client is not the authority.** The endpoint decides what exists, what you may
   see, and whether a write is valid. Anything computed locally here is a convenience,
   never a gate.

## Prerequisites

1. **Rust 1.83 or newer.** This crate declares no `rust-version` of its own; 1.83 is
   the floor its `elohim-seam-contracts` dependency declares, and so the effective
   floor for the family.
2. **An async runtime.** Every content call is `async`. The examples use `tokio`.
3. **Registry access.** This crate publishes to the Nexus registry named `elohim`, not
   to crates.io. Anonymous read is enabled, so no token is needed to build:

   ```toml
   # ~/.cargo/config.toml
   [registries.elohim]
   index = "sparse+https://nexus.ethosengine.com/repository/cargo-internal/"
   ```
4. **An endpoint to dial.** Quickstart step 1 needs none. Steps 2 and 3 do, and what is
   available differs sharply:

   | You want | Available? |
   |---|---|
   | A doorway, reachable over HTTP | Yes. `https://doorway-alpha.elohim.host` is live and open, and step 2 uses it to prove the boundary works. A development deployment with no uptime commitment. |
   | To actually **read** content through this crate | Not against that doorway. `Browser` `get()` structurally misses its cached types (rough edge 1), so a working read needs your own `elohim-storage` service. |
   | To actually **write** content through this crate | Same answer, and worse: writes through a public doorway are discarded silently (rough edge 8). |
   | An `elohim-storage` service | **You have to run one.** It is not on crates.io, has no public container image, and no public deployment serves `/db/…`. Step 3 brings one up. |

   In short: steps 1 and 2 need nothing you don't already have; **every path that moves
   real content requires a peer you run yourself.** There is no offline fallback either
   way: an unreachable configured endpoint returns `Err(SdkError::Network(..))`, while
   `Native` with no `sync_url` refuses `get()` and `flush()` with
   `Err(SdkError::InvalidMode(..))`.
5. **For step 3 only** — running your own peer needs a git checkout of the protocol
   monorepo (<https://github.com/ethosengine/elohim>), Node.js with `pnpm`, and whatever
   that repo's own setup requires for a Holochain conductor. **This document cannot bound
   that last part**, and a cold first run may cost considerably more than the commands in
   step 3 suggest — follow the monorepo's setup instructions before budgeting time. That
   heavier ask is exactly why steps 1 and 2 are built not to need it.

   **On version pairing:** this crate and the monorepo do not release in lockstep, and
   there is no tag that pins `elohim-sdk 0.1.0` to a known-good endpoint build. So the
   "matched crate-and-endpoint release" that makes `views` types trustworthy (see
   [Your types vs. `views` types](#your-types-vs-views-types)) is something you cannot
   currently construct from the outside. Practically: check out the default branch, expect
   some drift, and remember that a shape mismatch shows up as a *silently rejected write*
   (rough edge 2) rather than as a type error. That is the single best reason to keep
   error-level `tracing` on while you develop.

## Quickstart

Three steps, each with its own success signal. Step 1 proves your types are wired and
needs nothing running. Step 2 crosses the network boundary. Step 3 is the first
genuinely useful thing — a write and a read-back against a peer you control.

```bash
cargo new elohim-demo && cd elohim-demo
```

```toml
# Cargo.toml
[dependencies]
elohim-sdk = { version = "0.1.0", registry = "elohim" }
serde = { version = "1", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
```

### Step 1 — describe a content type and queue a write

Paste this as the whole of `src/main.rs`, then `cargo run`.

```rust
use elohim_sdk::{ClientMode, ContentClient, ContentReadable, ContentWriteable, WritePriority};
use serde::{Deserialize, Serialize};

// You declare your own struct — see "Your types vs. `views` types" for why this
// is not optional. The struct NAME is load-bearing, not just its
// `content_type()`: reads derive the URL segment from the Rust type name, writes
// use `content_type()`. Naming this `Content` makes both resolve to `content`,
// which is the resource elohim-storage actually serves.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Content {
    id: String,
    title: String,
}

impl ContentReadable for Content {
    fn content_type() -> &'static str { "content" }
    fn content_id(&self) -> &str { &self.id }
}

// The default `validate()` returns Ok(()); an empty impl means "no extra checks".
impl ContentWriteable for Content {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // The call SHAPES below are identical in every mode; the buffer sizing behind
    // them is not (see [the mode table](#what-each-mode-does)). `new()` never dials out.
    let client = ContentClient::new(
        ClientMode::Node {
            storage_path: "/tmp/elohim-demo".into(), // inert — see rough edge 4
            storage_url: "http://127.0.0.1:8090".into(),
            public_url: None,                        // inert — see rough edge 4
        },
        "lamad", // app scope — this is where `content` lives; see Vocabulary
    );

    // save() queues into the write buffer. No network call happens here.
    client.save(&Content { id: "hello".into(), title: "Draft".into() }).await?;

    let pending = client.write_buffer().pending_counts().await; // HashMap<WritePriority, usize>
    assert_eq!(pending.get(&WritePriority::Normal), Some(&1));

    println!("step 1 ok — 1 write queued, backpressure {}%", client.backpressure().await);
    Ok(())
}
```

Expected output:

```
step 1 ok — 1 write queued, backpressure 0%
```

The `assert_eq!` passing is the real success signal; the `println!` is for your eyes.
`backpressure()` returns a `u8` percentage from 0 to 100. It reads `0` here because
`Node` mode uses the seeding preset (ceiling 5000 operations), so one queued write is
0.02% and truncates to zero.

### Step 2 — cross the network boundary

No local stack needed: this dials the public alpha doorway. Replace `src/main.rs` with:

```rust
use elohim_sdk::{ClientMode, ContentClient, ContentReadable, SdkError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Content {
    id: String,
    title: String,
}

// Read-only here, so no `ContentWriteable` impl is needed.
impl ContentReadable for Content {
    fn content_type() -> &'static str { "content" }
    fn content_id(&self) -> &str { &self.id }
}

#[tokio::main]
async fn main() {
    let client = ContentClient::new(
        ClientMode::Browser {
            doorway_url: "https://doorway-alpha.elohim.host".into(),
            api_key: None, // this doorway's cache route is open; see "Authentication"
        },
        // Inert for this call — Browser reads carry no app scope segment at all.
        // A Node-mode version of this program would need the real scope; see
        // Vocabulary. (`lamad` happens to be that scope, so this is also correct.)
        "lamad",
    );

    let id = "feature-autonomous-entity-worker-scenarios-community";

    match client.get::<Content>(id).await {
        Ok(Some(c)) => println!("GOT THROUGH — item present: {}", c.title),
        Ok(None)    => println!("GOT THROUGH — endpoint answered 'no such item'"),
        // In Browser mode `SdkError::Network` carries BOTH transport failures and
        // HTTP status rejections, so the variant alone cannot tell them apart —
        // the message prefix can. See "Telling the two failures apart" below.
        Err(SdkError::Network(e)) if e.starts_with("HTTP ") => {
            println!("GOT THROUGH — endpoint rejected the request: {e}")
        }
        // This can be a transport failure OR a response-body decode failure.
        Err(SdkError::Network(e)) => println!("request/response failed: {e}"),
        Err(e)                    => println!("client operation failed: {e}"),
    }
}
```

**Telling the two failures apart.** `SdkError::Network` is overloaded on this path: a
transport failure (DNS, refused, TLS, timeout), a response-body decode failure, and a
non-2xx HTTP answer all land in it. The `"HTTP "` prefix identifies only the last case;
the variant alone cannot prove whether the endpoint answered. The two read paths are not
symmetric about this — `Node`/`Native` reads report an HTTP error as
`SdkError::Storage` instead. A `404` never reaches either arm; it becomes `Ok(None)`.

**This step is a boundary smoke test, not a read.** Any `GOT THROUGH` output proves the
endpoint answered; receiving content is not the criterion. A `request/response failed`
line is ambiguous because this API maps both transport and response-decode failures to
`SdkError::Network`; use the `curl` below to resolve which side failed. Data comes back
in step 3.

Expected output — **this is rough edge 1 showing itself, not a mistake on your part**
(verified against the alpha on 2026-08-07; the item id is a live-corpus fixture, so if it
is later removed the `curl` below is the authoritative check, not this program):

```
GOT THROUGH — endpoint answered 'no such item'
```

That item *does* exist on that doorway. Confirm it independently — this prints `200`:

```bash
curl -s -o /dev/null -w '%{http_code}\n' \
  https://doorway-alpha.elohim.host/api/v1/cache/Content/feature-autonomous-entity-worker-scenarios-community
```

The SDK misses it because of a casing mismatch (rough edge 1): the doorway indexes the
type as `Content` and this crate always lowercases it to `content`. If instead you see
`request/response failed`, run the `curl` above. A failed curl points to your network or
the alpha doorway; a `200` means the SDK reached the endpoint but could not decode its
response. A doorway outage is not a crate defect.

### Step 3 — a real round trip against your own peer

This is the first step that returns data. It needs an `elohim-storage` service, which you
run yourself — see [Prerequisites](#prerequisites) item 5 for the extra toolchain.

From a monorepo checkout: `pnpm install` at the repo root, then `pnpm run hc:start` from
`app/elohim-app/`. That brings up a Holochain conductor, an `elohim-storage` service, and
a doorway together. It binds **`elohim-storage` to `http://127.0.0.1:8090`** — the address
every example below uses — and **the doorway to `http://localhost:8888`**. Expect a cold
first run to take tens of minutes, dominated by the conductor toolchain rather than by
anything in this crate. Wait for storage to answer before continuing — this prints `200`
when it is ready:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8090/db/stats
```

Then replace `src/main.rs` with a write followed by a read-back:

```rust
use elohim_sdk::{ClientMode, ContentClient, ContentReadable, ContentWriteable};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")] // the /db wire format is camelCase
struct Content {
    id: String,
    title: String,
    // Optional in the write schema, but the read gate rejects rows without it —
    // skip it and the write lands while every read 404s. See the provenance note.
    #[serde(skip_serializing_if = "Option::is_none")]
    dht_anchor_hash: Option<String>,
}

impl ContentReadable for Content {
    fn content_type() -> &'static str { "content" }
    fn content_id(&self) -> &str { &self.id }
}
impl ContentWriteable for Content {}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ContentClient::new(
        ClientMode::Node {
            storage_path: "/tmp/elohim-demo".into(),
            storage_url: "http://127.0.0.1:8090".into(),
            public_url: None,
        },
        "lamad", // same scope as step 1 — `content` lives here
    );

    client.save(&Content {
        id: "hello".into(),
        title: "Draft".into(),
        dht_anchor_hash: Some("demo-local-anchor".into()),
    }).await?;

    // flush() reports Ok(()) even if the endpoint rejects the batch (rough edge
    // 2), so the read-back below IS the verification — not this line.
    client.flush().await?;

    match client.get::<Content>("hello").await? {
        Some(c) => println!("GOT THROUGH — item present: {}", c.title),
        None    => println!("wrote, but read back nothing — see the diagnostics below"),
    }
    Ok(())
}
```

Expected output:

```
GOT THROUGH — item present: Draft
```

**If you get `read back nothing`,** three different things can cause it, and this tree
separates them. Work down it in order.

1. **Is the service up?**
   `curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8090/db/stats`
   Not `200` → the stack is not ready; go back to `hc:start`.
2. **Is the app scope real?** A scope that the endpoint does not serve is *not* rejected —
   it silently selects an empty namespace, so every read is `Ok(None)`. Ask the list route
   whether anything at all lives in yours:
   `curl -s 'http://127.0.0.1:8090/db/lamad/content?limit=1'`
   An error or a persistent empty result for a scope you believe is seeded points at the
   scope, not at your write. `curl -s http://127.0.0.1:8090/db/stats` reports what the
   service actually holds — that is the authoritative answer for a stack you run yourself.
3. **Did the write land at all?**
   `curl -s -o /dev/null -w '%{http_code}\n' http://127.0.0.1:8090/db/lamad/content/hello`
   Turn on error logging first, because this is the branch `flush()` hides. Add **both**
   crates to `Cargo.toml` — the snippet names types from each:

   ```toml
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["fmt"] }
   ```

   then put this first in `main`:

   ```rust
   tracing_subscriber::fmt().with_max_level(tracing::Level::ERROR).init();
   ```

   - A `tracing` error line at flush time → **the endpoint rejected your batch** and
     `flush()` swallowed it (rough edge 2). The message carries the status and body; most
     often the item did not match the write schema.
   - `404` with **no** error line → the row was accepted and is being **hidden by the
     provenance gate**. Go to step 4.
4. **Did you omit `dhtAnchorHash`?** That is what the provenance gate checks. See the note
   below — a row without it is stored and then invisible to `get()`, which looks identical
   to never having been written.

**A fresh stack is empty.** `hc:start` gives you a running service with no content in it,
which is why step 3 writes its own row before reading one. To get the shared corpus onto
it instead, use the seeding variant — `pnpm run hc:start:seed` — which runs the monorepo's
import pipeline as part of startup. Corpus ingest is that pipeline's job, not this
crate's; `ContentClient` writes individual rows, it does not import a corpus. Once seeded,
list what you actually have with
`curl -s 'http://127.0.0.1:8090/db/lamad/content?limit=5'` and read any of those ids with
`get()`.

## What happens after `flush()`

Worth knowing before you assume your write reached the network, because the chain has
three links and this crate only performs the first.

1. **Your write lands in the peer's database.** That is the whole of `flush()`. It is an
   HTTP POST to one service, and it is the only link this crate participates in.
2. **The peer projects the row into an Automerge document and announces it.** A listener
   inside `elohim-storage` picks up each content write, writes the corresponding document
   (one per content id, under the `elohim` sync namespace), and announces the new head to
   peers it is connected to; a periodic sync round is the fallback if the announcement is
   dropped. This is automatic and server-side, and it is **gated on reach: only the widest
   tier propagates.** The peer projects a row only if its stored reach string is
   `commons` — or one of the two spellings it treats as the same tier, `public` and
   `community`. **`regional` and everything narrower is projected nowhere** and stays on
   the peer that holds it. So one tier of the eight distributes; the other seven do not.

   **Where a row's reach comes from is not `ContentReadable::reach()`.** That method is
   unused (rough edge 6), and the client transmits nothing from it. A row's reach is
   whatever your serialized JSON carries in a `reach` field — and if you omit the field, as
   the step-3 struct does, **the endpoint defaults the row to `commons`**. That default is
   on the propagating side, so an SDK-written row distributes unless you deliberately
   narrow it. To narrow one, add a `reach` field to your struct and serialize a value from
   the ladder; do not override `reach()` and expect it to have any effect.
3. **Notarization is separate and is not triggered by your write.** The real DHT anchor —
   the thing that makes provenance verifiable rather than merely declared — is written by
   the peer's conductor-verified path, not by an HTTP write. That is why step 3 has you
   supply a placeholder anchor to get past the read gate.

Two practical consequences. A single local stack has no peers to announce to, so link 2 is
real but unobservable there — do not read "nothing propagated" as a defect on a one-node
setup. And nothing in this crate reports on links 2 or 3: `flush()` returning `Ok(())`
tells you about link 1 at best (rough edge 2 says even that much is unreliable), never
about propagation.

**Stopping the stack.** The services run in the foreground of the `hc:start` shell —
`Ctrl-C` there brings all three down. Conductor and storage state persists between runs
in the monorepo's working directories, so a restart picks up what you wrote; consult the
monorepo if you need to reset it.

**The provenance note.** An `elohim-storage` service gates external reads on provenance:
a content row is only served once it carries either a DHT anchor or a record of having
been published to peers. A row written straight over HTTP has neither, so it is stored
and then invisible to `get()`. Supplying `dhtAnchorHash` satisfies the gate.

**And the gate is presence, not proof.** The service does not verify the anchor at write
time — it stores whatever string you send and the read gate is a null check on the
column. That is why an obvious placeholder works locally, and it is also why the gate is
not a security boundary: it separates "this row has a provenance story" from "this row
has none." Real verification happens later, when a peer *notarizes* the content — commits
it to the Holochain DHT, where other peers can check the claim independently — and
replaces your declared value with the resulting anchor.

Keep the field on your struct — it is part of the content shape, not a hack. What changes
when you ship is the *value*: on a real ingest it is the content's own content-address,
computed from the bytes, and a later notarization on the peer supersedes it with a real
anchor. `"demo-local-anchor"` is a development-loop expedient standing in for that
computation, and it is the only part of step 3 you should not copy forward.

## Your types vs. `views` types

The crate re-exports `elohim-views` as `elohim_sdk::views` — the generated wire shapes an
endpoint emits (`views::lamad::ContentView`, `ContentWithTagsView`,
`CreateContentInputView`, and peers in the `imagodei`, `shefa`, `qahal`, `epr`, and
`infrastructure` modules; browse them with `cargo doc -p elohim-views`). It is reasonable
to ask why the quickstart declares its own `struct Content` instead. The answer is that
you have no choice:

- `get::<T>()` requires `T: ContentReadable`. That trait is defined in this crate, and
  `views` types are defined in `elohim-views`. From *your* crate both are foreign, so
  Rust's orphan rule forbids `impl ContentReadable for views::lamad::ContentView`. A
  `views` type can never be the `T` in `get::<T>()` from a downstream crate.
- **So: declare your own struct** and implement the two traits on it, exactly as the
  quickstart does. Deserialization ignores unknown JSON fields, so a narrow struct
  against a wide response is fine and normal.
- **Use `views` types** as field types inside your own structs — for example a
  `Vec<views::lamad::ContentGraphNodeView>` you never have to hand-model — and when you
  bypass `ContentClient` by calling `StorageClient` or `reqwest` yourself and
  deserializing the response directly. They are code-generated from the same source as
  the HTTP contract, so within a matched crate-and-endpoint release they cannot drift
  from it. A deployment on a different version still can, so treat them as a strong
  default rather than a cross-version guarantee.

## How URLs are built

Reads and writes derive the type segment by **different rules**. This is the single
most surprising thing in the crate, so it is worth internalizing before you name a
struct.

**Writes** (`save()` then `flush()`) use the string your `content_type()` returns.
Despite living on a trait named `ContentReadable`, that method governs the *write* path
and nothing else:

```
POST {host}/db/{app_id}/{content_type()}/bulk
```

The body is a JSON array of your serialized items, grouped by content type, in camelCase.
The endpoint validates each item against its own content schema — your struct must
serialize into a shape it accepts, or it rejects the whole batch (silently, per rough
edge 2).

**Where that schema is.** In the `views` re-export. For `content`, the accepted write
shape is `views::lamad::CreateContentInputView`: `id` and `title` are the only required
fields, everything else is optional, and `dhtAnchorHash` is the one you will also want
(step 3 explains why). Read it with `cargo doc -p elohim-views` and mirror the fields you
need. Other content types have their own `Create…InputView` in the same module family —
that type *is* the source of truth for what a write may contain.

**Reads** (`get()`) ignore `content_type()` entirely and use the Rust type's own name:
take `std::any::type_name::<T>()`, keep the last `::` segment, and lowercase it. There
is **no word-boundary splitting** — a struct named `ManifestoDraft` reads from
`…/manifestodraft/{id}`, never `…/manifesto_draft/{id}`.

```
GET {storage_url}/db/{app_id}/{lowercased type name}/{id}     # Node, Native+sync_url
GET {doorway_url}/api/v1/cache/{lowercased type name}/{id}    # Browser — no app scope
```

**The resource set belongs to the endpoint, not to you.** You cannot invent a content
type from the client: the `elohim-storage` service serves a fixed list of resources, and
`elohim-views` is its inventory (`cargo doc -p elohim-views` — each writable resource has
a `Create…InputView`). A type name the endpoint does not serve fails exactly the way a
wrong app scope does: reads return `Ok(None)` and writes are rejected silently. The
`ManifestoDraft` name above illustrates the lowercasing rule; it is not a usable resource.

**What to do about it:** name your struct so that its lowercased name equals the
resource you want to read. For `elohim-storage`'s content table that is `content`, so a
struct named `Content` lines up on both paths — which is why the quickstart uses that
name. Name it anything else, or return a different string from `content_type()`, and
only the write path lands. Keep the two in sync yourself.

A `404` from a reachable endpoint becomes `Ok(None)`. A `200` deserializes the body as
`T`.

**Reads are by id only.** There is no list or query method on `ContentClient` —
`get_batch` loops `get` over ids you already have, sequentially, and the first `Err`
aborts the whole batch. Ids come from outside this crate: a seeded corpus, another
service, or a list route called directly: a doorway serves
`GET {doorway}/api/v1/cache/{type}` and an `elohim-storage` service serves
`GET {storage}/db/{app}/{type}`. Mind the casing difference between them — that is
rough edge 1 again, and it applies to the list routes too.

**Wire format.** The `/db/…` surface is camelCase in both directions, so a struct with
multi-word fields needs `#[serde(rename_all = "camelCase")]` (quickstart step 3 shows
this). The doorway's cache route returns whatever the projection stored, which is also
camelCase for the types it serves today.

## Authentication

`api_key` applies to `Browser` mode only. When set, it is sent as
`Authorization: Bearer <key>` on both the cache read and the bulk write. Whether a given
doorway requires one is a deployment question — the public alpha's cache route is open,
so `None` works there. A rejected key surfaces as
`Err(SdkError::Network("HTTP 401 - …"))` from `get()`; on `flush()` it is only logged
(rough edge 2). The other modes have no credential field: an `elohim-storage` service is
expected to be reached over a trusted network path.

## What each mode does

`ContentClient::new` always takes two arguments — the mode, and the app scope string.

```rust
use elohim_sdk::{ClientMode, ContentClient};

// Browser — reads a doorway's cached projection.
let client = ContentClient::new(
    ClientMode::Browser {
        doorway_url: "https://doorway.example.com".into(),
        api_key: None,
    },
    "lamad",
);

// Node — talks to a peer's elohim-storage service. The full read/write path,
// and the mode to reach for unless you have a specific reason not to.
let client = ContentClient::new(
    ClientMode::Node {
        storage_path: "/data/elohim".into(),
        storage_url: "http://127.0.0.1:8090".into(),
        public_url: None,
    },
    "lamad",
);

// Native — reserved for a local-storage path that is not implemented. Only the
// `sync_url: Some(_)` arm does anything; it then behaves like Node ON THE WIRE
// but keeps the `default` buffer preset (1000 ops, not Node's 5000) — see the
// mode table below. There is no reason to choose it today; prefer Node.
let client = ContentClient::new(
    ClientMode::Native {
        storage_path: "/data/elohim".into(),
        sync_url: Some("http://127.0.0.1:8090".into()),
    },
    "lamad",
);
```

|  | `Browser` | `Node` | `Native` + `sync_url` | `Native`, no `sync_url` |
|---|---|---|---|---|
| dials | a doorway | a peer's `elohim-storage` | a peer's `elohim-storage` | nothing |
| `get()` | `GET {doorway}/api/v1/cache/{type}/{id}` | `GET {storage}/db/{app}/{type}/{id}` | identical to `Node` | `Err(SdkError::InvalidMode)` |
| `save()` | queues in memory | queues | queues | queues |
| `flush()` | `POST {doorway}/db/{app}/{type}/bulk` — but see rough edge 8 | `POST {storage}/db/{app}/{type}/bulk` | identical to `Node` | returns `Err(SdkError::InvalidMode)` **without draining** the queue |
| app scope used? | **writes only** — the cache read route has no app segment | reads and writes | reads and writes | n/a |
| credential | `api_key` — see [Authentication](#authentication) | none | none | none |
| write buffer preset | `for_interactive` — 100 ops, flush at 50% | `for_seeding` — 5000 ops, flush at 90% | default — 1000 ops, flush at 80% | default |
| inert fields | — | `storage_path`, `public_url` (rough edge 4) | `storage_path` | `storage_path` |
| works offline | no | no | no | no |

`ClientMode::Browser` is a *destination* choice, not a compile target — see the `wasm32`
note under [Features](#features).

Convenience constructors: `ContentClient::for_lamad(mode)` and `for_elohim(mode)` fix
the app scope; `ContentClient::anonymous_browser(doorway_url)` builds a `Browser` client
in the `lamad` scope.

## What's in the box

| Surface | What it is |
|---|---|
| `ContentClient`, `ClientMode` | Mode-aware content access. `get`, `get_batch`, `save`, `save_immediate`, `flush`, `backpressure`, `write_buffer`, `mode`, `app_id`. |
| `ContentReadable`, `ContentWriteable` | The two traits your content types implement. They are bounds for you to satisfy, so the crate ships no implementations of its own. |
| `WriteBuffer`, `WritePriority`, `WriteOp` | Priority-ordered write batching with last-write-wins dedup and a backpressure signal. Presets live on the public `cache` module as `cache::WriteBufferConfig`. |
| `ReachLevel`, `ReachEnforcer` | The reach ladder and a local comparison helper. Advisory only — see rough edge 6. |
| `ProjectionWarmer` | Intended for peer operators pushing content *into* a doorway's cache. Its routes have no server counterpart today — see rough edge 9. |
| `SdkError`, `Result` | `Network`, `Serialization`, `Storage`, `Sync`, `BackpressureFull`, `InvalidMode`, `Config`. Two more variants exist — `NotFound` and `AccessDenied` — but no `ContentClient` path described here produces them (a 404 becomes `Ok(None)`; reach is never enforced client-side), so do not write a `match` that depends on them. |
| `views` | Re-export of `elohim-views`, the generated wire shapes. See [Your types vs. `views` types](#your-types-vs-views-types). |
| `contracts` | Re-export of `elohim-seam-contracts`. See [The contracts surface](#the-contracts-surface). |
| `StorageClient`, `StorageConfig`, `AutomergeSync`, `SyncResult` | Re-exported from `elohim-storage-client` under the `client` feature. |
| `Cacheable`, `CacheSignal`, `CacheRule`, `CacheRuleBuilder` | Types for declaring how a doorway should cache and invalidate a projection, re-exported from `doorway-client` (available with `client`). `ContentClient` never consults them — they pair with `ProjectionWarmer`, so rough edge 9 applies. Skip unless you are building the doorway-facing side. |

Four call semantics worth stating outright:

- **`save()`** calls `validate()`, then `to_json()`, and queues the result at `Normal`
  priority. It never touches the network. **`to_json()` is what actually goes on the
  wire**, and its default is plain `serde` serialization of your struct — so the normal
  way to fix a rejected batch is to change your struct or its serde attributes.
  Overriding `to_json()` is supported and is the escape hatch when the shape you want to
  send genuinely differs from the shape you want in memory.
- **`save_immediate()`** queues at `High` priority and then calls `flush()`. One flush
  takes at most the mode preset's batch size, priority-first; it can include other queued
  items and is not a delivery confirmation. See rough edge 2.
- **`get_batch(&[&str])`** collects hits into a `HashMap`; misses are simply absent.
- **`queue()`** on `WriteBuffer` (which `save` calls) returns
  `Err(SdkError::BackpressureFull(100))` once the buffer is at its ceiling. Watch
  `backpressure()` — a `u8` from 0 to 100 — and slow down before that happens.

`ContentReadable` requires `content_type()` and `content_id()`, and offers three
defaulted hooks: `reach()` (`"commons"`), `is_cacheable()` (`true`), and `cache_ttl()`
(3600s). Only `cache_ttl()` currently affects behaviour, via `ProjectionWarmer`.
`ContentWriteable` adds `validate()`, which `save()` calls before queueing, and
`to_json()`.

### Buffer presets

| Preset | Ceiling | Batch size | Flush watermark † | Auto-flush interval † |
|---|---|---|---|---|
| `for_interactive` | 100 | 10 | 50% | 100 ms |
| `default` | 1000 | 50 | 80% | 1000 ms |
| `for_recovery` | 2000 | 100 | 75% | 2000 ms |
| `for_seeding` | 5000 | 500 | 90% | 5000 ms |

**† Nothing in this crate acts on those two columns.** There is no background flush task:
`flush()` only ever runs when you call it, and the auto-flush interval is stored and never
read. The watermark is likewise only exposed through `WriteBuffer::should_flush()`, which
the client never consults — you may poll it yourself. **A long-lived client will fill to
its ceiling and start returning `SdkError::BackpressureFull` unless you drive `flush()`
from your own loop or timer.** Treat that as required work, not an optimisation.

**The minimal flush loop.** `ContentClient` is `Send + Sync` but **not** `Clone`, so share
one between your writer and your timer with `Arc`:

```rust
use std::sync::Arc;
use std::time::Duration;

let client = Arc::new(ContentClient::new(/* … */));

// The drive loop. Without something like this, the buffer only ever grows.
let flusher = Arc::clone(&client);
tokio::spawn(async move {
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        tick.tick().await;
        // For configured modes, an empty buffer returns early. Native without
        // sync_url always refuses with InvalidMode; see rough edge 3.
        if let Err(e) = flusher.flush().await {
            // The attempted batch is not requeued. See rough edge 2.
            tracing::error!("flush failed: {e}");
        }
    }
});

// …your writers now call client.save(&item).await? from anywhere.
```

For `Browser`, `Node`, or `Native` with a `sync_url`, one `flush()` attempts at most one
batch. At shutdown, repeat until the pending counts are empty; there is no flush-on-drop.
This proves only that every queued batch was attempted, not that it landed — current
error handling can consume a failed batch (rough edge 2). Native without a `sync_url`
cannot drain by design; choose a configured mode before using this loop.

**The preset is fixed by the mode**, per the mode table — `Browser` gets
`for_interactive`, `Node` gets `for_seeding`, `Native` gets `default`. `ContentClient` has
no constructor or setter that accepts a `cache::WriteBufferConfig`, so `for_recovery` is
not reachable through the client at all today; it is available only if you build a
`WriteBuffer` directly. Choosing a buffer profile therefore means choosing a mode, and if
your traffic does not match its mode's preset, drive `flush()` more aggressively to
compensate.

### Features

| Feature | Turns on | Choose it when |
|---|---|---|
| `client` *(default)* | HTTP access via `reqwest` and `elohim-storage-client`, including the root-level `AutomergeSync` re-export | Always, unless you want types only. |
| `sync` | `client`, plus the same `AutomergeSync` / `SyncResult` pair under the `elohim_sdk::sync` module path | You prefer the module path. It adds no capability over `client`. |
| `wasm` | `client` | Nothing today. It is currently a plain alias for `client`: it adds no wasm-specific code path and does not change reqwest's backend. See the `wasm32` note below before targeting a browser. |
| `native` | `client`, plus a `rusqlite` dependency that no code in this crate uses | Nothing today. It reserves the dependency for the unimplemented local-storage path. |
| `full` | `native` + `sync` | Nothing today, for the same reason. |

`default-features = false` is supported and gives you a types-and-primitives build: the
traits, the whole `cache` module (`WriteBuffer`, `WriteOp`, `WriteBufferConfig` — so the
`for_recovery` preset stays reachable), `ReachLevel`/`ReachEnforcer`, `SdkError`, `views`,
and `contracts`,
with no HTTP client and no `ContentClient`. Useful for a crate that models protocol
content but leaves transport to its caller.

**On `wasm32`:** `ClientMode::Browser` is a *destination* choice — it means "dial a
doorway" — and is unrelated to your compile target. Whether this crate builds and runs on
`wasm32` is **unverified**: the HTTP layer is a native-configured `reqwest` (rustls, with
a request timeout), and nothing in the crate is target-conditional. Treat browser
targeting as unproven and test it before committing to it; the mode name is not a promise
about the target.

## CRDT sync

Content converges as Automerge documents, one document per content id. This crate's
handle on that is **`AutomergeSync`**, re-exported from `elohim-storage-client` and
available with the default `client` feature.

**You need Automerge itself.** The document `AutomergeSync` hands you is an
`automerge::Automerge`, and this crate does not re-export the Automerge API. Automerge is
pre-1.0, so the compatibility unit is the `0.x` line, not the major version — pick the
same one `elohim-storage-client` links or the types will not match:

```toml
# Cargo.toml — add alongside elohim-sdk
automerge = "0.10"
```

Confirm what your build actually resolved with `cargo tree -i automerge`; if it reports
two versions, that is the mismatch and `&doc` will not typecheck.

**Document addressing.** A document is identified by two things: the `app_id` you put in
`StorageConfig` and an opaque **`doc_id`** string. Neither is derived from your Rust types
— nothing connects `ContentReadable::content_id()` to a `doc_id` automatically.

**`StorageConfig`'s `app_id` is not the same namespace as `ContentClient`'s app scope**,
despite the identical name. The content *row* `hello` lives at scope `lamad` on the
`/db/…` surface; the Automerge *document* projected from it lives under `elohim` on the
sync surface. The peer's projector writes every content document into that one namespace.
So the convention to address a projected content document is:

```
app scope = "elohim"          doc_id = "node:{content_id}"
```

So the content row `hello` from the quickstart lands at scope `elohim`, doc id
`node:hello`. Address it any other way and you get a fresh, parallel document that
nothing else syncs with.

This example needs the same running `elohim-storage` service as quickstart step 3:

```rust
use automerge::transaction::Transactable;
use elohim_sdk::{AutomergeSync, StorageClient, StorageConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut sync = AutomergeSync::new(StorageClient::new(StorageConfig {
        base_url: "http://127.0.0.1:8090".into(),
        app_id: "elohim".into(), // the document namespace
        ..Default::default()
    }));

    // `doc` is an `automerge::Automerge`; empty if the peer has none.
    let mut doc = sync.load("node:hello").await?;

    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "title", "Updated")?;
        Ok(())
    })
    .map_err(|failure| failure.error)?;

    sync.save("node:hello", &doc).await?;               // sends only changes since last sync
    let result = sync.sync("node:hello", doc).await?;   // bidirectional merge
    println!("changed: {}, heads: {:?}", result.changed, result.heads);
    Ok(())
}
```

A healthy run prints one non-empty hex head, e.g.
`changed: false, heads: ["a3f1…"]`. `changed: false` is expected here — the merge found
nothing new from the peer because you just sent the only change. An empty `heads` vector
means nothing was written.

`AutomergeSync` exchanges Automerge changes with the endpoint's sync API and tracks known
heads per document, so `save` and `sync` send deltas rather than whole documents.
`exists` and `forget` round out the surface.

Two boundaries worth knowing:

- **Sync is not a per-type trait.** You do not implement anything to make a type
  syncable. `AutomergeSync` operates on Automerge documents addressed by string id, one
  level below your typed structs.
- **The peer, not the client, projects documents into rows.** An `elohim-storage` peer
  runs a projection listener that turns each content write into its Automerge document
  and announces the new head to connected peers. That machinery is server-side; you are
  talking to it, not running it.

Zooming out, the protocol layers convergence like this: Automerge resolves *values*; the
Holochain DHT owns *version lineage* and elects a canonical head; SQLite is a read
projection of the result. This crate reaches only the first and third, through an
endpoint.

## The contracts surface

`elohim_sdk::contracts` re-exports `elohim-seam-contracts`, a leaf crate (zero
first-party dependencies, std-only by default, WASM-buildable) carrying boundary types
the protocol learned the hard way, so your runtime **inherits** them rather than
re-deriving them one incident at a time. Note that the package is
`elohim-seam-contracts` but its library name is `seam_contracts`.

- **`Answer<T>`** — `Present(T)` / `Absent` / `Unreachable`. Absence that was *observed*
  and absence that was never *established* are different facts, and collapsing them into
  `Option<T>` is how a node ends up claiming authority over content it merely never
  received. Reach for this at your own boundaries; this crate's own `get()` does not
  return it yet (rough edge 5).
- **`ReasonLabel`** — a trait you implement on your outcome enum so every decision
  increments a labelled counter through a typed reason instead of a raw string. Ships
  with conformance assertions (`assert_reason_labels_conformant`, `…_stable`,
  `…_discriminating`).
- **Property harnesses** — `check_arbitration` / `assert_arbitration` for winner
  selection, `check_quiescence` / `assert_quiescent` for settling behaviour, and
  liveness checks. They live behind that crate's default-off `harness` feature. Enable
  it in your own dev-dependencies rather than through the SDK, so nothing links a test
  harness at runtime:

  ```toml
  [dev-dependencies]
  elohim-seam-contracts = { version = "0.1.0", registry = "elohim", features = ["harness"] }
  ```

  Keep that version identical to the one the SDK resolves, or you will have two copies of
  the crate and the types will not match across them — the same trap as the Automerge
  version above. Check with `cargo tree -i elohim-seam-contracts`.

## Rough edges

*Verified against `elohim-sdk` 0.1.0, and against `doorway-alpha.elohim.host` as it
stood on 2026-08-07.* If a reproduction below no longer behaves as described, treat that
as the defect having moved rather than as your setup being wrong, and check the
repository (see [Project links](#project-links)) before working around it.

1. **`Browser` mode `get()` cannot reach the doorway's cached types.** The doorway
   indexes documents under their own type spelling (`Content`, capital C) and matches it
   exactly; this crate always lowercases the Rust type name. No Rust type name survives
   `.to_lowercase()` as `Content`, so the read structurally misses and you get
   `Ok(None)`. Verify:

   ```bash
   curl -s -o /dev/null -w '%{http_code}\n' \
     https://doorway-alpha.elohim.host/api/v1/cache/Content/feature-autonomous-entity-worker-scenarios-community   # 200
   curl -s -o /dev/null -w '%{http_code}\n' \
     https://doorway-alpha.elohim.host/api/v1/cache/content/feature-autonomous-entity-worker-scenarios-community   # 404
   ```

   Workaround: call the doorway's cache route yourself with `reqwest`, bypassing
   `ContentClient`; or use `Node` mode against an
   `elohim-storage` service, whose resource names are lowercase and do line up.

2. **Configured modes take a batch before they know it landed, and never requeue it.** A
   transport failure (connection refused, DNS, TLS, timeout) surfaces as `Err`; an HTTP
   `4xx` or `5xx` is logged at `error` level through `tracing` and swallowed as `Ok(())`.
   Either way the taken operations are gone from memory. A batch spanning content types
   can also partly land before a later request fails. Install a tracing subscriber and
   confirm writes by reading them back; neither `flush()` nor `save_immediate()` is a
   delivery acknowledgement.

3. **`Native` without a `sync_url` has no storage backend.** `get()` and `flush()` return
   `SdkError::InvalidMode`; `flush()` refuses before taking a batch, so queued operations
   remain available. The configuration is honest but still not useful for persistence —
   choose `Node` or supply a `sync_url`.

4. **Nothing here is offline-capable, and several declared surfaces are inert.**
   `storage_path` (on `Node` and `Native`) and `public_url` (on `Node`) are accepted and
   never read; the `native` feature's `rusqlite` dependency is unused;
   `WriteBufferConfig`'s `auto_flush_ms` is stored and never read and its `high_watermark`
   is never consulted by the client, so **there is no background flush task despite the
   configuration implying one**; and `ClientMode::supports_offline()` reports `true` for
   `Native` and `Node`, which no behaviour in this crate backs up. Pass any values you
   like for the inert fields, and drive `flush()` yourself.

5. **`get()` returns `Result<Option<T>>`, and `Ok(None)` is narrower than it looks.** It
   means *the one endpoint you dialled answered 404*. It does not mean the content is
   absent from the network — another peer may hold it, and the endpoint you asked may
   simply not have received it yet. Against a doorway the gap is wider still, since a
   doorway answers from a cached projection rather than from the content itself. That is
   exactly the collapse `contracts::Answer<T>` exists to undo; migrating these signatures
   is future work. Until then, treat `None` as "unresolved at this endpoint."

6. **Reach is declared but not wired.** `ContentClient` constructs a `ReachEnforcer`
   internally, but no call path reads it and there is no accessor — and the client never
   transmits a requester reach to the endpoint. `ContentReadable::reach()` is likewise
   declared and unused. You can use `ReachEnforcer` directly as a local comparison, but
   even then it is not enforcement: the authoritative reach gate is the serving peer's.
   Never treat a local `can_access` as permission. To actually set a row's reach, serialize
   a `reach` field — see [What happens after `flush()`](#what-happens-after-flush), which
   also explains why that choice decides whether the row propagates at all.

7. **No DHT participation** *(scope, not a defect — listed here because readers expect to
   find it in this section)*. Identity, attestations, and consent live on the Holochain
   DHT behind the storage service, not in this surface. This crate's plane is the content
   projection.

8. **A `Browser`-mode write against the public alpha vanishes silently.** This is the
   composite of two facts above, and it is worth stating on its own because it is the
   most likely way to lose data with this crate. `Browser` `flush()` posts to
   `{doorway}/db/…`; `doorway-alpha.elohim.host` does not serve that route and returns
   `404`; rough edge 2 swallows the `404` and returns `Ok(())`. So a `Browser` client
   pointed at the only publicly reachable endpoint reports every write as successful while
   discarding all of them. Verify the missing route:

   ```bash
   curl -s -o /dev/null -w '%{http_code}\n' https://doorway-alpha.elohim.host/db/stats   # 404
   ```

   Whether *your* doorway serves `/db/…` is a deployment question, so check before you
   write through one. The doorway `hc:start` brings up locally **does** serve it, which
   makes `Browser` mode testable end-to-end on your own stack even though the public alpha
   is not:

   ```bash
   curl -s -o /dev/null -w '%{http_code}\n' http://localhost:8888/db/stats   # 200 locally
   ```

9. **`ProjectionWarmer` has no server counterpart.** It posts to
   `{doorway}/projection/warm` and `{doorway}/projection/invalidate`, and no current
   doorway serves either route. Against the public alpha both return `404`:

   ```bash
   curl -s -o /dev/null -w '%{http_code}\n' -X POST \
     -H 'Content-Type: application/json' -d '{"type":"content","id":"x","data":{}}' \
     https://doorway-alpha.elohim.host/projection/warm   # 404
   ```

**Is this usable in production today?** Honestly: not as a dependency you would ship on.
The read/write path works against a peer you run, and the crate is a reasonable way to
build against the protocol while it settles — but with eight known defects (edge 7 above
is scope, not a defect), no published `elohim-storage`, and a `0.1.0` surface that will
move, treat it as a development platform rather than a stable API.

**So what does it still buy you over `reqwest`?** Four things that survive every edge
above. The `views` re-export and `ContentReadable`/`ContentWriteable` give you the wire
shapes and a typed boundary instead of hand-modelled JSON. The write buffer gives you
priority ordering and last-write-wins dedup, which is real work you would otherwise
write. `ClientMode` makes the destination a constructor argument, so the same code runs
against a doorway or a peer. And `contracts` hands you `Answer<T>` and `ReasonLabel` —
boundary types you would otherwise re-derive from your own incidents. What it does *not*
yet buy you is reliable error reporting on writes, offline capability, or reach
enforcement; for those you are still reading this section.

## Next steps

- **Write your own flush loop.** Nothing flushes the buffer for you (see the note under
  [Buffer presets](#buffer-presets)), and the preset comes with the mode rather than being
  selectable. A timer or a `should_flush()` poll is the smallest thing that keeps a
  long-lived client from filling to `BackpressureFull`.
- **Adopt `contracts::Answer<T>` at your own boundaries** even though `get()` does not
  return it yet. It is the cheapest place to stop conflating "absent" with "never
  answered."
- **Reach for `AutomergeSync`** when you need convergent documents rather than
  last-write-wins rows — and address them with the scope/`doc_id` convention above.
- **Install a `tracing` subscriber before you trust anything.** Several failure paths in
  this crate — including every rejected write — report only through `tracing`. The
  two-line setup is in quickstart step 3's diagnostics ladder.

## Project links

- **Source, issues, and protocol documentation:**
  <https://github.com/ethosengine/elohim>. This crate lives at `crates/elohim-sdk/`. That
  repository is where to check whether a rough edge above has been fixed, and where to
  report a new one.
- **API reference:** `cargo doc --open -p elohim-sdk`. Every type in
  [What's in the box](#whats-in-the-box) carries rustdoc; this README covers only the
  parts whose behaviour is not obvious from a signature.
- **Public alpha doorway:** `https://doorway-alpha.elohim.host` — a live endpoint for
  cached reads, used by quickstart step 2. A development deployment, not a service-level
  commitment.

## Publishing

Maintainer material; skip it if you are consuming the published crate.

The crate family publishes to the Nexus registry `elohim`. Every path dependency also
carries a `version` — cargo strips `path` on publish and resolves from the registry, so
a bare path dep makes the family unpublishable.

The family is a small DAG, not a chain. `elohim-views`, `elohim-seam-contracts`, and
`doorway-client` have no first-party dependencies and can go in any order. Then
`elohim-storage-client`, which needs `elohim-views`. Then `elohim-sdk`, which needs all
four.

## License

AGPL-3.0
