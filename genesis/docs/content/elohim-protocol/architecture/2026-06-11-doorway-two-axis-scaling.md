---
title: Doorway Two-Axis Scaling
id: doorway-two-axis-scaling
tier: architecture
status: accepted — As-implemented distillation (truth:DERIVED — refines the resilience canon; the graduation/recovery story originates there, not here)
created: 2026-06-11
informed-by:
  - genesis/docs/content/elohim-protocol/resilience/README.md (Parts V/VI — the canon this seed composes with; stewardship surface + patron-CDN)
  - genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md (visitor/peer graduation patterns; anti-binary participation)
  - genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md (graduation vocabulary; substrate landed, feature held)
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md (the three access tiers this scaling model serves)
informs:
  - All doorway capacity/deployment planning (replica counts, conductor pool sizing)
  - Future axis separation work (projection-replica vs identity-host split)
  - Graduation-pipeline specs (source-chain migration, recovery registration)
derived_from:
  - doorway/doorway-service/SCALING.md  # retired to git 2026-06-11 (doorway island recompose; authored 2026-04-30)
cites:
  - "resilience-protocol-spec | the canon this seed refines (truth:DERIVED) — Part V stewardship surface + Part VI patron-CDN originate the graduation/recovery story | sha256:5d5f1f85fe7dcfe2 | path: genesis/docs/content/elohim-protocol/resilience/README.md"
  - "session-bridge-design | origin of the staged-participation vocabulary — anti-binary visitor-to-peer graduation patterns (§0) | sha256:1d52dbaa44affce5 | path: genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md"
  - "app-manifest-staged-intents-design | manifest-level grammar for graduation intents (substrate landed, feature held) | sha256:98e0a6576d9a197a | path: genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md"
  - "doorway-access-tier-patterns | the three access tiers each flywheel stage is served by (Tier 1 anon / Tier 2 hosted / Tier 3 steward-via-web) | sha256:f862d55525b442c3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md"
---

# Doorway Two-Axis Scaling

Doorway has two fundamentally different scaling concerns. They are orthogonal, not in tension; they share one process today, and the seam between them is already expressed in config.

```
                    Axis 1: PROJECTION (visitors reading content)
                    │  scales with: replicas, CDN, MongoDB read replicas
                    │  bounded by: content popularity — UNBOUNDED
                    │  resolves: never (success = more readers)
                    │
  ──────────────────┼────────────────── Axis 2: IDENTITY HOSTING
                    │                   (humans in transition to P2P)
                    │  scales with: conductor pool
                    │  bounded by: hosted-user count — BOUNDED
                    │  resolves: graduation flywheel (users leave)
                    │
                    doorway (today: both in one process)
```

Axis 1 is classic web2: cache reads, no agent identity, every web2 scaling technique applies. Axis 2 is the agency transition: custodial keys, conductor cells, per-agent state — and it shrinks as doorway succeeds. The deployed reality is a single doorway replica per environment that scales via the conductor pool behind it, not via doorway replicas (genesis/orchestrator/manifests/doorway/README.md:12).

Doorway is five services in one process: DNS/TLS gateway (both axes, stateless) · bootstrap/signal (axis 2, in-memory) · projection cache (axis 1, MongoDB) · identity host (axis 2, conductor pool) · recovery registrar (axis 2, lightweight metadata). The same five-role decomposition is the deployment's own self-description (genesis/orchestrator/manifests/doorway/README.md:3-9).

## As-implemented

Each mechanic below is live in the tree, cited at source. This section supersedes SCALING.md's "Current State vs Target" table, whose "Multi-conductor routing NOT BUILT / Dynamic agent provisioning NOT BUILT" rows were stale-pessimistic by retirement time.

### Axis 1 — projection read path (LIVE)

- **DHT → MongoDB projection engine**: doorway's own module headline (doorway/doorway-service/src/lib.rs:14). Populated by the signal subscriber (doorway/doorway-service/src/projection/subscriber.rs) connected to the conductor app WebSocket.
- **Tiered resolution**: `DoorwayResolver` wraps `elohim_cache_core::ContentResolver` with the fallback chain Projection → Conductor → External (doorway/doorway-service/src/cache/resolution.rs:3-16,35,91-93). A `projection_only` constructor exists for conductor-less instances (resolution.rs:153).
- **Write/read split**: `PROJECTION_WRITER` env flag, default `true` (doorway/doorway-service/src/config.rs:177-178). Orchestrator manifests deploy a writer (genesis/orchestrator/manifests/doorway/staging.yaml:119, alpha.yaml:113, prod.yaml:139, alpha-b.yaml:148) and a reader Deployment with `PROJECTION_WRITER=false` (genesis/orchestrator/manifests/doorway/staging-read.yaml:3,89), with read-heavy ingress paths routed to the read replicas (staging.yaml:305). Those manifests' "See doorway/SCALING.md" comments repoint to this seed at island retirement (deferred-ref repairs).
- **Blob pantry cache, write-on-fetch**: `/blob/{hash}` is cache-first; pantry hits are served locally with `Cache-Control: public, max-age=31536000, immutable` (doorway/doorway-service/src/routes/storage_proxy.rs:267-281). Range/206/oversized/non-200 are never cached; single-target dispatch, no fan-out (gospel: doorway/CLAUDE.md "No Blob Fan-Out").

