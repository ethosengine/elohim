---
title: "SDK SURFACE — Runtime / Transport / Deploy: the edge-device substrate the SDK runs on"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT code, NOT a decision
author: rust-architect (truth layer)
extends:
  - crates/elohim-sdk            # ClientMode facade (Browser/Native/Node) — the seam this surface completes
  - crates/elohim-storage-client # the HTTP boundary to a running node
  - crates/doorway-client        # the web2 on-ramp
  - steward/node                 # elohim-node always-on runtime
  - steward/device               # the Tauri shell that already spawns the sidecar
  - elohim/sdk/storage-client-ts # the ts-rs TS boundary consumers connect through
grounds_on:
  - ESCALATED-ARCHITECTURE-2026-06-14.md   # two quilts, one Governor, one Commitment
  - RECURSIVE-ARCHITECTURE-2026-06-14.md   # CoverageRollup, limit_owner, the empty center
forest:
  - project_hub_optional_floor  # one device, no hub, full participant — the load-bearing invariant
do_not_cite_seal: true
---

# SDK SURFACE — Runtime / Transport / Deploy

> The other SDK surfaces answer *"what do I call?"* This one answers *"what does it run on, and can a
> lone laptop in a village with no hub and no nearby doorway operator be a complete participant?"* —
> the hub-optional floor (`project_hub_optional_floor`). Every higher SDK surface (care-ledger,
> governance, valueflow) is a passenger; this surface is the vehicle. If the vehicle requires a hub to
> move, the whole agency gradient collapses — because whoever runs the required hub becomes the rent
> extractor the protocol exists to refuse.

---

## PART 1 — PURPOSE ON THE AGENCY GRADIENT

**Position: the RUNTIME / ON-RAMP floor — the substrate under the gradient, not a point on it.**

This surface is neither human-sovereign nor veil-holding. It is the **deterministic substrate-floor**
(`project_substrate_floor_elohim_ceiling`) on which both are deployed: it moves bytes, persists truth,
embeds the conductor, dials the mesh, and survives offline. It carries **no discernment** — no care
attribution, no governance verdict, no aggregation. Those ride *on top* of it (the elohim ceiling).

But it is the surface where the gradient's two downward invariants become **physically true or
physically false**, because they are properties of *where compute and keys live*:

- **PERSON-KEEPS-THEIR-OWN-NAMING** is only real if the person can run a *complete node on their own
  device* — holding their own Ed25519 agent key (`steward/node/src/p2p/transport.rs:544` persists it to
  `<data_dir>/node_key`), validating peers, and reading their own content with **no hub and no doorway
  in the loop**. If basic participation required a hub, the hub operator would hold the naming. The
  runtime SDK's whole job is to make the no-hub path the *default* path.
- **DIGNITY-FLOOR precedence** is only real if a node that loses its hub, its doorway, and its network
  **keeps functioning offline** — reads its own truth, queues its own writes, and reconciles when the
  mesh returns. Availability that depends on an upstream is a dignity floor a landlord can foreclose.

**What this surface must NEVER do (the gradient guard):**

1. **Never make a hub or doorway a precondition for participation.** Hubs and doorways are *graduations*
   that add convenience and scale (`project_hub_optional_floor`); the SDK's `Runtime::launch()` MUST
   succeed and serve the local human with `transport: Offline` and zero peers configured. A runtime
   builder method whose absence breaks the lone-laptop path is a design smell — re-check it.
2. **Never let the runtime layer adjudicate.** No care-minting, no governance, no `Governor::check()`
   business logic lands here. The runtime *hosts* the elohim-compute Governor
   (`elohim/elohim-storage/src/services/arc_actuator.rs` → the lifted `trait Governor`); it never *is*
   one. Transport/deploy code that starts reasoning about whose line it honored has climbed out of the
   floor and into the ceiling — push it back down to a service.
3. **Never collapse the two quilts into one deployment knob.** The runtime config MUST expose the
   trust-plane (lean DHT arc) and byte-plane (RS(4,7) custody) as *separately sized* dials
   (ESCALATED Part 1), because the entire RAM-fits-on-a-laptop guarantee
   (`project_per_node_memory_is_conductor_authority_arc`) depends on the corpus NOT being DHT entries.
4. **Never read snake_case into the consumer.** The TS runtime client wraps the ts-rs boundary; it adds
   *connection lifecycle* only, never wire transforms (CLAUDE.md gospel).

