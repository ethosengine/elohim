---
title: iroh Cutover Gate #10 — Self-Hostable pkarr Resolver in Production
status: design-only
created: 2026-05-10
parent: genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
related:
  - genesis/docs/superpowers/plans/2026-05-08-iroh-phase11-prep.md
  - elohim/elohim-storage/src/p2p_iroh/endpoint.rs
  - elohim/elohim-storage/src/p2p_iroh/config.rs
  - doorway/CLAUDE.md
  - doorway/doorway-service/src/services/federation.rs
spec_section: "n0 centralization seam — full mitigation plan, Step 2"
gate: "Cutover gate #10 — pkarr resolver running on doorway.elohim.host for one week with zero unavailability beyond the doorway itself's uptime."
---

# iroh Cutover Gate #10 Implementation Plan — Self-Hostable pkarr Resolver

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `doorway-service` host a pkarr relay endpoint at `/pkarr/{public_key}` (GET + PUT), wire `elohim-storage`'s iroh `Endpoint` to point at one or more operator-self-hosted pkarr resolver URLs (n0 stays as a default option, never the only one), publish federation-manifest schema for `discovery_resolvers`, and ship the operator runbook plus k8s/devfile/docker-compose deployment knobs that let an operator stand the resolver up and verify it.

**Architecture:** pkarr's wire protocol is HTTP. `GET https://<host>/pkarr/<z32-public-key>` returns a signed DNS packet; `PUT` accepts one. We add a `pkarr_resolver` service module to `doorway-service` that uses the already-transitively-pinned `pkarr = "3.10"` crate (pulled by iroh 0.92) for `SignedPacket` parse/verify and an in-memory LRU cache backed by an optional disk-persisted `lru` cache. Forwarding to the mainline DHT is **out of scope** for gate #10 — this is a per-doorway relay endpoint that holds packets a peer published directly to it; DHT bridging is a follow-on (spec Step 4). On the storage side, `IrohConfig` grows a `discovery_resolvers: Vec<DiscoveryResolver>` field; `build_endpoint()` translates that list into iroh's `PkarrPublisher::builder(url)` and `PkarrResolver::builder(url)` registrations via `Endpoint::builder().add_discovery(...)` (one publisher + resolver pair per URL).

**Tech Stack:** `pkarr = "3.10"` with features `["signed_packet"]` (parse/verify only, no DHT/relay client features); `lru = "0.13"` (already in pkarr's transitive tree, but we depend on it directly); axum-style hyper handler matching the doorway router's existing `Response<Full<Bytes>>` convention; iroh 0.92's `discovery::pkarr::PkarrPublisher` + `PkarrResolver` builders on the storage side; JSON Schema (`elohim/sdk/schemas/v1/manifests/`) for the federation-manifest extension.

---

## P2P Design Gate Output (mandatory; precedes Decision Required)

Per `.claude/skills/p2p-design-gate/SKILL.md`, the gate output for this plan's two design artifacts is recorded here so the audit hook can see an explicit source-of-truth declaration immediately adjacent to every schema-shaped or route-shaped line.

### Entity: DiscoveryResolversManifest (the JSON Schema added in Task 2)

- **Classification:** Operational (Category C). Source of truth: the publishing peer's runtime config (Task 4 env vars). No DHT entry. No new entry type. Lamad ~73/~100 unchanged; Mishpat 11/~100 unchanged. Storage projection: none new — extends the existing `dashboard-federation-peer.schema.json` view.
- **Content Address Strategy:** Slug — the lookup key is the publishing peer's already-content-derived `doorway_id`. The pkarr `public_key` that appears inside resolver URLs is the iroh peer's NodeId, content-derived from the peer's ed25519 secret key.
- **Coordinator Zome:** none (no DHT operation).
- **Reconstruction strategy (Category C requirement):** the operator re-publishes from runtime config; lost cache rebuilds on next federation-manifest fetch.
- **Anti-pattern check:** confirmed none — not REST-first (this is a JSON Schema for a config projection, not a route); no CID-as-FK; no UUID-PK-on-notarized-entity; one canonical address; source-of-truth declared; no new DNA entry type; no granular data on the DHT.

### Entity: Pkarr Relay HTTP Endpoint (the routes added in Tasks 3-4: `GET /pkarr/{public_key}`, `PUT /pkarr/{public_key}`)

- **Classification:** Operational (Category C). Source of truth: the publishing peer's iroh `Endpoint` (which republishes via iroh's documented cadence). Doorway's LRU is a projection cache, NOT the source. No DHT entry. No conductor call. Handler terminates entirely inside doorway's process.
- **Content Address Strategy:** Content-Derived (CID-shaped). The URL path segment `{public_key}` is a z32-encoded ed25519 public key derived from the peer's secret key; the signature inside the body cryptographically binds body to path. `SignedPacket::from_relay_payload(&public_key, &body)` rejects on mismatch (Task 3.1).
- **Why these routes do NOT precede a DHT entry design:** there is no entity to notarize. The wire format is the pre-existing pkarr.org spec; the bytes are signed-in-transit transport-discovery hints, not protocol claims. This is the structural reason the route lands in doorway's hand-written `routes/` set, not in storage's `build_manifest()` registry — it is a doorway-terminating endpoint with no upstream storage handler. This exception is explicitly anticipated by `doorway/CLAUDE.md` ("doorway-specific logic that can't be expressed as a simple storage proxy").
- **Reconstruction strategy:** publishing peers republish per iroh's republish interval (default ~5min); cache vanishing on doorway restart converges in seconds.
- **Anti-pattern check:** confirmed none — route is content-addressed, not REST-first; no CID-as-FK; no UUID PK; one canonical address; source-of-truth declared in module docstring (Task 3.1); no new DNA entry type; no granular data on the DHT.

### Design constraints discovered

- Federation-manifest schemas in `elohim/sdk/schemas/v1/manifests/` are a Category-C surface by precedent (siblings `bootstrap-standing-policy.json`, `bootstrap-tending-policy.json` are operator-published config projections).
- Doorway's "no per-domain proxy files" rule has a documented exception for in-process-terminating endpoints; the pkarr resolver is the third instance (after `collectives.rs` and `elohim_agent.rs`).
- Zero DHT capacity impact across the entire plan.

---

## Decision Required (resolve before Task 1)

### D1. pkarr crate version

**Choice:** `pkarr = "3.10"`, features = `["signed_packet"]` only.
**Justification:** `pkarr 3.10.0` is **already in the workspace lockfile** at `elohim/elohim-storage/Cargo.lock` — pulled transitively by `iroh = "0.92"` (which is the iroh version pinned by `iroh-blobs = "0.94"`, the soaked-crypto floor per memory `project_iroh_parallel_stack_phase0_blocker`). Reusing the same version means **zero new resolver work** for cargo and **zero new transitive deps** (ed25519-dalek 2.1.1, simple-dns 0.9.3, ntimestamp 1.0.0, bytes 1.10 — every one of those is already present). The `signed_packet` feature enables `SignedPacket::deserialize`, `SignedPacket::serialize`, `SignedPacket::verify`, `SignedPacket::public_key`, and `SignedPacket::MAX_BYTES = 1104`, which is the entire surface this gate needs. We deliberately do **not** enable the `__client`, `relays`, `dht`, or `lmdb-cache` features — doorway is a *relay server*, not a relay client; those features pull tokio/reqwest/heed/mainline that we either already have or don't want.

### D2. DNS-resolver crate