OPEN QUESTION: SCALING.md asserted a hot-cache tier of "10k entries, 5 min TTL" and a 1-5% conductor-fallback rate; neither figure was re-verified at source during this distillation — treat as design-era estimates until measured.

OPEN QUESTION: the staging-read manifest exists and is wired, but whether any environment is *currently running* nonzero read replicas is live-cluster state this seed does not assert.

### Axis 2 — conductor pool and identity hosting (LIVE)

- **Conductor registry**: `ConductorRegistry` maps agents to conductors — `register_conductor` (doorway/doorway-service/src/conductor/registry.rs:124), `get_conductor_for_agent` (registry.rs:186), `find_least_loaded` (registry.rs:196), `unregister_agent` (registry.rs:218).
- **Per-request routing**: `ConductorRouter::route(agent_pub_key)` does registry lookup → capacity check → overflow to healthiest pool → auto-assign to least-loaded with persisted assignment → default-pool fallback (doorway/doorway-service/src/conductor/router.rs:51-175). Per-conductor capacity is a configurable agent ceiling (`max_agents_per_conductor`, doorway/doorway-service/src/conductor/pool_map.rs:25,58).
- **Dynamic agent provisioning**: `POST /admin/hosted-users` (doorway/doorway-service/src/routes/admin_conductors.rs:331) calls `AgentProvisioner::provision_agent` → `generate_agent_pub_key` + `install_app` + `enable_app` (doorway/doorway-service/src/conductor/provisioner.rs:78,117,131,141), with `DELETE /admin/hosted-users/{agent_key}` deprovisioning (admin_conductors.rs:455).
- **Custodial key vault**: Argon2id key derivation (64 MiB memory cost) + ChaCha20-Poly1305 authenticated encryption (doorway/doorway-service/src/custodial_keys/crypto.rs:16-17,28-46); decrypted signing keys cached in process RAM keyed by `session_id` — 1-hour TTL, 10k-session cap, zeroize on eviction (doorway/doorway-service/src/custodial_keys/cache.rs:40-41,179-181).

The vault is *why* axis 2 does not scale with replicas: a hosted user's session is pinned to the process that decrypted their key. Identity hosting scales by adding conductors behind the one doorway, not by adding doorways.

OPEN QUESTION: SCALING.md's ~30-50 active hosted agents per conductor (~20-50 MB/cell) was a design estimate; the router's capacity ceiling is an agent *count*, not a measured-load signal, and the estimate has not been load-validated.

## The graduation flywheel

This seed is truth:DERIVED — the graduation and recovery story it scales for originates in the resilience canon (genesis/docs/content/elohim-protocol/resilience/README.md, Part V "Seeing What You Hold" and Part VI "The Patron-Enabled CDN"), and the staged-participation vocabulary originates in the session-bridge design (genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md §0, anti-binary participation) with its manifest-level grammar in genesis/docs/superpowers/specs/2026-05-28-app-manifest-staged-intents-design.md. This section states only what doorway *implements* of that story.

The flywheel: **visitor → hosted human → app user → node steward.** Each stage is served by a different access tier (genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md — Tier 1 anonymous, Tier 2 doorway-hosted, Tier 3 steward-via-web):

1. **Visitor** — projection cache only; zero conductor load (axis 1).
2. **Hosted human** — agent key generated, cells installed in the pool; the most expensive stage (one cell per human, axis 2).
3. **App user** — own device carries the cell; the steward's conductor load *decreases*; doorway keeps recovery registration.
4. **Node steward** — full peer; may run a doorway for their own community; original doorway's role shrinks to DNS/recovery.

Each graduation reduces the steward's identity-hosting load while increasing the network's capacity. It does nothing for axis 1 — popular content needs read capacity regardless.