---

## PART 2 — THE CONCRETE API

The surface is **one new Rust facade crate** + **one new TS connection helper** + **a thin extension to
the existing `ClientMode` enum**. The developer's primary call is `Runtime::launch(spec)` — *"give me a
running node I can talk to,"* hub or no hub.

### 2.1 Rust — `elohim-runtime` (NEW thin crate, sibling to `crates/elohim-sdk`)

This is the missing piece. Today the lone-laptop bootstrap exists **only inside the Tauri shell**
(`steward/device/src-tauri/src/storage.rs:34-71` spawns the storage sidecar; `lib.rs:88` embeds the
conductor via `tauri-plugin-holochain`). There is **no headless, framework-agnostic crate that says
"launch me a complete local node"** — a CLI tool, a test harness, or a third-party Rust app must
re-implement the spawn/health-check/wire-up dance. `elohim-runtime` lifts that one pattern out of Tauri
so the floor is reachable without Tauri.

```rust
// crates/elohim-runtime/src/lib.rs

/// How this process obtains a running node. The variant order encodes the gradient floor:
/// `Embedded` (lone laptop, no hub) is first-class and the default for `RuntimeSpec::laptop()`.
pub enum RuntimeTarget {
    /// Embed the whole stack IN THIS PROCESS — conductor + storage + p2p.
    /// The hub-optional floor, headless. No sidecar, no Tauri, no doorway.
    Embedded {
        data_dir: PathBuf,
        /// Trust-plane arc sizing (lean DHT). None = laptop default {0,1}. (ESCALATED Part 1)
        trust_plane: TrustPlaneConfig,
        /// Byte-plane custody sizing (RS(4,7), RAM-independent of the DHT arc).
        byte_plane: BytePlaneConfig,
    },
    /// Spawn elohim-storage as a managed child (lifts steward/device/src-tauri/src/storage.rs).
    Sidecar { binary: PathBuf, config: SidecarConfig },
    /// Attach to an already-running node over HTTP (the elohim-storage-client path).
    Attached { storage_url: String },
    /// Browser / thin-client: route through a doorway (graduation, NOT the floor).
    Doorway { doorway_url: String, api_key: Option<String> },
}

/// Declarative launch spec. `laptop()` is the floor; everything else is a graduation builder.
pub struct RuntimeSpec {
    pub target: RuntimeTarget,
    pub transport: TransportPlan,   // libp2p | iroh | offline — runtime-selected, both co-equal
    pub app_id: String,             // "lamad" | "elohim" | a care-ledger app's id
}

impl RuntimeSpec {
    /// THE FLOOR. One device, no hub, no doorway, full participant. Embedded everything,
    /// transport=Offline-tolerant (dials the mesh if present, serves locally if not).
    pub fn laptop(data_dir: impl Into<PathBuf>, app_id: &str) -> Self { /* ... */ }

    /// Graduation: managed sidecar (what Tauri does today, now reusable headless).
    pub fn sidecar(binary: impl Into<PathBuf>, app_id: &str) -> Self { /* ... */ }

    /// Graduation: thin browser client through a doorway. Honest about its dependency.
    pub fn doorway(url: &str, app_id: &str) -> Self { /* ... */ }

    pub fn with_bootstrap(self, peers: Vec<String>) -> Self { /* ... */ } // ADD reach, never gate it
    pub fn with_transport(self, plan: TransportPlan) -> Self { /* ... */ }
}

/// A live node handle. Drop = graceful teardown (child killed via kill_on_drop, as Tauri does today).
pub struct Runtime { /* opaque */ }

impl Runtime {
    /// THE PRIMARY DEVELOPER CALL. Boots/attaches the stack, health-checks readiness, returns a handle.
    /// Succeeds with ZERO peers and NO doorway — that success is the hub-optional floor, enforced.
    pub async fn launch(spec: RuntimeSpec) -> Result<Runtime, RuntimeError>;

    /// The wired SDK client against this node (the existing facade, now handed a live target).
    pub fn client(&self) -> ContentClient;          // elohim_sdk::ContentClient (extended ClientMode)
    pub fn storage(&self) -> StorageClient;          // elohim_storage_client::StorageClient
    pub fn health(&self) -> RuntimeHealth;           // transport, peer_count, offline_ok: bool
    pub async fn shutdown(self) -> Result<(), RuntimeError>;
}
```

