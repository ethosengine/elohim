---
title: The Elohim Seam Map — A Concern-Routing Atlas
id: elohim-seam-map-concern-routing
date: 2026-06-21
status: reference
author: workflow:elohim-seam-map-atlas
cites:
  - platform-one-sdk-many-apis-design | THE ELOHIM PLATFORM MODEL | sha256:a15b10c68787a460 | path: genesis/docs/superpowers/specs/2026-06-14-platform-one-sdk-many-apis-design.md
  - elohim-hub-boundaries-design | elohim-hub / elohim-node / elohim-storage | sha256:d7ffa707a34d126f | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md
  - weave-epic-arc-design | The Weave Epic | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - doorway-ssr-runtime | Doorway SSR Runtime | sha256:7f75b3027ae4f9d4 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
  - tiered-quilt-stewardship-design | Tiered Quilt Stewardship | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - hardware-spec | Elohim Protocol Hardware Ecosystem: Technical Specification | sha256:230d54b7e8ad2df2 | path: genesis/docs/content/elohim-protocol/hardware-spec.md
  - dna-upgrade-governance | DNA Upgrade Governance | sha256:48b79bbffd184d89 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md
  - wave3-valueflows-hrea-interop-design | Wave 3 | sha256:c8d903ad73f0284d | path: genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md
  - elohim-sdk-gospel | CLAUDE | sha256:100bb3131875b676 | path: elohim/sdk/CLAUDE.md
---

# The Elohim Seam Map — A Concern-Routing Atlas

## Purpose

This atlas exists so that every thought process — a developer's, an agent's, a future
self's — can locate *where it must understand the problem it is solving* before it writes a
line. Elohim spans a vertical composition stack (hardware → client) and a horizontal device
spectrum (smartwatch → home storage rack), and the same surface question — *"how do I add a
capability?"*, *"why won't this node do X?"* — has different answers at different seams and
different device scales. Misrouting is the dominant failure mode: a substrate identity bug
wears an aggregation costume; a packaging fact masquerades as a dataplane fact; a UI-render
gradient gets confused with hardware tiering. The atlas's job is to make a concern
*self-locate*: what KIND of problem is this, at what device scale, and where does it live?

This atlas is the durable, positive map — it reads the same in six months. For the *current
build-state* of any seam (wired, in flight, or still to build), see the dated readiness
assessments in §6.

---

## 1. The Master Diagram

Two coordinated figures share one frame. **Figure A** is the vertical composition stack of a
*single node* with the three extension seams annotated at the layers they cut. **Figure B** is
the horizontal device spectrum with the four participation tracks drawn as bands underneath.
The seams of Figure A and the tracks of Figure B are *different axes that intersect* — a node
has a position in both at once.

### Figure A — Vertical composition stack of one node (with the 3 extension seams)

```
            ┌──────────────────────────────────────────────────────────────┐
   CLIENT   │ client surface — render + a11y + emit-intent (thin-client)    │
            │   elohim-app · elohim-elements · graphos · doorway-app ·      │
            │   steward desktop (Tauri) · sophia (renderer only)            │
            └──────────────────────────────────────────────────────────────┘
                                     ▲  HTTP / camelCase wire
            ┌──────────────────────────────────────────────────────────────┐
 APP-MAN /  │ app-manifest + domain vocabulary (declarative composition)    │◄─┐ SDK SEAM
  DOMAIN    │   domains/<app>/manifest.json · wire-types crates · codegen   │  │ add a MANIFEST
            └──────────────────────────────────────────────────────────────┘  │ compose inward
                                     ▲                                          │
            ┌──────────────────────────────────────────────────────────────┐  │
   SDK      │ SDK grammar (the composition verbs above the API boundary)    │◄─┘
  GRAMMAR   │   authorAtom · commit(face) · runGovernor · rollupCoverage ·  │
            │   bindCapability         ⟨the cohesion grammar — see §6⟩      │
            └──────────────────────────────────────────────────────────────┘
                                     ▲
            ┌──────────────────────────────────────────────────────────────┐
   MODS /   │ mods / plugins — native extension INTO the runtime            │◄── PLUGIN SEAM
  PLUGINS   │   (no loader/host today; bridges are the compile-time form)   │    add NATIVE CODE
            └──────────────────────────────────────────────────────────────┘    extend downward
                                     ▲                                       ◄── BRIDGE SEAM
            ┌──────────────────────────────────────────────────────────────┐    add a CRATE
 RUNTIME /  │ runtime / binary — single-node composition + footprint        │    translate outward
  BINARY    │   elohim-storage · doorway · steward/node(elohim-node) ·      │
            │   embedded holochain conductor · cargo features (thin↔fat)    │
            └──────────────────────────────────────────────────────────────┘
                                     ▲
            ┌──────────────────────────────────────────────────────────────┐
  OS /      │ OS / packaging / deploy target                               │
 PACKAGING  │   OCI image · .deb/AppImage (Tauri) · nginx static bundle ·  │
            │   browser JS · k8s StatefulSet · Nix devShell · wasm32 zomes │
            └──────────────────────────────────────────────────────────────┘
                                     ▲
            ┌──────────────────────────────────────────────────────────────┐
 HARDWARE   │ hardware / capability gradient  L0 ── L1 ── L2 ── L3 ── L4 ── L5
            │   fob/sensor → phone/chromebook → laptop/Pi/NUC → desktop →  │
            │   family-node-base → family-node-extended ("store-anator")   │
            └──────────────────────────────────────────────────────────────┘
```

