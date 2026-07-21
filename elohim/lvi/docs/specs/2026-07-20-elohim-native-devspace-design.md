# lvi — Elohim-Native Devspaces (a Peer-Runtime for P2P-Sharable Development Environments)

**Date:** 2026-07-20
**Status:** Draft — pending operator review
**Author:** Matthew Dowell + Claude Opus 4.8

## Naming

**lvi** (льви — Ukrainian for *lions*; Lviv is the city of the lion, Lev's city) is the
devspace peer-runtime for the Elohim Protocol. The name is a deliberate homage: **Eclipse Che**
— the browser-based cloud development environment this work reimagines — was built substantially
by Ukrainian engineers (Codenvy), and Lviv holds a storied place in the Ukrainian mathematical
and cybernetic tradition (the Lwów mathematical school; the university's applied-mathematics and
cybernetics lineage). Che pioneered the art of the dev environment over the web. lvi carries that
art forward, P2P-native — and a pride of lions is exactly what a mesh of peers each hosting each
other's environments is. Наснаги, Львів. 🇺🇦

lvi joins the naming lineage of its dependencies: **brit** (בְּרִית, covenant — the VCS layer),
**rakia** (רָקִיעַ, firmament — the build substrate), **eprfs** (the filesystem projection), and
now **lvi** (the lions — the peer-runtime that runs devspaces on the substrate the other three
provide).

## TL;DR

Today the Elohim Protocol is developed inside Eclipse Che on Kubernetes, pulling a
`harbor.ethosengine.com/.../rust-nix-dev` image (brit's own `devfile.yaml` is the proof). lvi
replaces that stack with a **P2P-native devspace runtime**: a development environment is not an
*image you build and push* but a **covenant you mount** — a content-addressed manifest that lives
COLD as CIDs on a peer's storage, one-click WARMS onto any device via the shipped eprfs
lazy-materializer, is supervised as a sandboxed process tree, is projected at a preview URL
through a doorway, and is torn down when idle while its manifest persists forever as a CID.

This is **k8s *powers* re-derived over p2p, not k8s ported onto a DHT.** Every orchestration
power (declarative desired-state, reconciliation, ingress, quota) re-derives from primitives that
already exist — brit, rakia, eprfs, the steward/node pod, the doorway, and `delegates-compute`
compute-commitments. The one genuinely load-bearing net-new subsystem is the **devspace actuator**
(process supervision + sandbox containment over a live mount). This spec scopes a two-slice
walking skeleton on a real household node (matthew hosting james), and explicitly defers the
dependency-substrate, cross-peer placement, and metering tracks as named north-star follow-ons.

## Problem

### The web2 devspace stack, and why it's the thing to replace

The current developer experience — the one that built the protocol to this point — is Eclipse Che
on k8s. Its power is real and worth honoring: a Dockerfile is a **modular, composable,
content-addressed recipe** that ships OS + application + all dependencies reproducibly over the
web, and an orchestrator gives it scheduling, ingress, quota, and self-heal. That reproducibility-
over-the-web is exactly what made large-scale collaborative development tractable.

But it is captured stack all the way down: GitHub owns the source registry, Nexus the artifact
registry, Harbor the image registry, and Kubernetes the control plane — each a single point of
control, rent, and eviction. The protocol's own thesis (brit, rakia) is that these three powers —
informational, economic, governance — must be *coupled at the substrate*, owned by no one. A
developer environment is where all three meet: code is knowledge, building creates value,
stewardship is governance. The tooling that runs it should know all three. Today it knows none.

### Why not just port k8s onto p2p

The dominant failure mode (per the "k8s is NOT the architecture" invariant) is re-skinning k8s:
modelling compute/hardware and calling it protocol. k8s reconciles toward *an operator's declared
truth* in etcd; a control plane with a privileged observer is precisely the capture we reject. The
elohim re-derivation reconciles toward *witnessed reality* that no single party declares
(verify-locally-then-serve), and promotes work by **earned reach**, not `kubectl apply`. lvi must
re-derive the *powers*, never port the *architecture*.

## The three reframes lvi is built on

These emerged from an adversarial design workflow (7 first-principles lenses + red/green/judge on
4 load-bearing bets) and are the spec's conceptual spine.

### 1. Mount, don't ship — kill the image as a unit

A container image is *eager*: every layer tarball ships before the first process runs, because the
runtime historically had no way to fetch a missing file mid-execution. The substrate already has
exactly that missing capability, shipped and green: **eprfs `LocalMaterializer` with `Sparse` /
`FetchMissing`.** So a lvi devspace is *projected and mounted*, never built and pushed — a
`ProjectionManifest` walked sparse, each blob hydrated on first touch (verified by content-hash
before it lands), and discarded on teardown. "Pull" dies as a concept; the closure converges onto
disk exactly as far as execution demands. The image is the degenerate `LocalOnly` + fully-
pre-fetched special case.

### 2. The EPR Derivation — steal Nix's math, throw away its plumbing

The question "what is the EPR-native devfile, if not a Dockerfile and not Nix?" resolves to:
**steal Nix's one genuinely great idea — the *derivation* (a content-addressed function from
input-CIDs to output-CIDs) — and throw away everything else it drags in (the monolithic
`/nix/store`, the stdenv kitchen-sink, the DSL).** A devspace's environment definition is an
**input-addressed `ContentNode`** whose CID is:

```
CIDv1(dag-cbor, { toolchain_input_cids, source_closure_cid, build_command, schema_version_cids })
```

Dedup is automatic and Nix-grade *without* Nix: two devspaces sharing a Rust toolchain share
input-CIDs → share blobs → the store holds one copy. A 40MB toolchain does not drag a 2GB
transitive closure, because there is no stdenv — only the CIDs the derivation actually names.

The outside-the-box move, the one **Nix, Harbor, and GitHub structurally cannot make**:
**input-address for dedup; output-diverge for trust.** Do not force build determinism. Key the
store on the input-addressed derivation CID (perfect dedup, zero determinism assumption). Then
treat multi-peer output-CID *convergence* not as a correctness gate but as a **reach-earning
attestation**: when N diverse stewards independently build derivation D and produce the same
output-CID, that output *graduates* in reach (trusted → community → commons) — rakia's "reach IS
deployment promotion" invariant applied to builds. Non-determinism stops being a bug to eliminate
and becomes the raw material of a capture-resistant trust signal no single builder authors.

> Scope note: the full EPR-Derivation dependency substrate (source-closure resolution,
> `rakia-executor`, output-convergence attestation) is **Track B — deferred** (see Decomposition).
> Slice 0/1 use a hand-authored `DevspaceSeed` with pre-fetched toolchain marker blobs. The
> Derivation is the *north star the seed shape is designed toward*, not v1 work.

### 3. Reach-as-promotion — a control plane with no privileged observer

Deployment is not `kubectl apply`; it is content *earning* wider reach through witnessed
attestation. There is no etcd "truth," only per-evaluator projections; reconcile targets witnessed
reality, not an operator's declaration. This is the elohim power with no k8s equivalent — the true
differentiator — and its convergence mechanism is explicitly v2. lvi adopts the *principle* now (a
devspace's identity CID is authority-free; who may run it is a reach-scoped, notarized commitment)
and builds the *mechanism* (threshold-attested output convergence) later.

## Architecture

### Where lvi lives

`elohim/lvi/` — an in-tree cargo workspace (the **eprfs pattern**), a *peer directory to rakia*.
It is designed self-contained so it can graduate to its own `ethosengine/lvi` repository and
submodule later (the one operator-owned git step; brit and rakia already made that journey). lvi
sits *atop* its three siblings and depends on them:

```
        ┌────────────────────────────── lvi (the lions — devspace peer-runtime) ──────────────────────────────┐
        │  lvi-seed        lvi-actuator            lvi-ingress            lvi-cli                                │
        │  (DevspaceSeed)  (materialize/run/reap)  (doorway projection)   (author / warm / reap)                │
        └───────┬──────────────┬─────────────────────────┬──────────────────────────────────────────────────┘
                │              │                          │
      brit ─────┘        eprfs ┘                  doorway ┘         rakia ── (Track B: build closures)
   (covenant,        (ProjectionManifest,      (HostRegistry,     (DAG walk,
    source-closure    LocalMaterializer,        proxy pool,        derivation
    CIDs, NodeSeed)   verify_projection)        preview URLs)      executor — deferred)

   authorization:  Mishpat::Commitment `delegates-compute` + `bounds_validator`   (steward/node)
   supervision seam: steward/node pod  monitor→analyze→decide→execute  (ActionKind::Devspace)
```

### Crate layout (following the `rakia-*` / `eprfs-*` convention)

| Crate | Responsibility | Leans on |
|---|---|---|
| `lvi-core` | Shared types, the lifecycle state machine, errors, the `reach → sandbox-profile` derivation | brit `BritCid`/`Reach`, eprfs `ProjectionManifest` |
| `lvi-seed` | The `DevspaceSeed` ContentNode (the EPR-native "devfile"): entrypoint, port declarations, health probe, persistent-set, `bounds.epr_scope` | eprfs `ProjectionManifest`, brit `NodeSeed` |
| `lvi-actuator` | **The net-new core.** `Materialize` / `HealthCheck` / `Reap` handlers; podman-rootless spawn under a derived hard-quota profile; process supervision; edit-seal on reap | eprfs `LocalMaterializer`, steward/node pod `ActionKind` |
| `lvi-ingress` | Register a warmed devspace as an ephemeral doorway host; project openvscode-server WS + declared preview ports at a URL with an owner ACL | doorway `HostRegistry`, proxy pool |
| `lvi-cli` | `lvi author` / `lvi warm` / `lvi reap` / `lvi status` — the describable-CLI felt surface | all of the above |

### The `DevspaceSeed` — the EPR-native devfile

A `DevspaceSeed` extends eprfs's `ProjectionManifest` shape with a *runtime* spec. It is the
content-addressed answer to `devfile.yaml`:

```
DevspaceSeed {
  projection:      ProjectionManifest,   // repo + toolchain marker blobs → the mount (eprfs)
  derivation_cid:  Option<BritCid>,       // Track B: the EPR Derivation this seed realizes (None in v1)
  runtime: {
    entrypoint:    Vec<String>,           // e.g. ["openvscode-server", "--host", "0.0.0.0"]
    ports:         Vec<PortDecl>,          // { name, container_port, reach } — editor + preview ports
    health:        Option<HealthProbe>,
    persistent:    Vec<PathGlob>,          // the DECLARED, verified-on-reap mutable set (default: [])
  },
  bounds: {
    epr_scope:     EprScope,               // what the workload may touch (feeds delegates-compute)
    ttl:           Duration,               // the clock that bounds the trust window
    quota:         ResourceQuota,          // memory / cpu / pids / disk — the co-resident-safety floor
  },
}
```

Its CID is its identity. The `runtime` field is the small net-new addition; everything under
`projection` is shipped eprfs machinery.

### The lifecycle state machine

```
   COLD ──warm──▶ HYDRATING ──ready──▶ WARM (running, projected) ──idle/ttl──▶ REAPING ──▶ TORN-DOWN
    ▲   (seed CID on          (sparse mount,     (openvscode +           (seal edits→      (mount discarded;
    │    peer storage)         FetchMissing)      preview URLs live)      NodeSeed CID)     seed CID persists)
    └──────────────────────────────── rematerialize (from the same seed CID) ◀──────────────────────────────┘
```

- **COLD → WARM** is `LocalMaterializer::FetchMissing` against the seed's `ProjectionManifest`
  plus a supervised process spawn. The editor is interactive *before* the toolchain finishes
  hydrating (sparse mount; blocks fault in on touch).
- **Reap** stratifies mutable state (below) and seals uncommitted edits to a brit `NodeSeed` CID.
- **Rematerialize** re-runs the materializer against the *same* seed CID on any device; the sealed
  edit NodeSeed replays on top. `verify_projection` must show zero drift.

### Isolation model (honest restatement)

lvi runs a trusted peer's *unvetted* build/run code on its own hardware. The security model is
three independent layers, and none may be conflated:

1. **The trust graph selects the population.** Reach + attestation + a `delegates-compute`
   commitment decide *whose* code you will host at all. This is not isolation; it is admission.
2. **The sandbox contains an already-chosen peer.** `lvi-core` derives a *real* `reach → podman-rootless
   flags` function — at minimum `--memory --cpus --pids-limit --network=none --read-only` with a
   writable overlay scoped to the mount. This is off-the-shelf containment, not a protocol-derived
   novelty, and the spec says so plainly.
3. **The TTL bounds the window.** Revocation is **lease-expiry, not interrupt** — a running build
   cannot be SIGKILL'd mid-flight by a DHT signal in v1. TTLs are short enough (minutes-to-hours)
   that the un-interruptible window is an accepted, operator-known cost.

The one blast-radius nobody may hand-wave: **a hard resource quota isolating the devspace from the
co-resident conductor / DHT participant.** A fork-bomb, disk-filler, or memory-hog inside a lvi
devspace on matthew's rack must be quota-killed *without* endangering his live T1/T2 participation.
This is the load-bearing safety property Slice 1 must prove.

### Authorization

Authority to run is a **`Mishpat::Commitment` with action `delegates-compute`** — bounded, scoped,
revocable, TTL'd — validated by the existing 7-check `bounds_validator`. This is *schema-real but
unwired*: the same validator runs in production today for `replicates-commons`. lvi adds a
per-instance validator by **copying that live write-path pattern** (cheap wiring, not new
machinery). "matthew stewards *this* devspace for *this* agent, bounded by *these* EPR scopes,
until *this* TTL" is a single notarized, walkable commitment — displacing the X-API-Key grant.

### Ingress — thin doorway, peer-hosted compute, WebRTC transport

**The doorway does NOT project the devspace.** An IDE plus live processes plus preview ports is far
too heavy to proxy the way SSR app HTML is projected — and proxying it would make the doorway a
thing you can't leave. For devspace traffic the doorway keeps only its **thin** network
responsibilities — **bootstrap · signal · auth** — and sheds the gateway-proxy role:

- **Auth:** the doorway onboards + authenticates the visitor (the peer-native OAuth-like login,
  custodial keys, the chaperone). It vouches for identity; it does not carry the shell.
- **Signal:** the doorway brokers a **WebRTC** connection (its existing WebRTC-signal function).
  This is how a library-computer browser behind a corporate firewall reaches the host — an
  *outbound* WebRTC connection the doorway signals — not a doorway proxy.

**The offering peer's own compute hosts the devspace** — the nodes/blades in their **household
rack** that they steward, contracted per-blade via a `delegates-compute` commitment ("whichever
blade they can contract with"). The visitor's browser connects **peer-to-peer** to that blade;
openvscode-server's HTTP/WS is **tunneled over the WebRTC data channel** via a small client shim,
with a thin relay-of-last-resort only when direct traversal fails.

So `lvi-ingress` is **not** a doorway passthrough. It is: (a) register the devspace as reachable
(discovery), (b) let the doorway auth-gate + signal the WebRTC handshake, (c) a client-side shim
tunneling openvscode over the data channel. The device-local floor (Rung 1) needs none of this —
it is `localhost`. This ingress model belongs to Rungs 3; see **The onboarding & graduation
ladder** below.

### Mutable-state stratification

State across teardown is not one problem; it is three, and the default is safety:

- **Source edits** → sealed to a brit `NodeSeed` CID on reap (authority-free checkpoint; replays on
  rematerialize).
- **Derived caches** (`target/`, `node_modules/`, sccache) → steward-affinity *lease*, re-derived
  or evicted — **never commons-custodied** (the load-bearing "no derived bytes in commons"
  discipline, corrected 3× in project history).
- **Stateful-service data** (a running DB) → **refuse-by-default** unless an explicit `persistent`
  set is declared in the seed **and verified** on teardown. The author-declared-but-unverified path
  is the silent-data-loss mode; it does not ship.

## P2P Design Gate

*Run before any route/schema. The headline result: **lvi creates ZERO new Holochain DHT entry
types** — it composes on existing EPR-atom + head-anchor + Mishpat-commitment machinery, respecting
the DNA capacity budget (Lamad ~73/~100, Mishpat 11/~100). Routes FOLLOW from this design, they do
not precede it.*

### Entity: DevspaceSeed (the environment definition / EPR-native devfile)
- **Classification**: Notarized (A) — via the **existing** EPR head-anchor mechanism; **no new entry
  type**. A published devspace needs a witnessed head so peers can discover + verify it; a private
  device-local one is a local content-addressed atom with no DHT anchor.
- **Content Address**: Content-Derived **CID** (`bafyrei…`, dag-cbor) — a structured manifest/atom
  (extends eprfs `ProjectionManifest`), so its identity IS its content hash; changing it mints a new
  version, never mutates. Not Option 2/3 (it is content, not an agent-stance or a slug).
- **Source of Truth**: content-addressed eprfs/blob plane; DHT-notarized *head* when reach ≥ trusted.
- **Coordinator / projection**: rides the existing brit/eprfs atom + EPR-head notarization (like
  `NodeSeed`/`EprMeta`) — no new zome function; projected into the eprfs manifest store.
- **Anti-pattern check**: no new entry type (DNA-budget safe); CID exposed as `bafyrei…`, never a bare
  `sha256-` or a `cid:`-holding-a-sha; linked by EntryHash, never stored as a relational FK.

### Entity: delegates-compute Commitment (authority to run / to host)
- **Classification**: Notarized (A) — the **EXISTING** `Mishpat::Commitment` `delegates-compute` entry
  type. lvi reuses it; **no new type**.
- **Content Address**: Holochain **entry_hash** (per the commitment-CID = entry_hash rule — the
  action_hash is only the `dht_anchor_hash`).
- **Source of Truth**: Holochain DHT. Validated by the copy-of-`replicates-commons` `bounds_validator`.

### Entity: Running devspace instance (lifecycle state + host-blade binding)
- **Classification**: **Operational (C)** — ephemeral runtime state (which blade hosts it, COLD/WARM
  lifecycle phase, process handle). Fully reconstructable: re-materialize from the seed CID; the
  *authority* to run is the notarized commitment.
- **Content Address**: local instance id (Slug/UUID) — justified: ephemeral, no content to hash before
  it exists.
- **Source of Truth**: local (operational). No `dht_anchor_hash`. Reconstruction = seed CID + commitment.

### Entity: Per-devspace auth session/binding
- **Classification**: **Operational (C)** — a session/JWT scoped to the owning devspace; reconstructable
  by re-auth. The *authority* is the notarized commitment (which agent may access which devspace); the
  session is a derived operational cache of that fact.
- **Content Address**: session token (Slug/UUID, operational). **Source of Truth**: operational.

### Entity: Edit-seal NodeSeed (sealed mutable state on reap)
- **Classification**: Notarized (A) via the **existing** brit `NodeSeed` mechanism (authority-free
  checkpoint); **no new type**. **Content Address**: Content-Derived **CID** (`bafyrei…`).

### Design constraints discovered
- **Zero new entry types** — the entire model composes on existing EPR-atom + head-anchor +
  Mishpat-commitment machinery. This is the gate's most important outcome.
- **Routes follow the design**: `/devspace/{id}` and `/lvi/*` are the thinnest layer over the
  **operational** running-instance projection; the **notarized commitment** is the access authority;
  the **content-addressed seed** is the environment truth. No route is a source of truth.
- **Identity coherence**: the running-instance's host-blade reference is a *transport/node* identity —
  resolve through the canonical `AgentPeerBinding`/`peer_transport_manifest` resolver; never
  string-compare `agent_cid` (`uhCAk…`) against a node/blade transport id.
- **Identity ontology**: the device → household-node ladder is a **convenience/ubiquity + reach**
  gradient, **not** a sovereignty ascent. Device-local (Rung 1) is *already* full participation (the
  hub-optional floor); the household node adds reach + always-on convenience, never a "more sovereign"
  tier. Do not frame it as one.

## The onboarding & graduation ladder (lvi as an instance of the protocol-wide pattern)

lvi is **not a bespoke, one-topology feature** ("matthew stewards a devspace for james"). It is one
**instance of the Elohim Protocol's onboarding & graduation ladder** — the adoption pattern that
*every* epr-app rides. This spec is the first document to name that ladder as doctrine, and lvi is
its Eclipse-Che-shaped reference instance. Verified against the live doorway/steward substrate
(2026-07-20).

**The ladder (with the carrying primitive + honest status at each rung):**

- **Rung 1 — conductor-on-device (the floor). SHIPPED.** The app launches co-resident with a
  conductor on your own device — a complete DHT peer, standalone, zero doorway dependency
  (`steward/device`: `tauri_plugin_holochain::async_init` embeds the conductor in-process;
  `spawn_storage_sidecar` co-launches storage, event-gated). lvi's IDE co-launches as a *third*
  managed sibling — `spawn_devspace_actuator()`, ~150 lines mirroring the sidecar. Accessed at
  `localhost`; **no doorway, no WebRTC, no remote auth.** This is the hub-optional floor.
- **The flywheel — conductor-less zero-install on-ramp. SHIPPED.** A conductor-holder invites a
  conductor-less person *through a doorway* to try it out; they get a real agent with no install
  (custodial keys + the chaperone `POST /hc/connect` + `AgentProvisioner` onto a pooled conductor).
  They taste it, then graduate to their own conductor — "users leave for their own devices." The
  growth loop. lvi reuses this wholesale; it adds only a pooled *try-it* `DevspaceSeed` + containment.
- **Graduation (custodial → steward). SHIPPED (bookkeeping) / partial (data migration).**
  `handle_export_key` + `handle_confirm_stewardship` (Ed25519 possession proof → `is_steward=true`
  → deprovision custodial cell), device-side IPC closing the loop. The unbuilt gap — DHT
  source-chain migration of the graduate's data — is **protocol-wide, not lvi's**.
- **Rung 3 — household-rack backend through a doorway, web-ubiquitous, behind peer-native login.
  Plumbing SHIPPED; the generalized self-service loop UNBUILT.** The web convenience: reach *your*
  devspace from any browser (a library computer through a corporate firewall). Per the ingress
  model above, this is **thin doorway (auth + WebRTC signal) + household-rack-hosted compute
  (contracted per-blade) + browser↔blade WebRTC transport** — *not* a doorway proxy. What is
  shipped: the DHT-notarized `operate-doorway` operator authority embedded in the JWT (this is what
  makes "peer-native OAuth-like login" more than a login table — the token names which node holds
  your agent), `PortalHost` discovery, the OAuth `authorize`/`token` surface. What is unbuilt
  everywhere (only bespoke `deployments.json` proves it today): the *generalized self-service*
  foreign-browser→your-household-node handoff. lvi proves the household-rack case; the
  generalization is **doorway shared substrate lvi is the first consumer of — build once, push back
  to the doorway**, never reinvent per-app.

**The identity spine is consumed WHOLESALE.** lvi registers one `client_id` and inherits the OAuth
`authorize`/`token` flow, `identityGuard`/`sessionOrAuthGuard`, `DoorwaySessionClient`, custodial
chaperone, and the graduation path — the *same running code* elohim-app uses. lvi adds **zero
general auth machinery.**

**The honest net-new delta lvi builds (and nothing more):**

1. `spawn_devspace_actuator()` — ~150 lines, the Rung-1 co-launch (mirrors `spawn_storage_sidecar`).
2. The `DevspaceSeed` + actuator/containment **lifecycle** (the devspace-specific core — see
   Architecture).
3. **A devspace OAuth client with PKCE + real consent — NOT `trusted:true`.** This is the one place
   lvi must *not* inherit elohim-app's lower-stakes posture: the client fronts a **live shell with
   filesystem + secret access**. Plus **per-devspace auth binding** — a token scoped to the *owning*
   devspace, so an authenticated user cannot reach *another* user's shell (`isAuthenticated()` alone
   will not stop that).
4. The **WebRTC transport shim** for Rung 3 (browser↔rack-blade, doorway-signaled; openvscode
   tunneled over the data channel).

**Explicitly NOT lvi's** (protocol-wide substrate, pushed to doorway): the generalized Rung 3
self-service session handoff, and source-chain graduation-migration.

## Design constraints from the adversarial verdicts

The four load-bearing bets were adjudicated; their verdicts are binding design constraints:

- **Dependency substrate → RESHAPE.** v1 does not "replace Harbor." It ships the source/derivation
  descriptor shape and cache-accelerates the artifact layer opportunistically. Output-convergence
  is a v2 attestation, not a v1 trust claim (nothing converges until `rakia-executor` exists).
- **Isolation → VIABLE-WITH-CONDITIONS.** The three-layer model above, with the co-resident quota
  as the non-negotiable floor.
- **Orchestration sufficiency → VIABLE-WITH-CONDITIONS.** True *only* with "schedule/place"
  struck: one pre-named steward, one pre-named agent, no bidding, no leader election. The reconcile
  loop genuinely composes; the actuator, the `DevspaceSeed` runtime shape, and generic ingress are
  net-new.
- **Ephemeral, network-backed → RESHAPE (split).** The content-custody + mutable-state spine is
  viable on shipped code. Economics is **"authorization-bounded, clock-revocable,"** *not*
  "economically coherent" — a grant is not a meter, a TTL is not an interrupt. Realized-compute
  metering (REA R4) is deferred.

## Decomposition & buildable slices

### Slice 0 — the eprfs proof (zero stubs; provable on shipped code today)

Prove **COLD → WARM → teardown → rematerialize + mutable-state** without touching a single stub
subsystem (no pod supervision, no `delegates-compute` executor, no doorway projection):

1. Author a `DevspaceSeed`-shaped `ProjectionManifest` (a repo + a handful of toolchain marker
   blobs), custody-committed on matthew's rack.
2. On device B: `Sparse` → `FetchMissing` → edit a file → reap-seal the edit to a `NodeSeed` →
   discard the mount.
3. On device C: rematerialize from the **same manifest CID**; prove the edit survived via the
   sealed seed with `verify_projection` showing **zero drift**.

This is the single most-available win and de-risks the Bet-4 spine on code that already exists.

### Slice 1 — the actuator + adversarial containment (the net-new core, on a real household node)

On a real alpha-household node (matthew hosting james):

1. `lvi-actuator` implements pod `ActionKind::Devspace{Materialize, HealthCheck, Reap}` with a
   *real* podman-rootless handler carrying the derived hard-quota profile.
2. It runs `LocalMaterializer::FetchMissing` against a minimal `DevspaceSeed` and spawns
   openvscode-server, authorized by one hand-authored `delegates-compute` Commitment through the
   copy-of-`replicates-commons` validator.
3. `lvi-ingress` projects the editor at a preview URL through the doorway on a static port.
4. TTL-reap seals uncommitted edits (Slice 0's mechanism).

**Slice 1 must prove three things:**
- **One-click warm in seconds** — sparse mount; editor interactive before the toolchain finishes
  hydrating.
- **Co-resident safety** — the conductor / DHT participant stays healthy while an adversarial build
  (fork-bomb / disk-filler / memory-hog) is quota-killed.
- **Mutable state survives teardown** — reap seals to a CID that rematerializes with zero
  `verify_projection` drift.

### Explicitly deferred (named north-star, not v1)

- **Track B — dependency substrate:** brit `BritCid` → CIDv1 migration, `DevspaceSeed` source-
  closure resolution (build resolves source entirely from the p2p blob plane, zero web2 hits),
  `rakia-executor` + peer-dispatch, output-convergence attestation. *The GitHub/Nexus/Harbor
  replacement.*
- **Cross-peer placement** as a *market* (offer → bid → pick over gossipsub; **never** a scheduler
  with leader election — that is the capture smell).
- **Reach-as-promotion / threshold-attested convergence** (the reproducibility-as-reach mechanism).
- **Realized-compute metering (REA R4)** and interrupt-grade revocation.
- **The untrusted-tenant isolation tier** (firecracker/microVM) and dynamic multi-tenant TTL ingress.

## Confirmed operator decisions

- **Q1 — artifact-cache custody:** *Accept the v1 loss.* Ship the source/derivation plane; caches
  are rebuild-or-lease-from-steward, never commons blobs. Keeps the no-derived-bytes discipline;
  makes Track B a small extension rather than a blocker on `rakia-executor`.
- **Q2 — hosting trust posture:** *Accept a bounded-TTL run of a trusted peer's unvetted code inside
  the household trust boundary* — podman + hard quota + TTL, no artifact review, revocation =
  lease-expiry. The trust graph excluded untrusted tenants; the quota protects the substrate.
- **Q3 — placement model:** *Ship placement-fixed v1; build placement later as a market, never a
  master-electing scheduler.* Degrades gracefully to the hub-optional floor ("the one peer that
  answered").

## Risks & open design questions

- **Cold-start latency on multi-GB working sets / the warming device's disk ceiling** — a
  *risk-to-measure*, not a settled property. The 200GB workspace never exists as one blob (sparse
  mount hydrates on touch), but first-launch latency and the receiving device's disk headroom are
  unproven and must be instrumented in Slice 0/1.
- **The exact toolchain-marker-blob encoding** (how a seed names "obtain the Rust toolchain"
  without Nix and without a Dockerfile) is the first real design task inside `lvi-seed` — the EPR
  Derivation names the *shape*, but the concrete v1 encoding of pre-fetched toolchain blobs is to be
  specified in the implementation plan.
- **Submodule graduation** — lvi starts in-tree (eprfs pattern); extraction to `ethosengine/lvi` is
  a later operator-owned step.

## Success criteria

lvi v1 is done (operator scope decision, 2026-07-21) when a developer or elohim agent, **from a
library computer on a foreign network behind a corporate firewall, authenticates via the peer-native
OAuth-like login, reaches a devspace hosted on a blade in the offering steward's household rack over
doorway-signaled WebRTC, edits code in openvscode-server within seconds, and the environment tears
down and rematerializes from the same CID with edits intact — while an adversarial build inside it
cannot harm the host's live protocol participation.** That is the web-convenience headline — the thing
a self-hosting homelab person can do today, made available without being one — proven honestly on the
substrate the protocol already runs.

## Testing approach

- **Slice 0:** an integration test exercising author → sparse-warm → edit → reap-seal → discard →
  rematerialize → `verify_projection` (zero drift) across two local eprfs stores. Pure shipped-code;
  runs in CI.
- **Slice 1:** an a2o scenario (`genesis/a2o/features/`, `@concern` tagged) for one-click warm +
  preview-URL reachability (`pnpm look` against the projected doorway URL), plus a **containment
  scenario**: an adversarial workload (fork-bomb / disk-filler / memory-hog) is quota-killed while a
  co-resident conductor health probe stays green. The containment scenario is the load-bearing
  regression guard and must run on a real household node, not a stub.