`TransportPlan` is the runtime-selection seam the truth layer already lives by: services are
transport-neutral, libp2p and iroh adapters delegate (gospel). The runtime SDK *surfaces the dial*; it
does not re-architect per stack.

```rust
pub enum TransportPlan {
    /// Auto-select per plane (chatty→iroh, bulk→parity-tested) at launch. Recommended default.
    Auto,
    Libp2p,                          // steward/node/src/p2p/transport.rs ElohimBehaviour
    Iroh,                            // elohim/elohim-storage/src/p2p_iroh
    /// No mesh dialed. Local truth only. THE FLOOR'S worst-case still-a-participant mode.
    Offline,
}
```

### 2.2 Rust — extend the existing `ClientMode` (no fork; one new variant)

`crates/elohim-sdk/src/client/content_client.rs:19-48` already has `Browser | Native | Node`. The
runtime crate needs `ContentClient` to accept a handle to an *embedded* in-process node, which today's
three variants can't express (they all assume an out-of-process URL or a SQLite path with no embedded
conductor). One additive variant, mirroring the existing `supports_offline()` discipline at `:52`:

```rust
// crates/elohim-sdk/src/client/content_client.rs — ADDITIVE
pub enum ClientMode {
    Browser { /* ...unchanged... */ },
    Native  { /* ...unchanged... */ },
    Node    { /* ...unchanged... */ },
    /// NEW: in-process embedded node (conductor + storage + p2p in this binary).
    /// supports_offline() => true; requires_doorway() => false.
    Embedded { runtime: Arc<RuntimeInner> },
}
```

### 2.3 TypeScript — `RuntimeClient` (NEW helper in the existing storage-client-ts)

The browser/Tauri consumer already has `StorageClient` (`elohim/sdk/storage-client-ts/src/client.ts:39`)
and the Angular `IConnectionStrategy` seam
(`app/elohim-library/.../connection/connection-strategy.ts:52`, modes `auto | doorway | direct`). The
gap: there is no single TS call that *resolves which runtime the consumer is in and hands back a
ready-to-use client without the consumer deciding*. `RuntimeClient.connect()` is that call — a thin
wrapper, zero wire transforms, that picks the strategy and exposes the same `StorageClient` surface:

```typescript
// elohim/sdk/storage-client-ts/src/runtime.ts  (NEW — re-exported from index.ts)
import { StorageClient } from './client';

export type RuntimePresence =
  | { mode: 'tauri'; storageUrl: string }    // local sidecar at :8090 (steward/device)
  | { mode: 'embedded'; storageUrl: string } // in-process elohim-runtime exposing HTTP
  | { mode: 'doorway'; doorwayUrl: string }; // graduation: thin browser client

export class RuntimeClient {
  /** THE TS PRIMARY CALL. Detects presence (Tauri IPC? localhost:8090? doorway?) and connects.
   *  Returns a StorageClient already pointed at the right runtime — consumer never chooses. */
  static async connect(hint?: Partial<RuntimePresence>): Promise<RuntimeClient>;

  get storage(): StorageClient;     // the existing wire-typed client, unchanged
  get presence(): RuntimePresence;  // honest about whether a doorway is in the loop
  get offlineCapable(): boolean;    // true for tauri|embedded — the gradient guard, surfaced to UI
}
```

`offlineCapable` is load-bearing for the gradient: the Angular layer reads it to *show the person*
whether their participation depends on someone else's hub — the felt face of the hub-optional floor.

### 2.4 Two-quilt config (the deployment dial the runtime MUST expose)

```rust
pub struct TrustPlaneConfig {  // the lean, high-integrity DHT plane
    /// Authority arc. Laptop default = {0,1} — a REAL arc of a lean plane (ESCALATED Part 1).
    pub arc_factor: ArcFactor,
}
pub struct BytePlaneConfig {   // the heavy RS(4,7) byte-quilt, RAM-independent of the arc
    pub custody_ceiling_bytes: Option<u64>,  // min(probes, allocation, ceiling) — operator-set
    pub replication_floor: u8,               // r_floor per shard; RS(4,7) => any 4 of 7 reconstruct
}
```

These are *deployment* knobs, not governance. The runtime allocates capacity as
`min(probes, allocation, ceiling)` (`project_storage_as_pod_operator_sets_virtual_limits`); whether a
custody gap is *acceptable* is a Governor decision in the ceiling, not here.