**What's live is graduation accounting, not migration.** The graduation pipeline endpoints exist: `handle_graduation_pending` lists hosted users not yet graduated (doorway/doorway-service/src/routes/admin_conductors.rs:539,565-566), `handle_graduation_completed` reports freed capacity (admin_conductors.rs:608,634,669-678), and `handle_force_graduation` flips `is_steward: true`, nulls `conductor_id`, and best-effort deprovisions the conductor cell (admin_conductors.rs:685,729-755). This is **flag-state only**: a MongoDB flag plus cell teardown. No source-chain export/import exists anywhere in the crate (verified by search 2026-06-11) — a force-graduated user's hosted chain history does not move to their device. That gap is the first entry of the Vision-remainder ledger below.

## Coupling points and the separation seam

Both axes share one process today. The coupling is thin and enumerable:

| Coupling point | What it does | Live home |
|---|---|---|
| Signal subscriber | DHT signals → MongoDB projection; one writer only | doorway/doorway-service/src/projection/subscriber.rs, gated by `PROJECTION_WRITER` (config.rs:177-178) |
| Cache-miss fallback | Projection miss → conductor zome call | doorway/doorway-service/src/cache/resolution.rs (Projection → Conductor tier) |
| Shared `AppState` | Config, MongoDB handle, worker pools | doorway/doorway-service/src/main.rs — code convenience, not an architectural requirement |

The seam is already half-cut: `PROJECTION_WRITER=false` plus `DoorwayResolver::projection_only` (resolution.rs:153) yields a stateless axis-1 read replica with no conductor dependency — exactly the staging-read Deployment. The uncut half is the axis-2 side: the identity host (vault sessions, conductor routing, admin API, signal subscriber) remains one stateful instance. Full separation is not needed at current community scale; the seam exists so that it stays cheap when it is.

The K8s expression (writer Deployment + read Deployment + edgenode StatefulSet conductor pool) is the developer test-bench, not the architecture (cf. doorway/doorway-service/CLAUDE.md on k8s-as-dev-substrate): the model must hold for three blades in a closet. The human-topology worked examples (PTA steward, extended-family mesh, steward's-own-cell-in-the-pool) live in the retired SCALING.md, recoverable from git via the `derived_from:` entry.

## Vision-remainder (gap ledger)

Open gaps, verified un-homed elsewhere as of 2026-06-11 (searched genesis specs + timeline backlog). Origin for all: the retired SCALING.md Future Work section (see `derived_from:`).

1. **Source-chain migration for graduation** — graduation is flag-state + deprovision only (admin_conductors.rs:685-771); no export of the hosted source chain to the user's device exists in the crate. Until built, "app user" graduation moves the *flag* but strands the *history*. The canonical story-side framing is the resilience canon's recovery/succession arc — this is its missing substrate mechanic, not a new story.
2. **Recovery-registration persistence** — recovery *flow* types exist on the auth surface (doorway/doorway-service/src/routes/auth_routes.rs:463-519: `recovery_method`, `recovery_token`, status check), and the flow design was carried by RECOVERY-PROTOCOL.md (retired to git 2026-06-11; arc preserved in genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-recovery-protocol-arc.md; current canonical design: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md); durable recovery contracts that survive restarts (the "recovery registrar" role of the five-service decomposition) are not implemented. (src/orchestrator/disaster_recovery.rs is *content* replication, not identity recovery — do not conflate.)
3. **Capacity management beyond agent counts** — auto-assign-to-least-loaded IS live (router.rs:107-167; SCALING.md's "NOT BUILT" row was stale), but capacity = a static per-conductor agent ceiling (pool_map.rs:25), not measured RAM/activity/zome-load. Resource-aware assignment remains open.
4. **CDN-friendly blob headers (miss path)** — pantry *hits* carry `public, max-age=31536000, immutable` (storage_proxy.rs:276-279); the cache-miss passthrough response sets only Content-Type + CORP, no immutable Cache-Control. Partial, not absent.
5. **App-static-assets-as-blobs dogfood** — fonts, icons, sophia UMD bundle, HTML5-app ZIPs served as content-addressed blobs via `/blob/{hash}` instead of build artifacts; bootstrap-hybrid first paint; service-worker caching by hash; blob-serving bandwidth as measurable steward work (shefa accounting). No spec or backlog entry homes this today (searched 2026-06-11) — this ledger entry is its only live home.

OPEN QUESTION: whether gap 5 should land as a doorway spec or as part of the patron-CDN story-side work the resilience canon already names (Part VI's not-yet-wired story expressions) — they describe the same distribution surface from opposite ends.