### Figure B — Horizontal device spectrum × the 4 participation tracks

```
        smartwatch   phone     laptop    recycled  NUC/Pi    gaming-   Dwelling   home
        /fob/sensor             (Tauri)  -hub                desktop   Hub        rack
        L0–L1        L2        L2–L3     L3        L3–L4     L4        L5         L5
        ───────────────────────────────────────────────────────────────────────────────►
                                                                       (family-   (family-
                                                                        node-      node-
                                                                        base)      extended)

 T1 DHT  ████████████████████████████████████████████████████████████████████████████████  notary FLOOR
 (notary) identity · stewardship contracts · REA commitments · reach — tx5/WebRTC, ALL devices

 T3 spoke ███████████████░░░░░░░░░░ (spoke side) ──────► hosted by ──► ███████████████████  hub HOST side
 (HTTP/WS) wearable/IoT/phone bridge through a dwelling hub          recycled-laptop+ host

 T2 sub-   ░░░░░░░░░░░░░░░░░░ ██████████████████████████████████████████████████████████████  storage-running
 strate    (none)            laptop+ run elohim-storage: libp2p-primary ──► iroh-primary (thick)

 T4 door-  ◄══ browsers CONSUME (default reach: doorway.elohim.host) ══►        ███ host (opt-in)
 way       NOT a P2P participant; thick nodes may also operate a doorway as a separate deploy
```

**Reading the two figures together.** A device occupies one rung of Figure B's hardware band
*and* a slice of Figure A's stack. A smartwatch is L0, runs no runtime/binary, participates on
T1 (identity) and T3 (spoke), reaches everything else through a hub or doorway. A
family-node-extended rack is L5, runs the full elohim-storage binary with embedded conductor,
participates on T1+T2 (iroh-primary), hosts T3 spokes, and may *additionally* operate a T4
doorway. The **three extension seams** (SDK / bridge / plugin) are *where you add capability*;
the **four tracks** are *how a running thing participates*. They meet but never collapse.

---

## 2. The Device-Spectrum Table

`capabilityLevel` (L0–L5, substrate operations) is the axis below; it is orthogonal to
`stage` (1–4, onboarding sovereignty) and to product Tier (1/2/3). Archetype IDs are the
`device-*` definitions in `genesis/data/devices/devices.json`, wired per human via
`deviceArchetype` in `genesis/orchestrator/data/deployments.json`.

| Rung | Archetype (`device-*`) | L | formFactor | Binary it runs | Role(s) it can play | Tracks |
|---|---|---|---|---|---|---|
| Wearable / identity fob | `biometric-fob` | 0 | fob | none (`streamsTo` a peer) | signing oracle / identity attestation | T1, T3 (spoke) |
| Civic camera | `observer-camera` | 0 | iot-sensor | none | presence/observation attestation | T1, T3 |
| Civic mic array | `observer-mic-array` | 1 | iot-sensor | none | voice-presence attestation | T1, T3 |
| Env. sensor | `environmental-sensor` | 1 | iot-sensor | none (LoRaWAN→gw→HTTP/WS) | place-based value-flow signal | T1, T3 |
| Thin-client batch | `thin-client-batch` | 1 (3 composed) | thin-client | none indiv. | spoke; T3 *host* when composed | T1, T3 |
| Phone (the FLOOR) | `2019-android-phone` | 2 | phone | browser (hosted) | light client; the device backpressure exists for | T1, T3, (T2 rare) |
| Chromebook (edu spoke) | `chromebook-edu` | 2 | laptop | browser | hub-and-spoke spoke; multi-user | T1, T3 |
| Laptop (App Steward) | `recycled-laptop` | 3 | laptop | Tauri desktop (UI + conductor + storage sidecar) | full participant when plugged; can steward; consumer-grade hub | T1, T2 (libp2p) |
| Raspberry Pi 4 | `raspberry-pi-4` | 3 | sbc | elohim-storage (intended `elohim-node`) | always-on Tier-1 lightweight hub; full storage | T1, T2 |
| Home NUC | `home-nuc` | 4 | mini-pc | elohim-storage | mid-tier steward + small-model CPU inference | T1, T2, (T4 opt) |
| Gaming desktop (burst) | `gaming-desktop` | 4 | desktop | elohim-storage | surplus inference when not gaming; NOT always-on | T1, T2 |
| Family Node base | `family-node-base` | 5 | rack-module | elohim-storage + embedded conductor | full storage + 70B inference + doorway; custodial keys | T1, T2 (iroh), T3 host, T4 |
| Family Node extended ("store-anator") | `family-node-extended` | 5 | rack-module | elohim-storage + conductor | DwellingHub max: 20TB durable custody, multi-dwelling RS(N,K) | T1, T2 (iroh), T3 host, T4 |
| Dedicated server (institutional rack) | `dedicated-server` | 5 | server | elohim-storage + conductor | multi-family steward + HA doorway | T1, T2, T3 host, T4 |
| K8s pod | `k8s-pod-256mb` | 5 | container | elohim-storage | developer-convenience peer; the lean doorway that proved backpressure was needed | T1, T2, T4 |

Two non-monotonicities to keep straight: the k8s pod is L5 at 256MB (capability ≠ resources —
the doorway is just lean enough to fit), and the gaming desktop is L4 but `alwaysOn:false`
(**availability is a separate axis from capability**). The sharp inflections are L2→L3
(`canSteward` — below it a device holds only its own source chain), L3→L4 (`canInfer`), and
L4→L5 (`canDoorway`).