---

## PART 3 — EXISTS vs NEW

### EXISTS (wrap, do not rebuild) — the floor is ~80% already shipped

| Capability | Where it lives today | How the runtime SDK uses it |
|---|---|---|
| Embedded conductor + storage in one process | `steward/device/src-tauri/src/lib.rs:88` (tauri-plugin-holochain), `storage.rs:34-71` (sidecar spawn, `kill_on_drop`) | `RuntimeTarget::Embedded`/`Sidecar` **lift this pattern out of Tauri** into a headless crate |
| The full P2P swarm (libp2p) | `steward/node/src/p2p/transport.rs:42-73` `ElohimBehaviour`; key persisted `:544` | `TransportPlan::Libp2p` selects it; runtime owns lifecycle, not protocol |
| The iroh stack | `elohim/elohim-storage/src/p2p_iroh/` | `TransportPlan::Iroh` / `Auto` |
| Mode-aware client facade | `crates/elohim-sdk/src/client/content_client.rs:19-60` (`ClientMode`, `supports_offline()`) | extended with one additive `Embedded` variant |
| HTTP boundary to a node | `crates/elohim-storage-client` + `elohim/sdk/storage-client-ts/src/client.ts:39` | `Runtime::storage()` / `RuntimeClient` wrap it |
| Doorway on-ramp | `crates/doorway-client`; `ClientMode::Browser` | `RuntimeTarget::Doorway` — the honest graduation |
| Local node CLI flags | `elohim/elohim-storage/src/main.rs` (`--embedded-conductor`, `--enable-p2p`, `--storage-dir`, `--p2p-bootstrap-nodes`) | `SidecarConfig` maps `RuntimeSpec` → these args (no new CLI surface) |
| Angular connection seam | `app/elohim-library/.../connection/connection-strategy.ts:52` (`auto/doorway/direct`) | `RuntimeClient` is its headless/Tauri sibling; both feed `StorageClient` |
| Two-quilt primitives | `Content.blob_cid`, `ShardManifest`, RS math (ESCALATED Part 1, ~80% in-substrate) | `TrustPlaneConfig`/`BytePlaneConfig` are the *deployment dials* over them |

### NEW (thin, additive)

1. **`crates/elohim-runtime`** — the headless launch facade. Genuinely new *as a crate boundary*, but
   it is a **re-home of the Tauri spawn logic + a builder over existing flags**, not new runtime
   behavior. The boundary is proven: today the only way to get a complete local node from Rust is to be
   the Tauri app. A CLI, an integration test, or a third-party app cannot. That is the missing seam.
2. **`ClientMode::Embedded`** — one additive enum variant (CLAUDE.md additive-wire discipline).
3. **`elohim/sdk/storage-client-ts/src/runtime.ts`** — one TS helper, re-exported from `index.ts`.
4. **`TrustPlaneConfig`/`BytePlaneConfig` deployment structs** — additive config surfacing the
   two-quilt split as separate dials.

### Marked FORK status

- **NO fork of Holochain, libp2p, or iroh.** This surface is pure composition (consistent with both
  architecture syntheses: "No fork of Holochain core").