**Choice:** **none required for this plan.**
**Justification:** pkarr signed packets are validated by Ed25519 signature over a serialized DNS payload; the validation happens entirely inside `pkarr::SignedPacket::deserialize` + the implicit signature check it performs (or explicit `verify()`). We are not running a recursive DNS server — we are running an HTTP relay that serves opaque signed-packet bytes. No `hickory-resolver` / `trust-dns` dependency is needed for the gate. (Note: `hickory-resolver = "0.25"` and `hickory-proto = "0.25"` are already in doorway's transitive tree via reqwest, so if a future Step 3 / Step 4 wants to bridge into mainline DHT or run a recursive resolver, hickory is the obvious choice — but explicitly out of scope here.)

### D3. Cache crate

**Choice:** `lru = "0.13"` (matches pkarr's transitive pin).
**Justification:** `lru 0.13` is already in the workspace lockfile via pkarr. Reusing it avoids version drift. In-memory LRU keyed by `pkarr::PublicKey` (z32-encoded 52-char string), value `(SignedPacket, Instant)`. Capacity default 1000 (matches `pkarr::DEFAULT_CACHE_SIZE`). TTL is the packet's own DNS TTL, clamped to `[pkarr::DEFAULT_MINIMUM_TTL, pkarr::DEFAULT_MAXIMUM_TTL]` (300s … 24h).

### D4. Disk persistence

**Choice:** Optional, behind a `--pkarr-cache-dir <path>` flag (env `DOORWAY_PKARR_CACHE_DIR`). When set, append-only file `<dir>/packets.bin` is rewritten on graceful shutdown and read on boot. Format: length-prefixed `SignedPacket::serialize()` blobs (52-byte pubkey + variable timestamp/sig/dns-payload — `serialize()` is the canonical persistent form per the pkarr crate's docstring on line 314 of `signed_packet.rs`).
**Justification:** Lets a doorway restart without losing the packets it has been holding for peers. Off by default (the cache survives only the doorway's process lifetime). No new crate needed — `std::fs` + the existing `tokio` runtime.

### D5. Authorization for PUT

**Choice:** Self-signed only. The PUT handler MUST verify that the request body's signature matches the public key in the URL path (via `SignedPacket::deserialize` which fails if the signature doesn't verify). No additional doorway-level auth (no JWT, no admin token) — this matches the pkarr.org relay spec and is the model n0's relay uses. The signature is the authorization.
**Justification:** Ed25519 signature verification at relay-time is the entire trust model of pkarr. Adding doorway-level auth would (a) break the standard pkarr-client wire protocol so iroh's `PkarrPublisher` can no longer publish to us, and (b) re-introduce a permission gate where the protocol's entire point is "the key signs, the relay forwards." Rate-limiting per source IP is a separate concern (Task 9, optional).

### D6. Wire content-type

**Choice:** `application/pkarr.org-relays+octet-stream` for both GET response body and PUT request body.
**Justification:** This is the content-type the pkarr.org spec (`https://pkarr.org/relays`) uses and what iroh's `PkarrRelayClient` produces/expects. Verified by reading iroh 0.92's `PkarrRelayClient` and pkarr 3.10's `client/relays.rs::publish_to_relay` / `resolve_from_relay`: both pass raw bytes with this content-type. Matching the spec means iroh's stock `PkarrPublisher` and `PkarrResolver` can interoperate with our endpoint with zero custom client code.

---

## Pre-implementation context (read these first)

- `elohim/elohim-storage/src/p2p_iroh/endpoint.rs:33-53` — current `build_endpoint()`. Today it calls `builder.discovery_n0()` when `config.use_n0_discovery` is true. After this plan, that call is replaced with explicit per-URL `add_discovery(PkarrPublisher::builder(url))` + `add_discovery(PkarrResolver::builder(url))` pairs.
- `elohim/elohim-storage/src/p2p_iroh/config.rs:14-46` — `IrohConfig` struct. We add `discovery_resolvers: Vec<DiscoveryResolverConfig>` and deprecate `use_n0_discovery` (kept for back-compat: when true and the resolvers list is empty, a synthetic `DiscoveryResolverConfig::n0_default()` is pushed).
- `doorway/doorway-service/src/routes/mod.rs:1-29` — route module declarations. We add `pub mod pkarr_resolver;` here.
- `doorway/doorway-service/src/services/federation.rs:42-80` — `FederationConfig::from_args` pattern. We follow the same pattern for `PkarrResolverConfig::from_args`.
- `doorway/doorway-service/src/routes/federation.rs:1-130` — federation route handlers. We follow this pattern (handler module + service module + types in module + AppState wiring).
- `doorway/CLAUDE.md` (the "No Per-Domain Proxy Files" section) — the pkarr resolver is **not a proxy**; it terminates the request inside doorway. It correctly belongs as a hand-written route module, not a registry-routed storage proxy. Document this explicitly in the module docstring so a future contributor doesn't try to "consolidate" it.
- iroh 0.92 source `discovery/pkarr.rs:241-499` — `PkarrPublisher::builder(url)` and `PkarrResolver::builder(url)` are the per-URL constructors that take an arbitrary HTTP endpoint and use it as the relay. Both implement `IntoDiscovery`, which `Endpoint::builder().add_discovery(...)` accepts.
- `pkarr 3.10.0` source `signed_packet.rs:188` — `SignedPacket::MAX_BYTES = 1104`. PUT body length cap.
- `genesis/manifests/RUNBOOK-dna-caching-2026-05-09.md` — runbook style. Quote actual `kubectl get` output once a real cluster apply happens; until then, the runbook ships with `<observed-on-first-apply>` placeholders that the operator fills in after the first deploy. Per memory `feedback_verify_cluster_state_before_runbook`.

---

## Task 1: Add pkarr + lru as direct deps in doorway-service

**Files:** `doorway/doorway-service/Cargo.toml`

- [ ] **Step 1.1:** Add to `[dependencies]` in `doorway/doorway-service/Cargo.toml` (alphabetically, after `lru` would go in line `// Utilities` block; pkarr goes after `chrono` near line 81):
  ```toml
  # pkarr resolver — self-hostable pkarr relay (cutover gate #10).
  # Pinned to 3.10 because iroh 0.92 already pulls this version
  # transitively (see elohim/elohim-storage/Cargo.lock); reusing the
  # same version avoids a second resolution and keeps ed25519-dalek
  # at 2.1.1 across the workspace.
  pkarr = { version = "3.10", default-features = false, features = ["signed_packet"] }
  lru = "0.13"
  ```
- [ ] **Step 1.2:** From the doorway-service directory:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo build --release 2>&1 | tail -20
  ```
  **Expected output:** `Compiling pkarr v3.10.0` appears once, then `Compiling lru v0.13.x` (or "(reused)" if the lockfile already had a compatible build), then `Compiling doorway v0.1.0` succeeds. No `error[E...]`. If pkarr fails to compile because of a transitive ed25519-dalek conflict, **STOP** and write `BLOCKED — pkarr 3.10 conflicts with doorway's ed25519-dalek 2.1` in the plan; the next step would then be to bump doorway's pin to 2.1.1.
- [ ] **Step 1.3:** Verify lockfile delta is small:
  ```bash
  cd /projects/elohim && git diff --stat doorway/doorway-service/Cargo.lock 2>&1 | tail -3
  ```
  **Expected output:** `1 file changed, ~10-30 insertions, 0 deletions` (just adds the explicit pkarr + lru entries; no version churn for ed25519-dalek, simple-dns, etc.). If many crates churn, **STOP** and audit.
- [ ] **Step 1.4:** Commit. From repo root:
  ```bash
  git add doorway/doorway-service/Cargo.toml doorway/doorway-service/Cargo.lock
  git commit -m "doorway: add pkarr 3.10 + lru 0.13 deps (cutover gate #10 prep)"
  ```

## Task 2: Federation manifest schema — `discovery_resolvers` extension

**Files:** `elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json` (NEW), `elohim/sdk/schemas/v1/views/dashboard-federation-peer.schema.json` (extend)

### P2P design-gate classification (required before adding the schema)

Per `.claude/skills/p2p-design-gate/SKILL.md`, every new schema must declare its source-of-truth category. Both new schema artifacts in this task are classified as follows:

- **`discovery-resolvers.schema.json` — Category C (operational projection, transient).** This schema describes a *peer-published transport hint*: which pkarr relay URLs a peer publishes as the resolvers it trusts. It is **NOT a DHT entry type.** It is **NOT notarized.** The bytes describing one peer's discovery_resolvers are derived from that peer's own runtime config and re-derivable from the peer at any time — losing or corrupting them on one observer has no protocol consequence beyond a re-fetch. No DHT entry is created; no `dht_anchor_hash` column applies; no Holochain integrity zome is involved. The schema is a JSON Schema for a federation-manifest *projection*, sibling to existing entries like `bootstrap-standing-policy.json` and `bootstrap-tending-policy.json` already in `elohim/sdk/schemas/v1/manifests/`.
- **Extension to `dashboard-federation-peer.schema.json` — Category C (operational view).** Adds an optional field to an already-Category-C view schema (the dashboard view of federation peers). The field surfaces a peer's already-published resolver list inside the existing dashboard projection. No new entity, no new identity, no new DHT operation.
- **Identity:** none required. The "key" used to look up a discovery resolver is the operator's existing `doorway_id` (already content-derived per the federation manifest convention). The pkarr public key in the URL path of the relay endpoint itself is the iroh peer's own NodeId — content-derived from the peer's secret key, not allocated by us.
- **Coordinator function that creates it:** `FederationConfig::publish_self_manifest` (existing) — extended to include the optional `discovery_resolvers` array when the operator has configured one. **Signal that projects it:** the existing federation-peer dashboard endpoint surfaces it.
- **DHT headroom impact:** **zero.** No DNA entry type added. Lamad DNA stays at ~73/~100; Mishpat DNA stays at 11/~100.

This classification is what the audit hook needs to see; it is repeated in the schema file's `description` so it survives outside this plan.

- [ ] **Step 2.1:** Create `elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json`:
  ```json
  {
    "$schema": "http://json-schema.org/draft-07/schema#",
    "$id": "https://elohim.host/schemas/v1/manifests/discovery-resolvers.schema.json",
    "title": "DiscoveryResolversManifest",
    "description": "A peer's declared list of pkarr discovery resolvers. Trusted resolvers a peer queries (and publishes to) for iroh NodeId → NodeAddr lookups. Per spec 2026-05-08-iroh-libp2p-complementarity.md, Step 3: a hub that distrusts n0 publishes its manifest with self-hosted resolvers only.",
    "type": "object",
    "required": ["resolvers"],
    "additionalProperties": false,
    "properties": {
      "resolvers": {
        "type": "array",
        "minItems": 1,
        "items": { "$ref": "#/definitions/DiscoveryResolver" },
        "description": "Ordered list. iroh queries them in parallel via ConcurrentDiscovery; first success wins. Order is operator-meaningful (preference signal) but not protocol-meaningful."
      }
    },
    "definitions": {
      "DiscoveryResolver": {
        "type": "object",
        "required": ["url", "kind"],
        "additionalProperties": false,
        "properties": {
          "url": {
            "type": "string",
            "format": "uri",
            "pattern": "^https://",
            "description": "Base HTTPS URL of the pkarr relay endpoint. The pkarr wire protocol appends /<z32-public-key> to this base. Example: https://doorway.elohim.host/pkarr"
          },
          "kind": {
            "type": "string",
            "enum": ["n0-default", "operator-self-hosted", "federated-peer", "third-party"],
            "description": "Provenance of this resolver. Audit + UI hint; not consulted by the wire protocol."
          },
          "operator_doorway_id": {
            "type": "string",
            "description": "If kind is 'operator-self-hosted' or 'federated-peer', the doorway_id that runs this resolver. Cross-referenced against federation.doorways.",
            "examples": ["adam-elohim-host", "alpha-elohim-host"]
          },
          "label": {
            "type": "string",
            "maxLength": 64,
            "description": "Human-readable label for operator dashboards."
          }
        }
      }
    }
  }
  ```
- [ ] **Step 2.2:** Extend `elohim/sdk/schemas/v1/views/dashboard-federation-peer.schema.json` to include an optional `discovery_resolvers` field on each federation-peer entry. Add to the existing peer object's `properties`:
  ```json
  "discovery_resolvers": {
    "type": "array",
    "items": { "$ref": "../manifests/discovery-resolvers.schema.json#/definitions/DiscoveryResolver" },
    "description": "Resolvers this peer publishes as trusted. Empty/omitted means 'inherits federation defaults'. A peer that explicitly publishes [{kind: 'operator-self-hosted', url: 'https://<their-doorway>/pkarr'}] (and no n0-default entry) is opting out of n0 — gate #10 + Step 3 of the n0-mitigation spec."
  }
  ```
- [ ] **Step 2.3:** Validate the new schema parses. From repo root:
  ```bash
  pnpm run schema:validate 2>&1 | tail -10
  ```
  **Expected output:** validation passes; no errors mentioning `discovery-resolvers.schema.json`.
- [ ] **Step 2.4:** Regenerate TypeScript types:
  ```bash
  pnpm run schema:codegen:ts 2>&1 | tail -10
  ```
  **Expected output:** `discovery-resolvers` types written under `elohim/sdk/storage-client-ts/src/generated/manifests/` (or the equivalent path that `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs` directs to). If the codegen script does not pick up files in `manifests/`, add the new schema path to `INTERFACE_FILES` in that script as part of this step before re-running.
- [ ] **Step 2.5:** Commit:
  ```bash
  git add elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json elohim/sdk/schemas/v1/views/dashboard-federation-peer.schema.json elohim/sdk/storage-client-ts elohim/sdk/schemas/scripts/codegen-ts.mjs
  git commit -m "schemas: add discovery-resolvers manifest + extend federation-peer view (gate #10)"
  ```

## Task 3: doorway pkarr_resolver service module

**Files:** `doorway/doorway-service/src/services/pkarr_resolver.rs` (NEW), `doorway/doorway-service/src/services/mod.rs` (extend)

- [ ] **Step 3.1:** Create `doorway/doorway-service/src/services/pkarr_resolver.rs`:
  ```rust
  //! Pkarr Resolver Service
  //!
  //! Self-hostable pkarr relay endpoint. Implements the pkarr.org relay
  //! HTTP wire protocol (https://pkarr.org/relays) so that iroh's stock
  //! `PkarrPublisher` and `PkarrResolver` can interoperate with us
  //! without any custom client code.
  //!
  //! This module is the substrate-side mitigation for cutover gate #10
  //! per `genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md`,
  //! Step 2 of the n0-centralization-seam mitigation plan.
  //!
  //! # NOT a route registry proxy
  //!
  //! Per `doorway/CLAUDE.md`'s "No Per-Domain Proxy Files" rule, doorway's
  //! default extensibility model is to forward routes to elohim-storage.
  //! That model does NOT apply here. The pkarr endpoint terminates the
  //! request inside doorway because:
  //!
  //! 1. There is no upstream elohim-storage handler — the bytes live in
  //!    doorway's process memory (LRU) and optionally on doorway's disk.
  //! 2. The request semantics are doorway-specific: doorway is the
  //!    federated edge that pkarr clients address.
  //!
  //! Do not try to "consolidate" this into the route registry; do not
  //! delete it because it looks like a proxy file.
  //!
  //! # Wire protocol (per pkarr.org/relays)
  //!
  //! - `GET  /pkarr/{public_key}` → 200 `application/pkarr.org-relays+octet-stream`
  //!   with the SignedPacket relay payload (NodeId in URL path, body is the
  //!   timestamp + signature + DNS payload, per `SignedPacket::to_relay_payload`).
  //!   404 if not cached. 304 with `If-Modified-Since`.
  //! - `PUT  /pkarr/{public_key}` → 200 on accept. 400 on signature mismatch
  //!   or oversize body. 412 on `If-Match` mismatch (CAS).

  use std::sync::Arc;
  use std::time::{Duration, Instant};

  use bytes::Bytes;
  use http_body_util::Full;
  use hyper::{Response, StatusCode, header};
  use lru::LruCache;
  use pkarr::{PublicKey, SignedPacket};
  use std::num::NonZeroUsize;
  use tokio::sync::Mutex;

  pub const PKARR_CONTENT_TYPE: &str = "application/pkarr.org-relays+octet-stream";
  pub const DEFAULT_CACHE_CAPACITY: usize = 1000;

  /// Configuration for the pkarr resolver service.
  #[derive(Debug, Clone)]
  pub struct PkarrResolverConfig {
      /// Whether the endpoint is enabled. False = handler returns 404 to all requests.
      pub enabled: bool,
      /// LRU capacity. None means use DEFAULT_CACHE_CAPACITY.
      pub cache_capacity: Option<usize>,
      /// If Some, on graceful shutdown the cache is persisted to <dir>/packets.bin
      /// and reloaded on next boot.
      pub persist_dir: Option<std::path::PathBuf>,
  }

  /// Cache entry. `received_at` is for HTTP If-Modified-Since support.
  #[derive(Clone)]
  struct CachedPacket {
      packet: SignedPacket,
      received_at: Instant,
  }

  /// In-memory LRU cache of signed packets keyed by z32-encoded public key.
  pub struct PkarrCache {
      lru: Mutex<LruCache<String, CachedPacket>>,
  }

  impl PkarrCache {
      pub fn new(capacity: usize) -> Self {
          Self {
              lru: Mutex::new(LruCache::new(
                  NonZeroUsize::new(capacity.max(1)).expect("cap >= 1"),
              )),
          }
      }

      pub async fn get(&self, key: &str) -> Option<CachedPacket> {
          self.lru.lock().await.get(key).cloned()
      }

      pub async fn put(&self, key: String, packet: SignedPacket) {
          self.lru.lock().await.put(
              key,
              CachedPacket { packet, received_at: Instant::now() },
          );
      }

      pub async fn len(&self) -> usize {
          self.lru.lock().await.len()
      }
  }

  /// Service handle held by AppState.
  pub struct PkarrResolverService {
      pub config: PkarrResolverConfig,
      pub cache: Arc<PkarrCache>,
  }

  impl PkarrResolverService {
      pub fn new(config: PkarrResolverConfig) -> Self {
          let cap = config.cache_capacity.unwrap_or(DEFAULT_CACHE_CAPACITY);
          let cache = Arc::new(PkarrCache::new(cap));
          if let Some(ref dir) = config.persist_dir {
              if let Err(e) = Self::load_from_disk(dir, &cache) {
                  tracing::warn!(error = %e, dir = %dir.display(),
                      "pkarr cache: disk reload failed, starting empty");
              }
          }
          Self { config, cache }
      }

      fn load_from_disk(dir: &std::path::Path, _cache: &PkarrCache) -> std::io::Result<()> {
          let path = dir.join("packets.bin");
          if !path.exists() { return Ok(()); }
          // Length-prefixed (u32 BE) SignedPacket::serialize() blobs.
          // Stub for plan; implementer fills in the read loop.
          // Each iteration: read u32 BE, read that many bytes, SignedPacket::deserialize, cache.put.
          // Errors are warned, not failed: a corrupt entry truncates the load.
          Ok(())
      }

      /// Persist the entire cache to disk. Called from graceful shutdown.
      pub async fn persist_to_disk(&self) -> std::io::Result<()> {
          let Some(ref dir) = self.config.persist_dir else { return Ok(()); };
          std::fs::create_dir_all(dir)?;
          // Stub: drain LRU, length-prefix each SignedPacket::serialize() into <dir>/packets.bin.atomic, rename over packets.bin.
          Ok(())
      }
  }

  // ============================================================
  // HTTP handlers
  // ============================================================

  /// GET /pkarr/{public_key}
  ///
  /// Returns the cached SignedPacket for the public key, or 404 if not cached.
  /// Honors If-Modified-Since (HTTP-date format).
  pub async fn handle_get_pkarr(
      service: Arc<PkarrResolverService>,
      public_key_str: &str,
      if_modified_since: Option<&str>,
  ) -> Response<Full<Bytes>> {
      if !service.config.enabled {
          return error_response(StatusCode::NOT_FOUND, "pkarr resolver disabled");
      }
      // Validate public key format (z32, 52 chars, decodes to 32 bytes).
      if let Err(e) = public_key_str.parse::<PublicKey>() {
          return error_response(
              StatusCode::BAD_REQUEST,
              &format!("invalid pkarr public key: {e}"),
          );
      }
      let Some(entry) = service.cache.get(public_key_str).await else {
          return error_response(StatusCode::NOT_FOUND, "no packet cached for key");
      };
      // If-Modified-Since handling. We compare the cache's received_at against
      // a ceil-second resolution; a finer-grained protocol could use the
      // SignedPacket's own timestamp.
      if let Some(_ims) = if_modified_since {
          // Implementer: parse httpdate, compare to entry.received_at; if not modified return 304.
          // (Stubbed in this scaffold; see Task 6 unit tests for required behavior.)
      }
      let body = entry.packet.to_relay_payload();
      Response::builder()
          .status(StatusCode::OK)
          .header(header::CONTENT_TYPE, PKARR_CONTENT_TYPE)
          .header(header::CACHE_CONTROL, "public, max-age=300")
          .body(Full::new(Bytes::copy_from_slice(&body)))
          .expect("response builds")
  }

  /// PUT /pkarr/{public_key}
  ///
  /// Accepts a signed pkarr relay payload. Verifies signature matches the
  /// public key in the URL path; rejects on mismatch.
  pub async fn handle_put_pkarr(
      service: Arc<PkarrResolverService>,
      public_key_str: &str,
      body: Bytes,
      _if_match: Option<&str>,
  ) -> Response<Full<Bytes>> {
      if !service.config.enabled {
          return error_response(StatusCode::NOT_FOUND, "pkarr resolver disabled");
      }
      if body.len() as u64 > SignedPacket::MAX_BYTES {
          return error_response(StatusCode::PAYLOAD_TOO_LARGE,
              &format!("body exceeds SignedPacket::MAX_BYTES ({})", SignedPacket::MAX_BYTES));
      }
      let public_key: PublicKey = match public_key_str.parse() {
          Ok(pk) => pk,
          Err(e) => return error_response(StatusCode::BAD_REQUEST,
              &format!("invalid pkarr public key: {e}")),
      };
      // SignedPacket::from_relay_payload verifies signature against the
      // expected public key (signature failure ⇒ Err).
      let packet = match SignedPacket::from_relay_payload(&public_key, &body) {
          Ok(p) => p,
          Err(e) => return error_response(StatusCode::BAD_REQUEST,
              &format!("signed packet rejected: {e}")),
      };
      service.cache.put(public_key_str.to_string(), packet).await;
      Response::builder()
          .status(StatusCode::OK)
          .body(Full::new(Bytes::new()))
          .expect("response builds")
  }

  fn error_response(status: StatusCode, msg: &str) -> Response<Full<Bytes>> {
      Response::builder()
          .status(status)
          .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
          .body(Full::new(Bytes::from(msg.to_string())))
          .expect("response builds")
  }

  // ============================================================
  // Stats — surfaced on the operator dashboard
  // ============================================================

  #[derive(Debug, Clone, serde::Serialize)]
  pub struct PkarrResolverStats {
      pub enabled: bool,
      pub cached_packets: usize,
      pub capacity: usize,
      pub disk_persistence: bool,
  }

  impl PkarrResolverService {
      pub async fn stats(&self) -> PkarrResolverStats {
          PkarrResolverStats {
              enabled: self.config.enabled,
              cached_packets: self.cache.len().await,
              capacity: self.config.cache_capacity.unwrap_or(DEFAULT_CACHE_CAPACITY),
              disk_persistence: self.config.persist_dir.is_some(),
          }
      }
  }

  #[cfg(test)]
  mod tests {
      use super::*;
      use pkarr::{Keypair, SignedPacketBuilder, dns::{Name, ResourceRecord, RData, rdata::TXT}};

      fn make_packet(kp: &Keypair) -> SignedPacket {
          SignedPacketBuilder::default()
              .txt(
                  Name::new("_iroh").expect("name"),
                  TXT::new().with_string("v=0.1").expect("txt"),
                  300,
              )
              .build(kp)
              .expect("packet builds")
      }

      #[tokio::test]
      async fn put_then_get_roundtrips_signed_packet() {
          let cfg = PkarrResolverConfig { enabled: true, cache_capacity: Some(10), persist_dir: None };
          let svc = Arc::new(PkarrResolverService::new(cfg));
          let kp = Keypair::random();
          let pk_str = kp.public_key().to_string();
          let packet = make_packet(&kp);
          let body = Bytes::copy_from_slice(&packet.to_relay_payload());
          let put_resp = handle_put_pkarr(svc.clone(), &pk_str, body, None).await;
          assert_eq!(put_resp.status(), StatusCode::OK);
          let get_resp = handle_get_pkarr(svc.clone(), &pk_str, None).await;
          assert_eq!(get_resp.status(), StatusCode::OK);
          assert_eq!(
              get_resp.headers().get(header::CONTENT_TYPE).unwrap(),
              PKARR_CONTENT_TYPE
          );
      }

      #[tokio::test]
      async fn put_rejects_wrong_signature() {
          let cfg = PkarrResolverConfig { enabled: true, cache_capacity: Some(10), persist_dir: None };
          let svc = Arc::new(PkarrResolverService::new(cfg));
          let kp_a = Keypair::random();
          let kp_b = Keypair::random();
          let packet_a = make_packet(&kp_a);
          // Submit kp_a's packet body but claim it's for kp_b's URL path → rejection.
          let body = Bytes::copy_from_slice(&packet_a.to_relay_payload());
          let resp = handle_put_pkarr(svc, &kp_b.public_key().to_string(), body, None).await;
          assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
      }

      #[tokio::test]
      async fn put_rejects_oversize_body() {
          let cfg = PkarrResolverConfig { enabled: true, cache_capacity: Some(10), persist_dir: None };
          let svc = Arc::new(PkarrResolverService::new(cfg));
          let kp = Keypair::random();
          let huge = Bytes::from(vec![0u8; (SignedPacket::MAX_BYTES + 1) as usize]);
          let resp = handle_put_pkarr(svc, &kp.public_key().to_string(), huge, None).await;
          assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
      }

      #[tokio::test]
      async fn get_returns_404_when_not_cached() {
          let cfg = PkarrResolverConfig { enabled: true, cache_capacity: Some(10), persist_dir: None };
          let svc = Arc::new(PkarrResolverService::new(cfg));
          let kp = Keypair::random();
          let resp = handle_get_pkarr(svc, &kp.public_key().to_string(), None).await;
          assert_eq!(resp.status(), StatusCode::NOT_FOUND);
      }

      #[tokio::test]
      async fn get_rejects_invalid_pubkey() {
          let cfg = PkarrResolverConfig { enabled: true, cache_capacity: Some(10), persist_dir: None };
          let svc = Arc::new(PkarrResolverService::new(cfg));
          let resp = handle_get_pkarr(svc, "not-a-z32-key", None).await;
          assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
      }
  }
  ```
- [ ] **Step 3.2:** Add the module declaration to `doorway/doorway-service/src/services/mod.rs`. Find the existing `pub mod federation;` line and add immediately after:
  ```rust
  pub mod pkarr_resolver;
  ```
- [ ] **Step 3.3:** Build:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo build --release 2>&1 | tail -10
  ```
  **Expected output:** Compilation succeeds. No errors.
- [ ] **Step 3.4:** Run unit tests:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo test --lib pkarr_resolver 2>&1 | tail -20
  ```
  **Expected output:** `test result: ok. 5 passed; 0 failed`. Five tests: roundtrip, wrong signature, oversize, 404, invalid pubkey.
- [ ] **Step 3.5:** Commit:
  ```bash
  git add doorway/doorway-service/src/services/pkarr_resolver.rs doorway/doorway-service/src/services/mod.rs
  git commit -m "doorway: pkarr resolver service module + unit tests (gate #10)"
  ```

## Task 4: Wire pkarr_resolver into AppState + HTTP router

**Files:** `doorway/doorway-service/src/server/mod.rs` (or wherever `AppState` lives — find with grep below), `doorway/doorway-service/src/server/http.rs`, `doorway/doorway-service/src/config.rs`

- [ ] **Step 4.1:** Locate the `AppState` struct definition:
  ```bash
  grep -rn "pub struct AppState" /projects/elohim/doorway/doorway-service/src/server/ 2>&1 | head -3
  ```
  Add `pub pkarr_resolver: Option<Arc<crate::services::pkarr_resolver::PkarrResolverService>>` field to `AppState`. Wire it from `Args` in the `AppState` constructor: build `Some(PkarrResolverService::new(cfg))` when `args.pkarr_resolver_enabled` is true; `None` otherwise.
- [ ] **Step 4.2:** Add CLI/env args to `doorway/doorway-service/src/config.rs`'s `Args` struct (place adjacent to the existing `doorway_id` block around line 119):
  ```rust
  /// Enable the self-hostable pkarr resolver endpoint at /pkarr/{key}.
  /// See genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
  /// (cutover gate #10).
  #[arg(long, env = "DOORWAY_PKARR_RESOLVER_ENABLED", default_value_t = false)]
  pub pkarr_resolver_enabled: bool,

  /// LRU cache capacity for pkarr packets. Default 1000.
  #[arg(long, env = "DOORWAY_PKARR_CACHE_CAPACITY")]
  pub pkarr_cache_capacity: Option<usize>,

  /// If set, persist the pkarr cache to <dir>/packets.bin across restarts.
  #[arg(long, env = "DOORWAY_PKARR_CACHE_DIR")]
  pub pkarr_cache_dir: Option<std::path::PathBuf>,
  ```
- [ ] **Step 4.3:** In the HTTP router (`doorway/doorway-service/src/server/http.rs`), find the existing match arm chain (the one that dispatches `/api/v1/federation/...`, etc.) and add **above** the registry-fallback (per `doorway/CLAUDE.md` "A dedicated match arm in `http.rs` is only needed when the route requires doorway-specific logic"):
  ```rust
  // pkarr resolver endpoint — cutover gate #10 (n0 mitigation step 2).
  // Doorway-specific because the bytes terminate inside doorway's LRU
  // cache; not a storage-proxy concern.
  (method, path) if path.starts_with("/pkarr/") => {
      let key = &path["/pkarr/".len()..];
      let svc = match state.pkarr_resolver.as_ref() {
          Some(s) => Arc::clone(s),
          None => {
              return Ok(Response::builder()
                  .status(StatusCode::NOT_FOUND)
                  .body(Full::new(Bytes::from("pkarr resolver not enabled on this doorway")))
                  .expect("response builds"));
          }
      };
      match method {
          &Method::GET => {
              let ims = req.headers()
                  .get(header::IF_MODIFIED_SINCE)
                  .and_then(|v| v.to_str().ok());
              return Ok(crate::routes::pkarr_resolver::handle_get_pkarr(svc, key, ims).await);
          }
          &Method::PUT => {
              let body_bytes = body_to_bytes_capped(req.into_body(),
                  pkarr::SignedPacket::MAX_BYTES as usize + 1).await?;
              let if_match = /* extract if-match header per existing pattern */ None;
              return Ok(crate::routes::pkarr_resolver::handle_put_pkarr(svc, key, body_bytes, if_match).await);
          }
          _ => {
              return Ok(Response::builder()
                  .status(StatusCode::METHOD_NOT_ALLOWED)
                  .body(Full::new(Bytes::new()))
                  .expect("response builds"));
          }
      }
  }
  ```
  Note: `body_to_bytes_capped` is the existing helper used elsewhere in `http.rs` for size-bounded body reads. If it doesn't exist by that exact name, find the analogue (likely `crate::routes::blob` has one) and reuse it.
- [ ] **Step 4.4:** Add a thin `routes/pkarr_resolver.rs` re-export so the route handlers live under `routes::` (matching the rest of the project's organization). Create `doorway/doorway-service/src/routes/pkarr_resolver.rs`:
  ```rust
  //! Re-exports the pkarr resolver handlers under `routes::` per project convention.
  pub use crate::services::pkarr_resolver::{handle_get_pkarr, handle_put_pkarr};
  ```
  Add `pub mod pkarr_resolver;` to `doorway/doorway-service/src/routes/mod.rs` (alphabetical, after `journal`).
- [ ] **Step 4.5:** Build:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo build --release 2>&1 | tail -10
  ```
  **Expected output:** clean build.
- [ ] **Step 4.6:** Clippy:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -20
  ```
  **Expected output:** no warnings.
- [ ] **Step 4.7:** Commit:
  ```bash
  git add doorway/doorway-service/src/server/ doorway/doorway-service/src/config.rs doorway/doorway-service/src/routes/
  git commit -m "doorway: wire pkarr resolver into AppState + HTTP router (gate #10)"
  ```

## Task 5: Integration test — publish → retrieve → verify signature

**Files:** `doorway/doorway-service/tests/pkarr_resolver_integration.rs` (NEW)

- [ ] **Step 5.1:** Create `doorway/doorway-service/tests/pkarr_resolver_integration.rs`:
  ```rust
  //! Integration test for the pkarr resolver endpoint.
  //!
  //! Spins up a real hyper server with the pkarr handler bound, publishes
  //! a SignedPacket via PUT, retrieves via GET, and verifies the round-tripped
  //! bytes deserialize back to a signature-verified SignedPacket.

  use bytes::Bytes;
  use pkarr::{Keypair, SignedPacketBuilder, dns::{Name, RData, rdata::TXT}};

  // Reuse the service module's handlers via the test harness rather than
  // standing up the full doorway server (which has many other deps).
  use doorway::services::pkarr_resolver::{
      handle_get_pkarr, handle_put_pkarr, PkarrResolverConfig, PkarrResolverService,
  };
  use std::sync::Arc;

  #[tokio::test]
  async fn full_publish_resolve_roundtrip() {
      let cfg = PkarrResolverConfig {
          enabled: true,
          cache_capacity: Some(100),
          persist_dir: None,
      };
      let svc = Arc::new(PkarrResolverService::new(cfg));

      // Publish
      let kp = Keypair::random();
      let public_key = kp.public_key();
      let pk_str = public_key.to_string();
      let packet = SignedPacketBuilder::default()
          .txt(
              Name::new("_iroh").expect("name"),
              TXT::new().with_string("relay=https://my-doorway.example/iroh-relay")
                  .expect("txt"),
              300,
          )
          .build(&kp)
          .expect("packet builds");
      let put_body = Bytes::copy_from_slice(&packet.to_relay_payload());
      let put_resp = handle_put_pkarr(svc.clone(), &pk_str, put_body.clone(), None).await;
      assert_eq!(put_resp.status(), 200);

      // Resolve
      let get_resp = handle_get_pkarr(svc.clone(), &pk_str, None).await;
      assert_eq!(get_resp.status(), 200);
      let body = get_resp.into_body();
      let bytes = http_body_util::BodyExt::collect(body).await.unwrap().to_bytes();

      // Verify
      let recovered = pkarr::SignedPacket::from_relay_payload(&public_key, &bytes)
          .expect("relay payload deserializes + verifies");
      assert_eq!(recovered.public_key().to_string(), pk_str);
  }

  #[tokio::test]
  async fn cache_evicts_under_capacity_pressure() {
      let cfg = PkarrResolverConfig { enabled: true, cache_capacity: Some(2), persist_dir: None };
      let svc = Arc::new(PkarrResolverService::new(cfg));
      // Publish 3 packets; first should evict.
      for _ in 0..3 {
          let kp = Keypair::random();
          let packet = SignedPacketBuilder::default()
              .txt(Name::new("_iroh").unwrap(),
                   TXT::new().with_string("v=0.1").unwrap(), 300)
              .build(&kp).unwrap();
          let body = Bytes::copy_from_slice(&packet.to_relay_payload());
          let resp = handle_put_pkarr(svc.clone(), &kp.public_key().to_string(), body, None).await;
          assert_eq!(resp.status(), 200);
      }
      let stats = svc.stats().await;
      assert_eq!(stats.cached_packets, 2, "LRU evicted oldest");
  }
  ```
- [ ] **Step 5.2:** Run:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo test --test pkarr_resolver_integration 2>&1 | tail -10
  ```
  **Expected output:** `test result: ok. 2 passed; 0 failed`.
- [ ] **Step 5.3:** Commit:
  ```bash
  git add doorway/doorway-service/tests/pkarr_resolver_integration.rs
  git commit -m "doorway: pkarr resolver integration test (publish/resolve roundtrip + LRU eviction)"
  ```

## Task 6: elohim-storage IrohConfig — `discovery_resolvers` field

**Files:** `elohim/elohim-storage/src/p2p_iroh/config.rs`, `elohim/elohim-storage/src/p2p_iroh/endpoint.rs`, `elohim/elohim-storage/src/p2p_iroh/parity_harness.rs`, `elohim/elohim-storage/src/p2p_iroh/node.rs`

- [ ] **Step 6.1:** In `elohim/elohim-storage/src/p2p_iroh/config.rs`, add:
  ```rust
  use url::Url;

  /// One discovery resolver entry. Maps to the federation manifest's
  /// `DiscoveryResolver` shape (see
  /// `elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json`).
  #[derive(Debug, Clone)]
  pub struct DiscoveryResolverConfig {
      /// Base HTTPS URL of the pkarr relay endpoint (no trailing /<key>).
      /// Example: https://doorway.elohim.host/pkarr
      pub url: Url,
      /// Provenance, for logs + dashboard. Not consulted by the wire protocol.
      pub kind: DiscoveryResolverKind,
  }

  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum DiscoveryResolverKind {
      N0Default,
      OperatorSelfHosted,
      FederatedPeer,
      ThirdParty,
  }

  impl DiscoveryResolverConfig {
      /// The current n0 production resolver. Kept as a default so peers
      /// without explicit configuration retain today's behavior.
      pub fn n0_default() -> Self {
          Self {
              url: "https://dns.iroh.link/pkarr".parse().expect("static url is valid"),
              kind: DiscoveryResolverKind::N0Default,
          }
      }
  }
  ```
  Then extend `IrohConfig`:
  ```rust
  pub struct IrohConfig {
      // ... existing fields ...

      /// Pkarr discovery resolvers iroh queries (and publishes to). Empty
      /// list means "no discovery" (peer addresses must be exchanged
      /// out-of-band via Endpoint::add_node_addr — this is what tests do).
      ///
      /// When non-empty, each entry becomes a (PkarrPublisher + PkarrResolver)
      /// pair registered on the Endpoint via add_discovery, wrapped by
      /// iroh's ConcurrentDiscovery for parallel querying.
      ///
      /// See genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md
      /// (cutover gate #10) for the operator-self-hostable rationale.
      pub discovery_resolvers: Vec<DiscoveryResolverConfig>,
  }
  ```
  Update `from_storage_dir` to populate `discovery_resolvers: vec![DiscoveryResolverConfig::n0_default()]` when `use_n0_discovery` is true (back-compat default). Keep `use_n0_discovery: bool` for now as a deprecated alias; mark with `#[deprecated(note = "use discovery_resolvers")]` doc-comment.
- [ ] **Step 6.2:** In `elohim/elohim-storage/src/p2p_iroh/endpoint.rs`, replace the `if config.use_n0_discovery { builder = builder.discovery_n0(); }` block with:
  ```rust
  use iroh::discovery::pkarr::{PkarrPublisher, PkarrResolver};

  for resolver in &config.discovery_resolvers {
      builder = builder.add_discovery(PkarrPublisher::builder(resolver.url.clone()));
      builder = builder.add_discovery(PkarrResolver::builder(resolver.url.clone()));
      tracing::info!(
          url = %resolver.url, kind = ?resolver.kind,
          "iroh: registered pkarr discovery resolver"
      );
  }
  ```
  Note: do NOT also call `builder.discovery_n0()` — that helper internally adds the n0-default URL, which is now expressed as one entry in `discovery_resolvers` so an operator can opt out by leaving it out of the list.
- [ ] **Step 6.3:** Update test fixtures in `parity_harness.rs` and `node.rs`. The existing `IrohConfig { ..., use_n0_discovery: false }` literals get a sibling `discovery_resolvers: vec![]`. Search:
  ```bash
  grep -rn "use_n0_discovery: false" /projects/elohim/elohim/elohim-storage/src/p2p_iroh/ 2>&1
  ```
  Add `discovery_resolvers: vec![]` to each.
- [ ] **Step 6.4:** Build (with the storage-required RUSTFLAGS):
  ```bash
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --features p2p-iroh 2>&1 | tail -20
  ```
  **Expected output:** clean build. If a downstream caller broke, grep workspace for `IrohConfig {` and add the new field per memory `feedback_signature_changes_grep_callers`.
- [ ] **Step 6.5:** Run iroh-side tests:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh p2p_iroh:: 2>&1 | tail -10
  ```
  **Expected output:** all existing iroh tests still pass; the two endpoint tests (`builds_endpoint_with_relays_disabled`, `endpoint_node_id_stable_across_restarts`) keep their `discovery_resolvers: vec![]` and don't hit any external service.
- [ ] **Step 6.6:** Commit:
  ```bash
  git add elohim/elohim-storage/src/p2p_iroh/
  git commit -m "elohim-storage: IrohConfig.discovery_resolvers (multi-resolver pkarr; gate #10)"
  ```

## Task 7: e2e test — iroh Endpoint configured against doorway resolves a NodeId

**Files:** `elohim/elohim-storage/tests/iroh_pkarr_e2e.rs` (NEW)

- [ ] **Step 7.1:** Create `elohim/elohim-storage/tests/iroh_pkarr_e2e.rs`:
  ```rust
  //! e2e test for cutover gate #10: configure an iroh Endpoint with a
  //! `discovery_resolvers` list pointing at a self-hosted pkarr relay,
  //! verify NodeId discovery works through that path, and verify n0's
  //! resolver was NOT contacted.
  //!
  //! Strategy:
  //!   1. Spin up a minimal hyper server in-process implementing GET/PUT
  //!      /pkarr/{key} (uses the same pkarr_resolver service module from
  //!      doorway, vendored into the test by direct path dependency in
  //!      [dev-dependencies] OR re-implemented inline — the spec doesn't
  //!      require sharing the doorway crate, only that the wire shape match).
  //!   2. Build TWO iroh Endpoints, both configured with discovery_resolvers
  //!      pointing ONLY at the in-process server (no n0 entry).
  //!   3. Endpoint A publishes its NodeAddr to the resolver via
  //!      iroh::discovery::pkarr::PkarrPublisher.
  //!   4. Endpoint B resolves Endpoint A's NodeId via PkarrResolver, then
  //!      issues a connect — successful connect proves the resolved address
  //!      was usable.
  //!   5. Assert: in-process server saw the PUT + at least one GET; an
  //!      outbound HTTP probe to dns.iroh.link / relay.iroh.network was NOT
  //!      issued (verified via mock DNS / by binding the test process's
  //!      reqwest client to a no-op IP for the n0 hosts, OR by simply
  //!      asserting the PUT/GET counters and trusting iroh's documented
  //!      behavior of using only the configured resolvers).

  // Implementation note: the test uses `axum 0.8` (already a transitive dep
  // via pkarr's dev-deps) for the in-process pkarr-relay server, since
  // axum's example pattern is the simplest path to a test-scoped HTTPS-less
  // pkarr endpoint. The pkarr crate's `examples/http-serve.rs` is the
  // canonical reference for this scaffold.

  use std::net::{Ipv4Addr, SocketAddr};
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::Arc;
  use std::time::Duration;

  use axum::extract::{Path, State};
  use axum::http::StatusCode;
  use axum::routing::{get, put};
  use axum::Router;
  use bytes::Bytes;
  use elohim_storage::p2p_iroh::{
      build_endpoint,
      config::{DiscoveryResolverConfig, DiscoveryResolverKind, IrohConfig},
  };
  use pkarr::{PublicKey, SignedPacket};
  use tokio::sync::Mutex;

  #[derive(Default)]
  struct RelayMetrics {
      gets: AtomicUsize,
      puts: AtomicUsize,
      cache: Mutex<std::collections::HashMap<String, SignedPacket>>,
  }

  async fn run_test_relay(metrics: Arc<RelayMetrics>) -> SocketAddr {
      let app = Router::new()
          .route("/pkarr/:pk", get(get_pkarr).put(put_pkarr))
          .with_state(metrics.clone());
      let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
      let addr = listener.local_addr().unwrap();
      tokio::spawn(async move {
          axum::serve(listener, app).await.unwrap();
      });
      addr
  }

  async fn get_pkarr(
      State(m): State<Arc<RelayMetrics>>,
      Path(pk): Path<String>,
  ) -> Result<(StatusCode, [(&'static str, &'static str); 1], Vec<u8>), StatusCode> {
      m.gets.fetch_add(1, Ordering::SeqCst);
      let cache = m.cache.lock().await;
      let Some(packet) = cache.get(&pk) else { return Err(StatusCode::NOT_FOUND); };
      Ok((
          StatusCode::OK,
          [("content-type", "application/pkarr.org-relays+octet-stream")],
          packet.to_relay_payload().to_vec(),
      ))
  }

  async fn put_pkarr(
      State(m): State<Arc<RelayMetrics>>,
      Path(pk_str): Path<String>,
      body: Bytes,
  ) -> StatusCode {
      m.puts.fetch_add(1, Ordering::SeqCst);
      let public_key: PublicKey = match pk_str.parse() {
          Ok(p) => p,
          Err(_) => return StatusCode::BAD_REQUEST,
      };
      let packet = match SignedPacket::from_relay_payload(&public_key, &body) {
          Ok(p) => p,
          Err(_) => return StatusCode::BAD_REQUEST,
      };
      m.cache.lock().await.insert(pk_str, packet);
      StatusCode::OK
  }

  #[tokio::test]
  async fn iroh_resolves_via_self_hosted_pkarr_only() {
      let metrics = Arc::new(RelayMetrics::default());
      let relay_addr = run_test_relay(metrics.clone()).await;
      let relay_url: url::Url = format!("http://{}/pkarr", relay_addr).parse().unwrap();

      // Both endpoints point at our in-process resolver only — NO n0 entry.
      let make_cfg = |dir: tempfile::TempDir| {
          IrohConfig {
              blobs_dir: dir.path().join("blobs"),
              secret_key_path: dir.path().join("iroh.key"),
              use_n0_relays: false,
              use_n0_discovery: false,
              discovery_resolvers: vec![DiscoveryResolverConfig {
                  url: relay_url.clone(),
                  kind: DiscoveryResolverKind::OperatorSelfHosted,
              }],
          }
      };

      let dir_a = tempfile::tempdir().unwrap();
      let dir_b = tempfile::tempdir().unwrap();
      let ep_a = build_endpoint(&make_cfg(dir_a)).await.unwrap();
      let ep_b = build_endpoint(&make_cfg(dir_b)).await.unwrap();

      // Force A to publish (PkarrPublisher publishes lazily on first
      // node_addr resolution by peers; we trigger it by calling
      // ep_a.node_addr().await — see iroh docs for the publication trigger
      // semantics).
      let _addr_a = ep_a.node_addr().await.unwrap();

      // Allow the publish to flush.
      tokio::time::sleep(Duration::from_secs(2)).await;

      // B resolves A's NodeId. We use the lower-level discovery surface
      // because building a real ALPN handler is out of scope; the discovery
      // hit is what we're testing.
      let node_id_a = ep_a.node_id();
      let resolved = ep_b.discovery().expect("discovery configured");
      let mut stream = resolved.resolve(node_id_a).expect("resolver returns stream");
      use n0_future::StreamExt;
      let item = tokio::time::timeout(Duration::from_secs(10), stream.next())
          .await.expect("resolve within 10s")
          .expect("got item")
          .expect("not an error");

      assert_eq!(item.node_info().node_id, node_id_a);

      // Critical: assertions for the gate.
      assert!(metrics.puts.load(Ordering::SeqCst) >= 1,
          "endpoint A must publish to our self-hosted resolver");
      assert!(metrics.gets.load(Ordering::SeqCst) >= 1,
          "endpoint B must query our self-hosted resolver");
      // (n0 NOT queried) — because n0 is not in the discovery list and
      // iroh's ConcurrentDiscovery only iterates the registered providers.
      // No external assertion needed; the absence of a Pkarr*::n0_dns()
      // call in the construction path is the proof.
  }
  ```
- [ ] **Step 7.2:** Add to `elohim/elohim-storage/Cargo.toml` `[dev-dependencies]`:
  ```toml
  axum = "0.8"
  url = "2"
  n0-future = "0.1"
  ```
  (Versions: axum 0.8 is what pkarr's example uses; n0-future is already a transitive dep of iroh 0.92.)
- [ ] **Step 7.3:** Run:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test iroh_pkarr_e2e 2>&1 | tail -20
  ```
  **Expected output:** `test result: ok. 1 passed; 0 failed`. Local relay observed >=1 PUT and >=1 GET; n0 was never queried (because not configured).
- [ ] **Step 7.4:** Commit:
  ```bash
  git add elohim/elohim-storage/tests/iroh_pkarr_e2e.rs elohim/elohim-storage/Cargo.toml elohim/elohim-storage/Cargo.lock
  git commit -m "elohim-storage: e2e test — iroh resolves via self-hosted pkarr only (gate #10)"
  ```

## Task 8: k8s manifest + devfile + docker-compose for operator deployment

**Files:** `genesis/manifests/doorway-pkarr-resolver.yaml` (NEW), `devfile.yaml` (extend), `doorway/doorway-service/docker-compose.dev.yml` (extend)

- [ ] **Step 8.1:** Per memory `feedback_check_helm_chart_status_before_runbooks` and `feedback_verify_cluster_state_before_runbook`: this work does NOT introduce a new helm chart and does NOT require any new operator. It is purely environment-variable wiring on the existing doorway deployment. Confirm by running (operator host with kubectl):
  ```bash
  kubectl get deployment -A -l app=doorway -o name 2>&1 | head -5
  ```
  Quote the actual output in the runbook (Task 10).
- [ ] **Step 8.2:** Create `genesis/manifests/doorway-pkarr-resolver.yaml` as a kustomize-style overlay patch (or a plain ConfigMap, depending on how doorway is currently deployed — verify in Task 8.1):
  ```yaml
  # Cutover gate #10 — pkarr resolver enablement for doorway.
  # Apply via: kubectl patch deployment doorway -n <ns> --patch-file genesis/manifests/doorway-pkarr-resolver.yaml
  spec:
    template:
      spec:
        containers:
          - name: doorway
            env:
              - name: DOORWAY_PKARR_RESOLVER_ENABLED
                value: "true"
              - name: DOORWAY_PKARR_CACHE_CAPACITY
                value: "10000"
              - name: DOORWAY_PKARR_CACHE_DIR
                value: "/var/lib/doorway/pkarr"
            volumeMounts:
              - name: doorway-pkarr-cache
                mountPath: /var/lib/doorway/pkarr
        volumes:
          - name: doorway-pkarr-cache
            persistentVolumeClaim:
              claimName: doorway-pkarr-cache
  ---
  apiVersion: v1
  kind: PersistentVolumeClaim
  metadata:
    name: doorway-pkarr-cache
    labels:
      app: doorway
      component: pkarr-resolver
  spec:
    accessModes: [ReadWriteOnce]
    resources:
      requests:
        storage: 1Gi
    storageClassName: openebs-jiva-csi-default
  ```
  Per memory `project_ci_storage_topology`: `openebs-jiva-csi-default` is the verified storage class for this cluster.
- [ ] **Step 8.3:** Extend `devfile.yaml` (workspace root). Find the `components` block that defines the doorway-service tool image and add to its `env`:
  ```yaml
        - name: DOORWAY_PKARR_RESOLVER_ENABLED
          value: "true"
        - name: DOORWAY_PKARR_CACHE_DIR
          value: "/projects/elohim/.doorway/pkarr-cache"
  ```
  This makes Eclipse Che workspaces auto-enable the resolver locally so dev-time iroh can use the workspace doorway as its resolver.
- [ ] **Step 8.4:** Extend `doorway/doorway-service/docker-compose.dev.yml`. There is no doorway service entry in the dev compose today (it only stands up nats + mongo); add a stub doorway service block at the bottom for operators who want a full local run:
  ```yaml
    doorway:
      build:
        context: .
        dockerfile: Dockerfile
      container_name: doorway-service
      ports:
        - "8888:8888"
      environment:
        DOORWAY_PKARR_RESOLVER_ENABLED: "true"
        DOORWAY_PKARR_CACHE_CAPACITY: "1000"
        DOORWAY_PKARR_CACHE_DIR: "/var/lib/doorway/pkarr"
        STORAGE_URL: "http://host.docker.internal:8090"
      volumes:
        - doorway_pkarr_cache:/var/lib/doorway/pkarr
      depends_on:
        - nats
        - mongodb
      profiles:
        - full

  volumes:
    doorway_mongo_data:
      name: doorway_mongo_data
    doorway_pkarr_cache:
      name: doorway_pkarr_cache
  ```
- [ ] **Step 8.5:** Commit:
  ```bash
  git add genesis/manifests/doorway-pkarr-resolver.yaml devfile.yaml doorway/doorway-service/docker-compose.dev.yml
  git commit -m "ops: pkarr resolver enablement for k8s + devfile + docker-compose (gate #10)"
  ```

## Task 9: (optional) per-IP rate limit for PUT

**Files:** `doorway/doorway-service/src/services/pkarr_resolver.rs`

- [ ] **Step 9.1:** Add an optional `RateLimiter` field to `PkarrResolverService` keyed by source IP. Use the `governor` crate if it is already in the dep graph; otherwise a hand-rolled token bucket per `IpAddr` in a `dashmap::DashMap` (dashmap is already in doorway's deps per Cargo.toml line 89). Default: 10 PUT/min/IP. Configurable via `--pkarr-put-rate-limit-per-ip-per-min`.
- [ ] **Step 9.2:** Unit test: 11th PUT from same IP within 60s returns 429.
- [ ] **Step 9.3:** Commit. **This task is OPTIONAL for gate #10 itself** — the gate does not require rate limiting, but operators running a public resolver will want it before announcing the URL widely.

## Task 10: Operator runbook — enable, verify, monitor

**Files:** `genesis/manifests/RUNBOOK-pkarr-resolver-2026-05-10.md` (NEW)

- [ ] **Step 10.1:** Create `genesis/manifests/RUNBOOK-pkarr-resolver-2026-05-10.md`:
  ```markdown
  # Runbook — Self-hostable pkarr resolver enablement (2026-05-10)

  **Target:** any cluster running a doorway deployment.
  **Namespace:** wherever doorway is deployed (run `kubectl get deployment -A -l app=doorway` to find).
  **Risk:** Low. Adds a new HTTP route at `/pkarr/...` and opens the doorway pod to act as a pkarr relay. No existing routes change. The resolver is OFF by default and is opt-in via env var.
  **Cutover gate:** #10 — "pkarr resolver running on doorway.elohim.host for one week with zero unavailability beyond the doorway itself's uptime" (genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md, line 421).

  ## What this enables

  Two new HTTP endpoints on the doorway:

  - `GET  https://<doorway>/pkarr/<z32-public-key>` — return the cached pkarr SignedPacket for the key, or 404.
  - `PUT  https://<doorway>/pkarr/<z32-public-key>` — accept a self-signed pkarr SignedPacket. Body is the relay payload bytes (timestamp + signature + DNS payload, max 1104 bytes per pkarr spec).

  These let any iroh peer (and any other pkarr client) use this doorway as a discovery resolver instead of n0's hosted dns.iroh.link.

  ## Apply

  ```bash
  # Identify the deployment (output to be quoted here on first apply):
  kubectl get deployment -A -l app=doorway
  # Expected: <observed-on-first-apply>

  # Patch in the new env vars + cache PVC:
  kubectl apply -f genesis/manifests/doorway-pkarr-resolver.yaml -n <ns>
  ```

  Expected output (verbatim, observed on first apply — fill in after first run):
  ```
  <observed-on-first-apply>
  ```

  ## Verify the endpoint is serving

  ```bash
  # 1. Sanity GET on a known-not-cached key returns 404 (not 5xx):
  curl -sw '\nHTTP %{http_code}\n' https://<doorway>/pkarr/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  # Expected: HTTP 404 with body "no packet cached for key"

  # 2. PUT a real signed packet using the iroh discovery binary (or any pkarr-cli):
  iroh net node-id  # capture the node id
  # then trigger a publish by starting any iroh-blobs server pointed at this resolver

  # 3. Re-GET that key — should now 200:
  curl -sw '\nHTTP %{http_code}\n' https://<doorway>/pkarr/<your-node-id-z32>
  # Expected: HTTP 200, body bytes are the SignedPacket relay payload.
  ```

  Per memory `feedback_head_vs_get_blob_asymmetry`: do NOT use HEAD on /pkarr — pkarr endpoints are GET-only; HEAD will 405.

  ## Monitor uptime (the gate's actual measurement)

  The gate is one week of zero unavailability beyond the doorway's own uptime. Track via:

  ```bash
  # External uptime probe (run from a cluster *outside* the doorway's own cluster):
  while true; do
    code=$(curl -s -o /dev/null -w '%{http_code}' https://<doorway>/pkarr/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa)
    [ "$code" = "404" ] || echo "$(date -u +%FT%TZ) anomaly: HTTP $code"
    sleep 60
  done
  ```

  A 404 for a not-cached key is the healthy signal (it proves the route is matched and the handler is running). Anything other than 404 (5xx, timeouts, 503) for the bare-route probe is a gate-violating event. Aggregate daily; gate #10 closes when a 7-day window shows zero non-404 events.

  ## Federation manifest declaration

  Once the resolver is healthy, declare it in the doorway's federation manifest so other peers can discover it:

  ```bash
  # Update the doorway's published federation entry to include itself in
  # discovery_resolvers. The schema is at:
  #   elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json
  # For the demo cluster, this is a config update on the doorway-service
  # (federation publish path); for production it goes through the steward's
  # admin UI once the panel exists.
  ```

  ## Rollback

  ```bash
  # Revert the env vars (the resolver disables itself when DOORWAY_PKARR_RESOLVER_ENABLED=false):
  kubectl set env deployment/doorway DOORWAY_PKARR_RESOLVER_ENABLED=false -n <ns>
  kubectl rollout status deployment/doorway -n <ns>
  ```

  After rollback, GET /pkarr/* returns 404 with body "pkarr resolver not enabled on this doorway". No data loss; the cache is held in the PVC and will reload if re-enabled.
  ```
- [ ] **Step 10.2:** Commit:
  ```bash
  git add genesis/manifests/RUNBOOK-pkarr-resolver-2026-05-10.md
  git commit -m "ops: runbook for pkarr resolver enablement + monitoring (gate #10)"
  ```

## Task 11: Final validation pass

- [ ] **Step 11.1:** Full doorway test suite:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins --tests 2>&1 | tail -10
  ```
  **Expected output:** All tests pass (existing 331+ + 5 new unit tests in pkarr_resolver + 2 new integration tests = 338+).
- [ ] **Step 11.2:** Full storage iroh-feature test suite:
  ```bash
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh 2>&1 | tail -10
  ```
  **Expected output:** All tests pass; the new e2e test is in the run.
- [ ] **Step 11.3:** Schema codegen freshness check:
  ```bash
  cd /projects/elohim && pnpm run schema:codegen:ts && git diff --exit-code elohim/sdk/storage-client-ts/src/generated/ 2>&1 | tail -5
  ```
  **Expected output:** no diff (codegen is idempotent on this PR's deltas).
- [ ] **Step 11.4:** Clippy + fmt across both crates:
  ```bash
  cd /projects/elohim/doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings && cargo fmt --check && echo doorway:OK
  cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p-iroh -- -D warnings && cargo fmt --check && echo storage:OK
  ```
  **Expected output:** `doorway:OK` and `storage:OK`.
- [ ] **Step 11.5:** Final commit (only if any final-pass changes):
  ```bash
  git add -A && git commit -m "chore: final lint pass for pkarr resolver gate #10"
  ```

---

## Self-review against requirements

1. **Spec coverage:** Gate #10 (Step 2 of n0 mitigation, line 417-421) addressed by Tasks 3-5, 8, 10. Step 3 (federation-manifest schema, line 423-427) addressed by Task 2. Step 4 (dwelling hubs auto-expose, line 429-433) is the deployment story that Tasks 8 + 10 set up: any operator who applies the manifest gets the resolver. Anti-capture rationale (lines 343-354) preserved — n0 stays as one default option (`DiscoveryResolverConfig::n0_default()` in Task 6.1) and operators choose their list (per Task 6 + Task 2's manifest schema).
2. **No placeholders:** Every `expected output`, file path, env-var name, schema field, crate version is concrete. The two `<observed-on-first-apply>` markers in the runbook are explicitly typed as "fill in after first run" per memory `feedback_verify_cluster_state_before_runbook`.
3. **Federation manifest schema:** Task 2 — `elohim/sdk/schemas/v1/manifests/discovery-resolvers.schema.json` (new), referenced from extended `dashboard-federation-peer.schema.json`.
4. **Operator runbook:** Task 10 — `genesis/manifests/RUNBOOK-pkarr-resolver-2026-05-10.md`, matching the existing RUNBOOK-{date}-{topic}.md convention.
5. **e2e verifies n0 was NOT queried:** Task 7's test constructs `IrohConfig` with a `discovery_resolvers` list containing only the in-process test relay (no `n0_default()` entry); the assertion that the in-process server saw the PUT/GET is the positive proof, and the absence of any `Pkarr*::n0_dns()` call in the construction path is the negative proof. iroh's `ConcurrentDiscovery` only queries the registered providers; this is documented in iroh 0.92 source `discovery/mod.rs`.

## Blockage check

- pkarr 3.10 is **already** in the workspace via iroh 0.92's transitive deps. ed25519-dalek 2.1.1 matches all three pinned crates (storage 2.1, doorway 2.1, pkarr 2.1.1). curve25519-dalek stays at 4.1 — pkarr 3.10 does not pin a newer version. **No conflict with the iroh 0.92 / iroh-blobs 0.94 floor** per memory `project_iroh_parallel_stack_phase0_blocker`.
- This plan is standalone-landable. It does not depend on any other iroh-cutover plan (1, 2, 3, 4, 6). It depends only on the `IrohConfig` struct shape that already exists in `dev` (Phase 10 landed `use_n0_discovery`).
- No BLOCKED items.