---

## 3. The Seam Catalog

One tight subsection per layer/seam. For each: **problem-class owned · "add a new X" path ·
canonical home · the adjacent-seam confusion to avoid.**

### 3.1 Hardware / capability gradient
- **Problem-class:** "what is this peer *able/allowed* to do, and how do operations adapt to
  what it reports?" The L0–L5 gradient; *deriving* operational-envelope params
  (`sync_budget_bytes`, `stewardship_capacity_gb`, `inference_model_class`) from the hardware
  spec.
- **Add a new X (archetype):** add a `device-*.md` fixture + `devices.json` entry to the
  `devices.schema.json` shape; wire `deviceArchetype` per human in `deployments.json`.
- **Home:** `genesis/plans/2026-04-13-device-archetypes-design.md` (L0-5 table `:33-41`;
  envelope `:222-230`); `genesis/docs/content/elohim-protocol/hardware-spec.md` (Stage `:13-18`,
  Tiers `:72-120`); fixtures `genesis/data/devices/*`; live declaration path
  `elohim/elohim-storage/src/services/boot_registration.rs` (capability_level copied `:114`,
  default-fallback hardcodes L2 `:49,:55`).
- **Confusion:** three gradients constantly conflated — `capability_level` (L0-5, what it can
  *do*) ≠ `stage` (1-4, where the human is on the journey) ≠ Tier (1/2/3, product class). And
  `nodeTypes` ∈ {remote, operations, edge, performance} is **k8s placement**, NOT capability —
  don't read a `nodeTypes` flip as a capability change (`feedback_k8s_is_not_the_architecture`).