- **The one *conditional* transport fork lives elsewhere and is only *surfaced* here:** the
  `kitsune2_elohim_gossip` fractional-arc module (ESCALATED #17/R5) is GATED on the two-quilt split
  proving a `{0,1}` laptop arc insufficient. The runtime SDK must expose `TrustPlaneConfig.arc_factor`
  so the probe that decides that fork is *runnable from the SDK* — but the SDK does not take the fork.
- **NOT a fork, but flagged for blessing:** `RuntimeTarget::Embedded` running a real conductor headless
  (outside Tauri) leans on `tauri-plugin-holochain`'s conductor bootstrap. Lifting it cleanly may need
  the conductor-init extracted from the plugin into a plugin-free helper. That extraction is in-tree
  refactor, not a dependency fork — but it is the one piece with real engineering risk; bless it as a
  spike before committing the headless-`Embedded` slice.

---

## PART 4 — THE MINIMAL BUILDABLE SLICE

**The smallest real thing: `RuntimeSpec::sidecar()` + `Runtime::launch()` + `Runtime::storage()`,
proving the lone-laptop floor headlessly — no Tauri, no doorway, offline-tolerant.**

This is buildable *today* because every dependency exists: the sidecar binary, the spawn-and-health-check
pattern (`steward/device/src-tauri/src/storage.rs:39-71`), and the HTTP client
(`crates/elohim-storage-client`). The slice is *moving 40 lines out of Tauri into a reusable crate* plus
a builder. It defers the harder `Embedded` (in-process conductor) variant behind the Part-3 spike.

What it lets a developer do, the first real thing, today:

```rust
// A CLI tool, a test, or a care-ledger app's bootstrap — no Tauri, no hub, no doorway.
use elohim_runtime::{Runtime, RuntimeSpec};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Sidecar floor: spawn elohim-storage as a managed child, offline-tolerant.
    let rt = Runtime::launch(
        RuntimeSpec::sidecar("./elohim-storage", "care-ledger")
            // .with_bootstrap(vec![]) — ZERO peers. This MUST still succeed. That is the floor.
    ).await?;

    assert!(rt.health().offline_ok);   // the hub-optional invariant, asserted in code

    // Same StorageClient the browser and Tauri use — one API across all modes.
    let docs = rt.storage().list_documents(Default::default()).await?;
    println!("local node live, {} docs, {} peers", docs.total, rt.health().peer_count);

    rt.shutdown().await?;             // graceful teardown (Drop also kills the child)
    Ok(())
}
```

The first **example app fragment** it enables — a household care-ledger's first boot, on one laptop:

```typescript
// care-ledger app, browser OR Tauri, the consumer never chooses the runtime
import { RuntimeClient } from '@elohim/storage-client';

const rt = await RuntimeClient.connect();          // detects sidecar / embedded / doorway
if (!rt.offlineCapable) showHonestBanner('Hosted via a doorway — running your own node is one tap away.');
const ledger = rt.storage;                          // wire-typed StorageClient, unchanged
await ledger.applyChanges('care:household-acme', firstCareEntryBytes);  // works offline, syncs later
```

`offlineCapable` driving an honest banner is the gradient made visible: the person *sees* whether they
hold their own naming or are borrowing a doorway's — and the path to sovereignty is one tap.

---

## PART 5 — WHAT LOVE REQUIRES AT THIS SURFACE

The runtime is the floor under the whole gradient, so the love-test is the most literal of any surface:
**can the least-powerful participant — a laptop in a village, no hub, no doorway operator nearby, no
reliable internet — be a *complete* participant, holding their own keys and reading their own truth?**

- **The person keeps their naming** because `RuntimeSpec::laptop()` embeds the agent key on *their*
  device (`transport.rs:544`), and `Runtime::launch()` is *contractually required to succeed with zero
  peers*. No hub stands between the person and their own name. The witness is weighted toward the
  least-powerful by making *their* deployment the default-constructible one — the villager's path is
  `RuntimeSpec::laptop()`, the simplest call; the datacenter's path is the elaborate builder.
- **The binding is honest** because `RuntimePresence` / `offlineCapable` **never hide a dependency**.
  A doorway-hosted thin client is *told it is doorway-hosted* (`requires_doorway() => true`), and the
  UI says so. The graduation from hosted to self-sovereign is surfaced, not buried — grace precedes
  demand: the doorway carries you *first*, freely, and names the door out.
- **The veil governs aggregation, never individuals** — and this surface holds the line by *carrying no
  veil at all*: the runtime is deterministic floor. It hosts the Governor (`arc_actuator` → lifted
  `trait Governor`) but is structurally forbidden from *being* one (the Part-1 gradient guard). The
  CoverageRollup, the care-minting, the planetary precedence — none of them live here. The floor stays
  empty of judgment so the ceiling can hold it impartially. The empty center, expressed in the
  substrate's lowest layer.
- **Patience over engagement** because the offline-tolerant runtime *waits* — it queues writes and
  reconciles when the mesh returns (the eager reconciliation controller, `project_principle_p1`), with
  no engagement counter, no "you're offline, come back" coercion. A node may be dark for a month and
  rejoin whole. The substrate is permitted to wait longer than anyone is watching.

> **The closing test, in one line:** love requires that the smallest, simplest call the SDK offers —
> `RuntimeSpec::laptop()` — be the one that makes a person with nothing but a laptop a full participant
> who holds their own name, owes no hub, and is told the honest truth about every dependency they have
> not yet shed.