- **Device-local axes (hyperscalers abstract these away; human-scale can't):** beyond compute/
  storage/network, each rung carries *sensors/peripherals* (a fob's signing oracle, a sensor
  feed, a camera), a *human-I/O modality* (watch face vs headless rack — a render-adaptation
  input, see §3.8), and *power/energy* — the last already a first-class substrate signal
  (`energy`) and what `arc_factor` / `alwaysOn` encode (availability is a separate axis from
  capability — the gaming desktop is L4 but `alwaysOn:false`).

### 3.2 OS / packaging / deploy target
- **Problem-class:** "will this code physically run on this device, and how does it get there?"
  — toolchain target (`wasm32-unknown-unknown` zomes vs native glibc/musl), packaging format
  (OCI vs `.deb`/AppImage vs nginx static vs browser JS), base image, entrypoint/process model.
- **Add a new X (deploy target):** add a Dockerfile / Tauri bundle target / nginx image; for
  edge, edit `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` (the
  repo manifest is the cleanup surface; the live cluster is the operator's).
- **Home:** root `CLAUDE.md` §Deployment Contexts (the four modes: Che / local / production /
  Tauri); `steward/device/CLAUDE.md` (Tauri shell); the Dockerfiles; `devfile.yaml` (Che dev
  substrate); `app/.../connection-strategy.provider.ts` (runtime context detection).
- **Confusion:** packaging ("which image runs") vs the network *role* it then plays. The
  `elohim-node` *container* runs the `elohim-storage` *binary* (`template.yaml:223` →
  `Dockerfile:260`) — a packaging fact, not a dataplane fact. Also: editing a manifest is a
  packaging act (here); reconciling the live cluster is operator-owned (never `kubectl` from dev).

### 3.3 Runtime / binary + footprint
- **Problem-class:** "which native binary (or none) runs here, in what compositional shape —
  thin vs fat, which cargo features, which embedded children?" The single-node *composition*
  axis (orthogonal to network role).
- **Add a new X (footprint flavor):** flip cargo `[features]` (storage `default = ["p2p",
  "graph-native"]`, `elohim/elohim-storage/Cargo.toml:267-273`); embed thin
  (`steward/node/Cargo.toml:36` = `default-features=false, features=["p2p"]`); the only shipped
  cut today is a brittle `sed` that strips V8/SSR from the storage image
  (`elohim/elohim-storage/Dockerfile:99-110`).
- **Home:** the 2026-06-21 footprint assessment
  `genesis/data/timeline/backlog/modular-runtime-plugin-integrity-feasibility.md`;
  `elohim/elohim-storage/CLAUDE.md`; `steward/node/CLAUDE.md`; `doorway/doorway-service/CLAUDE.md`.
- **Confusion:** the **`elohim-node` name overload** — the `steward/node` *binary* `elohim-node`
  (the dashboard/wizard daemon) vs the k8s *container* named `elohim-node` that runs
  `elohim-storage`. Also: footprint tuning is native-binary-only —
  never tweak WASM/DNA per device (it changes the DNA hash → partition).

### 3.4 Mod / plugin (native extension INTO the runtime)
- **Problem-class:** loading/extending native code into a running node *dynamically and
  integrity-bound* ("run this workload on that peer"). Two faces: native-plugin **tooling**
  (the loader/host/registry) and plugin **integrity** (a plugin "as-good-as-core").
- **Add a new X:** native extension is composed at the runtime/binary layer; integrity comes
  from routing a plugin's effects through the notary — the `bounds_validator` engine +
  `delegates-compute` authorization + the coordinator hot-swap pattern. See §3.6 for the
  bridge, the compile-time form of native extension.
- **Home:** `genesis/data/timeline/backlog/modular-runtime-plugin-integrity-feasibility.md`; the
  integrity rails are `elohim/elohim-storage/src/services/bounds_validator.rs` (7-check engine)
  + the `delegates-compute` Commitment (`mishpat/.../commitments.rs:540-589`) + coordinator
  hot-swap (`happ_manager.rs:418-494`).
- **Confusion:** (1) the **bridge is the compile-time form of native extension** — the plugin
  seam is "bridges, but dynamic + integrity-bound." (2) `@capability*` tags on elohim-elements
  are **UI render profiles** (lens/theme/contrast), NOT authorization; the authorization
  substrate is Mishpat `delegates-compute` commitments.

### 3.5 SDK grammar (compose inward — add a MANIFEST)
- **Problem-class:** declarative domain composition with integrity-*by-construction* and
  zero-new-DNA. The "ONE SDK, MANY APIs" model: a thin grammar learned once above the boundary,
  many capability surfaces below it. A new app = a new `domains/<app>/manifest.json`.
- **Add a new X (capability):** one of four cheap moves — a `signal_kind`, a Commitment
  `action` discriminator, a `Governor` impl, or a `CoverageRollup` predicate — **never** a new
  DNA entry type (the near-forbidden, operator-gated fifth move).
- **Home:** `elohim/sdk/CLAUDE.md` (the "could this be captured for rent?" inclusion test) +
  `elohim/sdk/domains/CLAUDE.md`; schema `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`;
  canonical manifest `elohim/sdk/domains/lamad/manifest.json`; spec
  `genesis/docs/superpowers/specs/2026-06-14-platform-one-sdk-many-apis-design.md`; skill
  `p2p-design-gate` (the mandatory per-capability gate).
- **Confusion:** the SDK boundary is **grammar-vs-capability**, NOT the *language-vs-language*
  ts-rs codegen seam (`#[derive(TS)] → export_bindings → storage-client-ts/src/generated/`) it
  rides on. Integrity here is *by construction* (it only composes already-integral generated
  types) — there is no notary to route through, unlike bridge/plugin.

### 3.6 Bridge (translate outward — add a CRATE)
- **Problem-class:** imperative interop facing the non-elohim world — Rust library crates that
  translate external protocols to/from the canonical EPR-REA substrate.
- **Add a new X (bridge):** create the crate under `bridges/` + add a Cargo dep + add a match
  arm + recompile (today: `else if sub_path=="vf-graphql"` at
  `elohim/elohim-storage/src/api/mod.rs:423-437`). Decide the host by traffic kind: web2 →
  doorway; protocol-shaped → storage.
- **Home:** `bridges/CLAUDE.md`; live bridge `bridges/valueflows/`; spec
  `genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md`;
  skill `rea-economics`.
- **Confusion:** bridge vs plugin (§3.4) — **not disjoint**; the bridge is the compile-time
  instance of native extension. Their integrity models *rhyme* (both untrusted edge code gated
  by the substrate — bridge: `qahal-authority`; plugin: hash-bound validator). The real
  discriminator is **direction + bind-time**, not "different things."

### 3.7 App-manifest / domain
- **Problem-class:** "add a new domain app" — declarative vocabulary (content types/formats,
  renderer mappings, relationships, signals, coupling rules) validated against the manifest
  schema; apps validate their own vocabulary, the schema validates the shape.
- **Add a new X (domain):** `domains/<app>/manifest.json` + `domains/<app>/types/` wire-types +
  codegen (`pnpm run lamad:codegen` / `schema:codegen:ts`). Seed data must use **lamad-manifest
  formats** (`sophia-quiz-json`, `html5-app`), not core protocol formats (`interactive`).
- **Home:** `elohim/sdk/domains/CLAUDE.md`; `elohim/sdk/domains/{lamad,qahal,shefa,…}/`; root
  CLAUDE.md §Schema & Manifest Sources of Truth.
- **Confusion:** core-vs-extensible formats (a core `interactive` has no renderer → raw-JSON
  fallback). This layer composes the SDK grammar (§3.5) — it is *not* a new bridge or plugin.

### 3.8 Client surface
- **Problem-class:** presentation + user-intent capture only — **UX, accessibility, sense-and-
  respond.** Render host-provided state; emit intent events. Anything that looks like
  state-ownership (substrate fetch, REA event creation, aggregation, submission orchestration)
  is NOT this seam — it belongs to a doorway route, a zome coordinator, or a storage projection.
- **Add a new X (UI primitive):** add a Lit element to `elohim-elements` (render + a11y + emit,
  bind nothing) → bind brand in graphos Library B → compose in the elohim-app shell. Render
  before reading source (Frontend Eyes: `pnpm look`, `pnpm graphos`).
- **Home:** `app/elohim-app/CLAUDE.md`, `app/elohim-elements/CLAUDE.md` (the thin-client
  anti-pattern table `:46-77`), `app/elohim-library/CLAUDE.md` (graphos), `sophia/CLAUDE.md`
  (renderer only); skill `looking-at-frontend`.
- **Confusion:** client vs doorway projection — a "content 404 / blob missing" looks like a UI
  bug but is a substrate replication problem (doorway is single-target, no fan-out). And the
  Capability-Profile lens gradient (a render concern) ≠ hardware-tier detection (a node
  concern, §3.1).

### 3.9 Role seam — Doorway projection (Track 4)
- **Problem-class:** make canonical substrate truth legible to browsers and the web2 world —
  HTTP, OAuth relying-party, manifest-driven routes, single-target proxy + cache, doorway-to-
  doorway federation. **Doorway is NOT a P2P participant** ("never swarms libp2p or iroh").
- **Add a new X (route):** add the match arm AND `is_service_path` (else the EPR router shadows
  it → SPA bundle, the `/auth/portal` incident shape); add an `is_service_path` unit test.
- **Home:** `doorway/doorway-service/CLAUDE.md` (+ `doorway/CLAUDE.md`); the live doorway↔hub
  split is `2026-05-02-elohim-hub-boundaries-design.md` §amended-2026-06-02 — **do NOT cite the
  retired `2026-05-08-doorway-hub-edge-design.md`** (compacted into the 05-02 doc).
- **Confusion:** hub ≠ doorway — "doorway projects outward to web2; hub projects inward to
  nearby peers." A "decentralize the doorway" concern is its own work, not the hub seam.

### 3.10 Role seam — Peer-hoster dataplane (Tracks 2 + 3)
- **Problem-class:** durable availability of records across disconnect — A edits offline →
  syncs to an always-on node → B (never concurrent) later syncs A's changes. Async store-and-
  forward CRDT sync (T2 hoster↔hoster) + spoke HTTP/WS (T3 spoke↔hoster), addressed *by peerId,
  no DNS, not a public doorway*.
- **Add a new X (sync/shard protocol):** add a libp2p request-response behaviour + codec
  (skills `libp2p-protocols`/`libp2p-transport`); the CRDT engine is Automerge (`automerge-sync`).
- **Home:** `elohim/elohim-storage/CLAUDE.md`; assessment
  `genesis/data/timeline/backlog/peer-hoster-async-sync-readiness-assessment.md`; CRDT store `sync/doc_store.rs`,
  node↔node loop `p2p/mod.rs`, spoke route `http.rs`.
- **Confusion:** peerId ≠ hostname — "reach my household node" is a peerId→household-binding
  problem (`household_id` at discovery), NOT a DNS/ingress problem. And peer-hoster (durable
  *availability* of records) ≠ aggregation (value *rollup* of aggregates), though both ride T2.

### 3.11 Role seam — Aggregation / recursive rollup (Track 2)
- **Problem-class:** recursive signal aggregation (region ← councils ← households) over the
  `CoverageRollup` engine, plus the REA value-flow turning delegated compute into accounted
  flow; witnessed across DHT boundaries.
- **Add a new X (aggregate entity/route):** invoke `p2p-design-gate` FIRST (classify the
  entity); rollup engine `recursion.rs`, consumer path `graph_views/shefa/coverage.rs:25-44`,
  cross-DHT carrier `p2p/view_federation.rs` (`MAX_PAYLOAD = 256 KiB`).
- **Home:** `elohim/elohim-storage/CLAUDE.md`; assessment
  `genesis/docs/content/elohim-protocol/history/2026-06-21-commons-compute-aggregation-readiness-assessment.md`; spec
  `2026-06-20-weave-epic-arc-design.md`; skill `rea-economics`.
- **Confusion:** flat `CoverageRollup` (single-level fold) and recursive multi-level rollup are
  different rungs of the same engine — keep them distinct. A Case-A all-zeros bug is a *Track-1
  identity-coherence* failure wearing an aggregation costume, not an aggregation bug.

### 3.12 Role seam — Hub cluster ops + enablement (the hub-INTERNAL swarm)
- **Problem-class:** the k8s-class concerns absorbed *inside* a hub — blade-to-blade mDNS
  discovery, leader election, pod consensus, replica/PVC placement — **plus** the enablement
  axis: the "hubbiness dial" that flips recycled hardware into the hub role, and identity-
  preserving tier graduation. Runs on a **second libp2p swarm distinct from Track 2**.
- **Add a new X (cluster behaviour):** `steward/node/src/{cluster,pod,p2p}/`; the hub-internal
  swarm is private to the `HouseholdHub`/`CollectiveHub` impl, mDNS-first.
- **Home:** `steward/node/CLAUDE.md`; arch `2026-05-02-elohim-hub-boundaries-design.md`
  (Hub trait, two-swarms table `:180-189`); assessment
  `genesis/data/timeline/backlog/hub-enablement-dial-readiness-2026-06-21.md`; DNA-key lineage
  `2026-06-11-dna-upgrade-governance.md`; skill `hc-dev-orchestrator`.
- **Confusion:** the hub-internal swarm (Seam 3.12, `steward/node`) ≠ Track-2 hub-to-hub
  federation (Seam 3.10/3.11, `elohim-storage`). Debugging blade consensus in
  `elohim-storage/src/p2p` means you're in the wrong crate. Also: hub-as-role ≠ Tier-3 hardware
  — a recycled laptop plugged in overnight IS a consumer-grade hub.

### 3.13 Confidentiality / encryption / secrets (the third CIA leg)
- **Problem-class:** keeping content readable only by intended readers — encryption-at-rest,
  private-replica encryption, reader-key envelopes, secret/key custody. The hyperscaler KMS +
  encryption-at-rest + secrets-manager analog. *Distinct from identity/integrity (T1) and from
  authorization (3.4):* this is **secrecy**, not who-you-are or may-you-act.
- **Add a new X (encrypted content class):** a `KeyEnvelope` reader-key wrap (X25519 reader
  set) over the blob/quilt plane.
- **Home:** `genesis/docs/content/elohim-protocol/history/2026-06-21-commons-compute-aggregation-readiness-assessment.md` (R6
  private-replica encryption); the blob plane (`BlobStore::store`);
  `2026-05-11-tiered-quilt-stewardship-design.md` (quilt). Conductor signing is the *integrity*
  side, not this.
- **Confusion:** confidentiality ≠ integrity ≠ authorization. The DHT-notary gives
  tamper-evidence and capability gives may-act, but **neither encrypts** — a notarized entry is
  world-readable. "Make this private" is the encryption plane, not a permission flag. And note:
  confidentiality is **not** one of the seven substrate signals (those are economic) — it is its
  own plane.

### 3.14 Temporal / scheduling
- **Problem-class:** *when* things happen across the fabric — durable timers, recurring ticks,
  deferred/scheduled work, validity horizons, expiry→obligation. The hyperscaler
  EventBridge-Scheduler / Step-Functions / cron analog.
- **Add a new X (scheduled behaviour):** the durable cross-fabric scheduler home; the
  aggregate-tick and the feedback model's validity horizons are its first instances.
- **Home:** `time` is a substrate signal (`elohim/sdk/CLAUDE.md` signal list); aggregate-tick
  `services/aggregator.rs::aggregate_and_emit`; validity horizons in the feedback/claims model
  (`elohim/sdk/CLAUDE.md` §Feedback); the harness cron (`/loop`, CronCreate) is the **agent**
  scheduler, distinct from the substrate one.
- **Confusion:** `time` as a substrate **signal** (economic, accounted) ≠ a durable
  **scheduler** (operational, when-to-fire). "Re-quilt every N hours" / "expire this claim" is
  this seam, not aggregation (3.11) or peer-hoster (3.10).

### 3.15 Resource governance / admission / backpressure
- **Problem-class:** how the system protects itself and allocates scarce capacity — quotas,
  rate-limits, circuit-breakers, load-shedding, admission control, the operator's
  allocation/ceiling. The hyperscaler throttling / quotas / autoscaling analog.
- **Add a new X (limit/guard):** `bounds_validator` (7-check, rate/ceiling per commitment);
  doorway 503-shed / admission (`is_service_path` / admission-exempt); self-heal circuit
  breakers (runtime-harvest); pantry backpressure; `arc_factor` (sync-budget intensity).
- **Home:** `elohim/elohim-storage/src/services/bounds_validator.rs`; doorway shed/watchdog
  `/metrics` (M1-M5); the self-heal runtime-findings ledger; `arc_policy.rs`; the elohim-operator
  allocation/ceiling (`2026-06-02-doorway-ssr-runtime.md` probes/allocation/ceiling).
- **Confusion:** the unifying owner of these guards is the *elohim-operator* (per-hub
  allocator). "The phone is getting flooded" or "the breaker is stuck open" is this seam — not
  the hardware seam (capability) nor the dataplane (records).

---

## 4. The Concern-Routing Table (the heart)

Find the row that matches your concern → it names the **seam**, the **device-scale** where it
bites, and the **home** to go read/edit. Rows span the full spectrum and every seam.

| The concern you have | Seam | Device-scale | Home (where to go) |
|---|---|---|---|
| A device is too small to run a conductor | Hardware gradient + T3 spoke (3.1, 3.10) | smartwatch/fob/sensor (L0-1) | device-archetypes-design `:33-41`; complementarity §Track 3 `:225-238` |
| A wearable must contribute a signal without a node | T3 spoke bridge (3.10) | wearable/IoT (L0-1) | `peer_map.rs:483-489` (Track3Bridge); `peer-hoster-async-sync-readiness` (B7) |
| A phone keeps OOM'ing / sync floods it | Hardware gradient + runtime footprint (3.1, 3.3) | phone (L2) | device-archetypes `:105`; backpressure; cargo `[features]` thin flavor |
| Render simpler for a child / small screen | Client surface — Capability Profile (3.8) | thin (any) | `elohim-elements/CLAUDE.md:80-98` (lens gradient — NOT hardware tiering) |
| A page 404s / a blob won't load | Client vs Doorway projection (3.8, 3.9) | thin browser | `doorway/CLAUDE.md` "No Blob Fan-Out" — substrate replication, not UI |
| Add a new domain app (e.g. a new pillar surface) | App-manifest / domain (3.7) | device-invariant | `domains/<app>/manifest.json`; `elohim/sdk/domains/CLAUDE.md` |
| Add a new capability to an existing app | SDK grammar (3.5) | device-invariant | signal_kind / Commitment action / Governor / CoverageRollup; `p2p-design-gate` |
| We need to talk to Mastodon / ActivityPub | Bridge (3.6) | thick (doorway host) | `bridges/CLAUDE.md`; doorway consumes web2 bridges (atproto, activitypub planned) |
| Interop with hREA / ValueFlows / VF-GraphQL | Bridge (3.6) | thick (storage host) | `bridges/valueflows/`; `api/mod.rs:423-437`; `rea-economics` |
| Extend the runtime with a native capability | Mod/plugin (3.4) | mid–thick | modular-runtime-plugin-integrity assessment; the native execution host + load-gate |
| "Run this workload on that peer" (compute distribution) | Mod/plugin (3.4) | mid–thick | the native execution host + `delegates-compute` authorization (3.4) |
| Tune the binary for a Raspberry Pi (shrink footprint) | Runtime / binary (3.3) | Pi/NUC (L3-4) | cargo `[features]` `Cargo.toml:267-273`; thin embed `steward/node/Cargo.toml:36`; `[profile.release]` tuning |
| Package the app for a desktop user | OS / packaging (3.2) | laptop (L2-3) | Tauri `.deb`/AppImage; `steward/device/CLAUDE.md`; context 4 (sidecar :8090) |
| Deploy a new edge node to the cluster | OS / packaging (3.2) | thick (L5 pod) | `_edgenode-consolidated.template.yaml`; `deployments.json` — repo is the surface, not kubectl |
| A laptop should host content for others | Peer-hoster dataplane (3.10) | laptop→hub (L3) | `peer-hoster-async-sync-readiness`; CRDT store + node↔node loop + spoke `/sync` client |
| A non-tech steward wants to turn a laptop into a hub | Hub enablement / dial (3.12) | recycled-laptop (L3) | `hub-enablement-dial-readiness`; the `steward/node` daemon + the hubbiness dial |
| Blades in my rack won't form a pod / leader flickers | Hub cluster ops (3.12) | DwellingHub/rack (L5) | `steward/node/src/{cluster,pod,p2p}/` — hub-INTERNAL swarm, not T2 |
| Aggregate signals to commons scale (region totals) | Aggregation / rollup (3.11) | always-on→thick | recursive rollup over `recursion.rs`; cross-DHT carrier `p2p/view_federation.rs` (R2) |
| The region's commitment totals look wrong / all-zero | T1 notary floor (under 3.11) | any | identity-coherence join (`agent_cid` namespaces); fix at the notary, not the projection |
| Durably custody 20TB across cities (survive a flood) | Hardware gradient + custody (3.1) | store-anator rack (L5) | tiered-quilt-stewardship; the `src/tier/` controller + RS(N,K) custody |
| An always-on node won't act as a doorway | Doorway projection (3.9) | NUC/rack (L4-5) | doorway-operator-ness is a *separate opt-in deploy*, not a property of a big node |
| Add a new doorway HTTP route | Doorway projection (3.9) | thick | match arm **+** `is_service_path` + unit test (`/auth/portal` shadow trap) |
| Move my identity from hosted to my own desktop | OS/packaging + client (3.2, 3.8) | laptop (L2-3) | Tauri key-bundle handoff `identity.rs:34` (carries the agent key, not the source-chain) |
| Graduate hardware (laptop→NUC→rack) keeping identity | Hub enablement (3.12) | L3→L5 | DNA-key lineage (`2026-06-11-dna-upgrade-governance.md`) |
| Make this content private / readable only by X | Confidentiality (3.13) | any | `KeyEnvelope` over the blob/quilt plane (encryption ≠ permission) |
| Schedule recurring work / expire a claim / re-quilt nightly | Temporal (3.14) | always-on | `time` signal + the durable scheduler home; aggregate-tick + validity horizons |
| A peer is being flooded / a breaker is stuck open / set a quota | Resource governance (3.15) | any | `bounds_validator` + doorway shed + self-heal; the elohim-operator allocator |

---

## 5. SDK vs Bridge vs Plugin — the disambiguator (make this unmistakable)

The most-confused region: all three answer the surface question *"how do I add a capability?"*
at three different layers, with three different artifacts, three different bind-times, three
different integrity stories. The one sentence: **you ADD a manifest (SDK), a library crate
(bridge), or native code into the runtime itself (plugin) — composing inward, translating
outward, extending downward.**

| | **SDK seam** | **Bridge seam** | **Mod/plugin seam** |
|---|---|---|---|
| You ADD | a **manifest** (declarative data) | a **library crate** consumed by a runtime | **native code** into the runtime + a load-gate |
| Direction | **inward** — compose existing primitives | **outward** — foreign protocol ↔ EPR-REA | **downward** — into the binary / footprint |
| Bind-time | declarative / data (validated, codegen'd) | **compile-time** (dep + feature + match arm + recompile) | compile-time today; runtime ASPIRATIONAL |
| Integrity | **by construction** (composes already-integral types; no notary) | **by routing through the notary** (translate INTO substrate; web-writes pull `qahal-authority`) | **by routing through the notary** (untrusted plugin; hash-bound validator gates effects) |
| "Add a new X" | edit manifest → `schema:validate` → codegen | `bridges/CLAUDE.md` 4-step (crate + dep + arm + recompile) | **NO canonical path** (loader/host absent) |
| Home | `elohim/sdk/CLAUDE.md` + `domains/CLAUDE.md` | `bridges/CLAUDE.md` | none (the absence is the finding) |
| Device manifestation | device-INVARIANT (same grammar everywhere) | clustered at the doorway-bearing thick end | parameterized along the footprint ladder (thin→thick) |

**Routing rule.** Ask in order: *What do I add?* → manifest = SDK; crate = bridge; native code
= plugin. *Which direction?* → compose inward = SDK; translate a foreign protocol outward =
bridge; tune/extend the runtime downward = plugin. *What's the integrity story?* → by
construction = SDK; by routing through the notary = bridge AND plugin (which is exactly why
those two get conflated — the tie-breaker is **direction + bind-time**). If the concern is
"run this compute on that peer," it is the **plugin seam**, not SDK or bridge. The bridge is
the compile-time form of native extension — so the plugin seam reads as "bridges, but dynamic
and integrity-bound."

---

## 6. Where the architecture is growing — the cohesion destination

This atlas is the durable, positive map: it says where each kind of work *goes* and how the
seams compose. For the **current build-state** of any seam — what is wired, in flight, or still
to build — read the dated (2026-06-21) readiness assessments (they are point-in-time and
go stale; this atlas does not):

- `genesis/data/timeline/backlog/hub-enablement-dial-readiness-2026-06-21.md` — the hub / enablement seam (3.12)
- `genesis/data/timeline/backlog/peer-hoster-async-sync-readiness-assessment.md` — the peer-hoster dataplane seam (3.10)
- `genesis/docs/content/elohim-protocol/history/2026-06-21-commons-compute-aggregation-readiness-assessment.md` — the aggregation / rollup seam (3.11); its **R6** section also carries the confidentiality / encryption seam (3.13)
- `genesis/data/timeline/backlog/modular-runtime-plugin-integrity-feasibility.md` — the runtime/footprint (3.3) + plugin (3.4) seams

A seam not named above grounds its current build-state from its **§3 Home** source files — these assessments are point-in-time and don't cover every seam.

### The cohesion destination — the five-verb grammar

The direction the whole seam-space grows toward is **one SDK over many APIs**: a thin
composition grammar, learned once, the same across every capability surface — five verbs, each
an instantiation of the one `Mishpat::Commitment` primitive (+ its faces, the Governor, the CID,
the CoverageRollup):

> **authorAtom · commit(face) · runGovernor · rollupCoverage · bindCapability**

A new app is a new manifest; a new capability is one of four cheap moves; nothing needs a new
DNA entry type. The layers it composes are real (the app-manifest schema, the domain manifests,
the wire-types crates, the ts-rs codegen); the grammar is the cohesion frame that makes the
whole seam-space legible as one SDK — *how it grows*, the destination every seam routes toward.
Source: `2026-06-14-platform-one-sdk-many-apis-design.md`.

---

## 7. The three planes — control / data / projection (an organizing lens)

A cross-cutting way to read the whole stack, and the one that makes it hyperscaler-legible:

- **Control / truth plane** — the DHT-notary: identity, commitments, attestations, reach.
  Source of truth; small, cheap, witnessed. (P1: storage is a *reconciliation controller* over
  this plane — DHT is the manifest, libp2p the controller, eager reconcile.)
- **Data plane** — actual byte/record movement: blobs, CRDT docs, shards, quilt RS(N,K). Where
  availability and durability live (peer-hoster §3.10; thick-end custody §3.1).
- **Projection plane** — read-optimized renders of truth for fast/legible access: storage SQLite
  projections (P2P side) and the doorway (web2 side, §3.9). **Never the source of truth.**

Routing tell: *what's true?* → control; *moving/holding bytes?* → data; *serving a fast or
legible view?* → projection. The recurring bug is treating a **projection as truth** — the
Case-A all-zeros join and any "SQLite is authoritative" drift are this mistake.

---

## 8. Hyperscaler-parity crosswalk — and the inversion

The yardstick is explicit: *human-scale architecture as capable as a hyperscaler.* This
crosswalk maps each cloud capability to the elohim-native seam that owns it — so a "does the
protocol have an X?" question routes straight to where X lives. (For each seam's current
build-state, see the dated assessments in §6.)

| Hyperscaler capability | Elohim-native seam |
|---|---|
| Compute (VM / container / serverless) | runtime/binary (3.3) + `delegates-compute` (3.4) + SSR/inference (thick 3.1) |
| Object / block storage | blob + quilt/pantry RS(N,K) (3.10, thick 3.1) |
| Database (SQL / NoSQL / graph) | DHT notary (control) + storage projection + graph substrate |
| CDN / edge cache | doorway cache + patron-CDN (3.9) |
| DNS | doorway DNS-over-HTTPS, names→CIDs (3.9) |
| IAM / auth | agent keys + attestation + capability / `delegates-compute` (3.4) |
| KMS / encryption-at-rest / secrets | confidentiality plane (3.13) |
| Messaging / event streaming / queues | DHT signals + signal harness + NATS (orchestrator) |
| Scheduler / Step-Functions / cron | temporal plane (3.14) |
| Throttling / quotas / autoscaling | resource governance (3.15) + elohim-operator |
| Observability / monitoring | `/metrics` + `/health` + Grafana stack + self-heal |
| Analytics / ML platform | recursive aggregation / `CoverageRollup` (3.11) + local inference |
| IaC / CI-CD | orchestrator + build-manifests + rakia + `deployments.json` |
| Marketplace / service catalog | "monorepo IS the catalog" (one-SDK-many-APIs) |
| Billing / metering | REA events + reach-earning + mutual credit |
| Multi-tenancy / regions | shem multi-tenant + cells + household locality |
| Backup / DR | quilt RS(N,K) + social-recovery quorum |
| IoT / edge devices | Track-3 spoke (3.10, thin 3.1) |

**The inversion — where human-scale doesn't match but EXCEEDS.** The whole **social /
governance / trust / recovery plane has no hyperscaler equivalent**, and that is the point.
Trust & Safety is a *structural substrate property* (the reach-earning gate + the social-recovery
quorum + graduated authority), not a policy team applied after the fact. Account recovery is a
*quorum of people who actually know you*, not a support ticket that may never answer. Billing is
*earned reach / mutual credit*, not rent extraction. Governance (qahal/mishpat, `runGovernor`)
lives *in the substrate*, not in an admin console. A hyperscaler cannot offer any of this.
**Parity is the floor; this plane is the ceiling** — and it is where the human-scale architecture
wins outright, not merely catches up.

---

*Self-location, in one breath:* what do I ADD (manifest/crate/native-code → SDK/bridge/plugin),
at what DEVICE-SCALE (L0 fob → L5 store-anator), over which TRACK (T1 identity-floor / T2
substrate / T3 spoke / T4 doorway), and which ROLE does the running thing play (doorway /
peer-hoster / aggregator / hub-cluster)? Answer those four and you are standing in the right
seam.
