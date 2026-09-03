---
id: ark-s1-station3b-custody-read-gate-plan
status: in-flight
cites:
  - "compute-envelope-tevah | Tevah | sha256:25153362aae54306 | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "ark-s1-station2-custody-plan | 2026-09-02-ark-s1-station2-custody-plan | sha256:36afd7fdbedd66a5 | path: genesis/docs/superpowers/plans/2026-09-02-ark-s1-station2-custody-plan.md"
---

# Tevah S1 · Station 3b — a stranger is refused: the custody-scoped read gate on the replication plane — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A death witness at `reach: private` leaves a peer only toward a peer with custody standing. Over HTTP an anonymous caller is refused on both the row and the bytes (already true — the story's step must prove it non-vacuously). Over the shard replication plane — libp2p and the iroh shard ALPN, the same `ShardService` — a `private` row and its blob are served only to the ward or to a peer that resolves to an agent holding a live custody commitment for that ward (or for that digest); an unresolved requester is withheld, fail-closed. A receiving peer without standing does not persist a `private` row it is handed. Green = `just test mesh '@concern:death-witness and (@station-2 or @station-3b)'` — station 2 must stay green under the gate, that is the non-vacuity of the whole slice.

**Architecture:** One pure decision predicate, `private_serve_verdict`, placed beside the existing pure `blob_serve_verdict` (`src/blob_reach.rs`) and `classify_pre_authorization` (`src/p2p/reach_authorization.rs`); its facts are resolved by a `CustodyStanding` resolver over the local `rea_commitments` projection (with a bounded own-conductor fallback by deterministic id) and the existing identity maps (`identity_map::PeerIdentityMap` for libp2p, `p2p_iroh::peer_map::lookup_by_iroh_node_id` for iroh). The requester's transport identity is plumbed into `ShardService::handle` — today the PeerId is known at `p2p/mod.rs:5521-5528` and discarded, and the iroh `Connection` is discarded at `p2p_iroh/shard.rs:62-75`. Three serve sites gate (`ListContent` omits, `GetContent` and blob `Get` refuse with a typed `reach-withheld` error); one receive site pre-authorizes (`store_acquired_record`). Nothing at the DHT changes; no wire variant is added.

**This is the two-faces model the crate already states** (`p2p/reach_authorization.rs` module doc): the serving peer is *the steward of what its agent authored* and enforces the reach the author declared (author-side earning, carried by the author's steward); the receiving peer takes standing in the ward's custody scope only through a commitment it itself authored (receiver-side pre-authorization — a scope decision, not a per-message filter: the scope is "custody of ward W", answered once per ward per cycle).

**Spec:** tevah §6 (witness path, custodians inside the declared reach), §6.5 correction "M9", §10 (stations), §12 items 10–12, 16. **Grounding (2026-09-03, read from code by two discovery passes):**

- HTTP: `GET /db/content/{id}` refuses anonymous for non-public reach with **403 + `requiredReach`** (`http.rs:7981-7992`); `GET /blob/{hash}` runs `blob_serve_verdict` and refuses anonymous when any referencing row is gated, **403** (`http.rs:3479-3491`, `blob_reach.rs:119-151`). Both already hold for the witness (station 2's blob leg observed the 403). The `X-Agent-Cid` header is unsigned — a caller on the mesh can self-assert. That is the standing `http-reach-enforcement-gap` backlog item, NOT this slice.
- Shard plane: `ShardService::handle` (`shard_service.rs:73-83`) → `handle_get` (blob bytes, **no DB, no reach**), `handle_list_content` (`MinTrust::Invisible`, comment "peers must see all local rows"), `handle_get_content` (same). `reach_filter` is a WHERE narrowing chosen by the *requester*. Call sites discard the peer identity (libp2p `p2p/mod.rs:5521-5528`; iroh `p2p_iroh/shard.rs:62-75`, `shard_backend.rs:48-49` wraps the same service).
- Receive: `store_acquired_record` (`p2p/mod.rs:10457-10538`) inserts `reach: record.reach` verbatim; shared by libp2p and iroh acquisition.
- Identity: `identity_map.lookup(&peer)` is used by exactly one protocol today (`handle_epr_atom_request`, `p2p/mod.rs:8749-8754`) — the pattern to copy. Bindings are self-asserted (`peer_identity_bindings.proof_status` default `unverified`); this gate is a **routing cut** like `spool_custody_author`'s ward resolution, not an attribution cut.
- Custody knowledge: `rea_commitments` (`provider`, `receiver`, `resource_classified_as`, `state`, `dht_anchor_hash`). Commitments converge from the OWN conductor with peers as discovery only (`projection_reconcile.rs:1-38`); the ward's peer learns a custodian's `custody-spool` id through peer inventory discovery, then `get_rea_commitment(id)` on its own conductor. The id is deterministic (`deterministic_spool_custody_id(provider, receiver, ward)`), so the ward can also compute the expected id for a requester and ask its own conductor once — the bounded fallback below.
- iroh byte plane: `iroh_blobs` `BlobsProtocol` on the router (`p2p_iroh/node.rs:75`) is a third-party content-addressed server with no reach concept. Out of this slice; declared M12.

## P2P design gate (run 2026-09-03)

No new data entity. This slice introduces a **decision predicate** and a **reason enum** over existing entities (Notarized (A) `content` rows on the elohim DNA riding `issue-report`; the blob bytes; `Commitment` rows). Step 1–3 are inherited from the station-2 gate; Step 4 is the birth rule and is answered here.

### Decision point: `private_serve_verdict`
- **Kind:** pure-decision-predicate; **reason enum:** `WithholdReason { UnresolvedRequester, NoStanding, WardUnresolved }`.
- **Inputs (facts, resolved by the caller):** row reach; whether the requester resolved to an agent; whether the requester IS the ward; whether the requester holds a live `custody-spool` whose `receiver` is the ward; whether the requester holds a live `custody-blob` for this digest. "Live" = `state` not in `spool_custody_author::RETIRED_STATES`.
- **Rule:** `public`/`commons` → serve (unchanged). `private` → serve iff requester resolved AND (is ward OR holds spool custody for ward OR holds blob custody for digest); else withhold with the reason. Every other reach → serve (unchanged this slice; declared — widening to `intimate`/`trusted`/`familiar`/`community` is a later station and must not ride this commit).
- **Ward resolution (caller):** a locally-authored row (`dht_anchor_hash` NULL and created by this node's spool ingest — `created_by == passport.node` of this berth) → ward = self `agent_cid` (`resolve_self_agent_cid`, None → withhold `WardUnresolved`); a replicated copy → ward = `receiver` of this peer's own live `custody-blob` naming the digest; neither → withhold `WardUnresolved`.
- **Requester resolution (caller):** libp2p PeerId → `identity_map.lookup`; iroh NodeId → `peer_map::lookup_by_iroh_node_id`; prefer a binding whose `proof_status` is verified when more than one row resolves; none → `UnresolvedRequester`.
- **Standing resolution (caller):** local `rea_commitments` first; if absent, ONE own-conductor `get_rea_commitment(deterministic_spool_custody_id(requester, ward, ward))` behind a per-`(requester, ward)` TTL cache (positive and negative, 60 s) — the uncancellable conductor call is bounded before it is made (`conductor-call-is-uncancellable` rail), and a `ListContent` page performs at most one such call per distinct requester, never one per row.
- **Refusal rendering:** `ListContent` omits withheld rows from the page (offsets walk the unfiltered set; `total` may over-report; declared); `GetContent` and `Get` answer `ShardResponse::Error("reach-withheld: <reason>")` — never `NotFound`/`ContentNotFound` (C4: a refusal is not an absence). No new wire variant (C10).
- **Observability:** counter `storage_private_withheld_total{site, reason}` + one `info` line per withhold with requester peer id, row id, reason; a `debug` line per serve-to-custodian.
- **Network stakes:** all four stages; floor-protected — a `private` row never cheapens at Simulacra.
- **Concern canon (C0–C14):** C0 answered (decision on the serving peer's storage = the author's steward; receive-side on the acquiring peer); C1 n-a; C2 answered (a verdict is never persisted, withhold never escalates, serve never stamps authority); C3 answered (a withheld page still returns; the conductor fallback is cached and single-shot); C4 answered (typed refusal ≠ not-found; omission from a listing is counted); C5 answered (standing derives from a notarized commitment row, never from the requester's claim); C6a answered (≤1 standing query per distinct requester per page, TTL-cached fallback); C6b answered (pure); C7 **partial** — inventory gossip still advertises private blob hashes to every peer (custodians need the advert; strangers learn a hash exists, never bytes) → M13; C8 answered (metric + log per decision); C9 **partial** — bindings self-asserted, Stage 1 floor; cross-signed binding (`identity-cross-signed` habit) is the named blocker before this gate may carry economic weight; C10 answered (no wire change; an old peer simply sees fewer rows); C11 n-a; C12 **answered — this is the concern**: consent is the counter-signed `custody-spool`; C13 answered (ward > custodian > stranger, by evidence class); C14 answered (withheld count + reason surface in metrics; residual = iroh byte plane M12).
- **Registration:** row in `elohim/elohim-storage/seam-registry.yaml` with `contractTests` naming the unit tests below (never `null` here — the tests land in the same commit).

### Design constraints discovered
- The witness row carries `created_by = passport.node` (a berth label), not an agent id — ward identity on a *replicated* copy is derivable only through the holder's own `custody-blob` row. That is fine for this slice (a custodian serves onward only to peers with standing for the same ward) and is recorded as the reason `WardUnresolved` exists.
- Station 2's chain depends on the custodian being SERVED the private row by the ward. If the ward's peer does not hold the custodian's `custody-spool` projection when the request arrives, the deterministic-id conductor fallback is what keeps station 2 green. The mesh run is the arbiter; if it reds, the fallback is the first suspect.
- Story posture (deferred from the station-2 authoring pass, decided here): the spec's reach contract is **403 with `requiredReach`** — deny access, acknowledge existence. Chosen over 404-hides-existence because the household's own non-vacuity control must distinguish "refused" from "missing"; existence of a *hash* is already advertised by inventory gossip (M13), so hiding it at HTTP would be theatre.

## Global Constraints

- Everything in the station-2 plan's Global Constraints holds (identity = one digest two renderings; row before blob; storage pulls; custodian authors; build environment; managed surfaces; commit discipline; never push / kubectl / touch mesh processes).
- **The mesh is held by the 0.7 cutover session.** No task in this plan starts, stops, restarts, or joins a mesh peer. Mesh evidence is a staged final leg the orchestrator runs when the mesh is released ("mesh free").
- **No breaking change stages:** `public`/`commons` and every non-`private` reach replicate exactly as today. The only behavioural change is for `private` rows, which before this slice should never have crossed peers except by custody.
- **Build env:** `cd /projects/elohim/elohim/elohim-storage && CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS='--cfg getrandom_backend="custom"' cargo <cmd>; echo EXIT=$?`. Focused: `cargo test --lib <module>`; the iroh tests: `cargo test --features "p2p p2p-iroh" --test iroh_shard_real_backend --test iroh_shard_parity`. Gate: `just gate elohim-storage`.

---

## File structure

```
elohim/elohim-storage/src/private_reach.rs               NEW  private_serve_verdict + WithholdReason + PrivateServeFacts (pure; unit tests)
elohim/elohim-storage/src/services/custody_standing.rs   NEW  CustodyStanding resolver: local rea_commitments → own-conductor deterministic-id fallback (TTL cache); trait for tests
elohim/elohim-storage/src/shard_service.rs               modify: handle(requester: Requester, req) — gate ListContent / GetContent / Get
elohim/elohim-storage/src/p2p/mod.rs                     modify: pass `peer` into handle_shard_request; store_acquired_record pre-authorizes private rows
elohim/elohim-storage/src/p2p_iroh/shard.rs              modify: accept() passes connection.remote_node_id() through
elohim/elohim-storage/src/p2p_iroh/shard_backend.rs      modify: backend.handle(requester, req)
elohim/elohim-storage/src/metrics.rs                     modify: storage_private_withheld_total{site,reason}
elohim/elohim-storage/seam-registry.yaml                 modify: private_serve_verdict row
elohim/elohim-storage/tests/iroh_shard_real_backend.rs   modify: private row withheld from an unresolved iroh requester
genesis/a2o/steps/mesh/death-witness.steps.ts            modify: station 3b steps (anonymous fetch on a mesh peer + non-vacuity control; late-joiner withheld)
genesis/a2o/features/resilience/death-witness.feature    modify: 3b un-@wip; 3b-ii (replication plane) added @wip until measured; glossary: custody wording, envelope/ark, 3a doorway Given
elohim/elohim-storage/.epr-meta/runtime-death-witnessed.habit.md   DELTA (after the mesh run)
genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md   §6.5 addendum: M12, M13, posture decision
```

---

## Tasks

### Task 1: the pure predicate + the standing resolver + serve-side wiring (libp2p and iroh)

**Executor:** Opus (`rust-architect`). **Reviewer:** Codex (read-only review script). **Files:** create `src/private_reach.rs`, `src/services/custody_standing.rs`; modify `src/shard_service.rs`, `src/p2p/mod.rs` (call site only — `handle_shard_request(peer, request)`), `src/p2p_iroh/shard.rs`, `src/p2p_iroh/shard_backend.rs`, `src/metrics.rs`, `src/lib.rs`/`src/services/mod.rs` (mod lines), `seam-registry.yaml`, `tests/iroh_shard_real_backend.rs`.

- [ ] `private_reach.rs`: `PrivateServeFacts { reach, requester_resolved, requester_is_ward, custody_for_ward, custody_for_digest, ward_resolved }`, `WithholdReason`, `PrivateServeVerdict::{Serve, Withhold(WithholdReason)}`, `pub fn private_serve_verdict(&PrivateServeFacts) -> PrivateServeVerdict`. Tests: public serves to anyone; private + unresolved → `UnresolvedRequester`; private + ward → serve; private + spool custody → serve; private + blob custody → serve; private + resolved stranger → `NoStanding`; private + ward unresolved → `WardUnresolved`; every other reach serves (pin the *declared* scope so widening is a deliberate edit).
- [ ] `custody_standing.rs`: `pub struct Requester { pub transport: TransportId /* Libp2p(PeerId) | Iroh(NodeId) | Local */, }`, `pub trait CustodyStanding { fn facts_for(&self, requester: &Requester, row: &RowFacts) -> PrivateServeFacts; }`, production impl over `DbPool` + `Arc<dyn PeerIdentityMap>` + iroh peer_map + optional `Arc<HcClient>` for the deterministic-id fallback with a `(requester_agent, ward) → (bool, Instant)` TTL cache (60 s, both signs). `RowFacts { reach, id, blob_digest (multihash digest, via BlobStore::parse_content_address), created_by, dht_anchor_hash_present }`. Ward resolution per the gate. A fake impl for tests.
- [ ] `shard_service.rs`: `handle(&self, requester: &Requester, req)`; `ListContent` filters withheld rows out of the returned page and counts them; `GetContent` and `Get` return `ShardResponse::Error(format!("reach-withheld: {reason}"))`. `Get` resolves referencing rows the way `blob_reach.rs` does (reuse its query; do not duplicate). Existing tests updated to pass a `Requester::Local`-style requester that preserves their behaviour; NEW tests: private row omitted from a stranger's `ListContent` page and present in a custodian's; `GetContent` refused with the `reach-withheld` prefix; `Get` refused for a blob referenced only by a private row.
- [ ] libp2p call site: `p2p/mod.rs` request arm passes `peer` → `Requester::Libp2p(peer)`. iroh: `shard.rs` `accept` reads `connection.remote_node_id()` → `Requester::Iroh(node_id)`; `shard_backend.rs` threads it. Integration test in `tests/iroh_shard_real_backend.rs`: a private row is withheld from an unresolved iroh requester and a public row still lists.
- [ ] `metrics.rs`: `storage_private_withheld_total{site, reason}`.
- [ ] `seam-registry.yaml`: `private_serve_verdict` row (kind `pure-decision-predicate`, C0–C14 statuses as in the gate, `contractTests` = the test names above).
- [ ] Clippy `-D warnings` clean; `cargo test --lib private_reach services::custody_standing shard_service`; the two iroh test targets. Commit as ONE path-limited commit: `feat(storage): custody-scoped read gate on the shard plane — private rows serve only to the ward or a standing custodian (station 3b, M9)`.

### Task 2: receiver-side pre-authorization

**Executor:** Codex. **Reviewer:** Opus. **Files:** `src/p2p/mod.rs` (`store_acquired_record` and the iroh acquisition path that shares it), `src/metrics.rs`, tests beside the existing acquisition tests (`tests/acquisition_pull_e2e.rs` fixture uses `reach: "public"` — add a `private` case). Depends on Task 1's `CustodyStanding`.

- [ ] Before inserting a received record with `reach == "private"`: resolve this peer's standing for the record's ward (self is never the ward here — the record came from a peer — so ward = the sender's agent via the same identity map, else the receiver of a local `custody-blob` for the digest); with no standing → skip the insert, count `storage_private_preauth_skipped_total{reason}`, log once per (sender, ward) per cycle. With standing → insert as today.
- [ ] Test: a private record from an unresolved sender is not persisted; a private record from a sender this peer holds a live `custody-spool` for is persisted; a public record is persisted regardless.
- [ ] Commit: `feat(storage): a peer without custody standing does not keep a private row it is handed (station 3b, receiver-side pre-authorization)`.

### Task 3: the story — station 3b steps, 3b-ii, and the deferred glossary pass

**Executor:** Sonnet (`general-purpose`). **Reviewer:** blind-reader (fresh, per `genesis/a2o/.epr-meta`), then the orchestrator. **Files:** `genesis/a2o/steps/mesh/death-witness.steps.ts`, `genesis/a2o/features/resilience/death-witness.feature`. Independent of Tasks 1–2 (disjoint files).

- [ ] Steps: `Given Jessica's peer holds a death witness` — reuse `listWitnesses`/`selectedWitness` (ark CLI, berth path) AND prove the row exists over HTTP on that peer with a credential that the substrate honours on the mesh (admin key `API_KEY_ADMIN` on `GET /db/content/{cid}` if that route honours it; otherwise the admin listing) — the control that makes the refusal non-vacuous. `When an anonymous caller fetches that witness on peer {string}` — resolve the peer with `meshPeer()` (household fixture, NOT `resolvePeerUrl`), GET `/db/content/{cid}` and `/blob/{blobRef}` with no headers, store both statuses. `Then the fetch was refused with a non-success status` — both statuses non-2xx, and the control was 2xx; assert the 403 body carries `requiredReach` when present (soft: log if absent).
- [ ] 3b-ii scenario (new, `@wip @station-3b-ii @requires:owned-substrate`): a fresh household peer with NO custody commitment for Jessica joins the running mesh (`just mesh join-peer <fresh-name>` — pick a fixture human not among the three; read `genesis/a2o/scripts/late-joiner-receipt.ts` for the staging shape), Jessica's conductor is killed, within 120 s Matthew and James hold the witness and the joiner does not, and the joiner's peer counts one pre-authorization skip. Steps may be authored against the mesh dials but stay `@wip` until measured — the orchestrator runs it.
- [ ] Glossary pass (deferred findings from the station-2 blind read): (a) 3a's Given names a doorway the mesh serves — check `MESH_PORTAL`/doorway :8888 wording matches the Background; (b) posture sentence under REACH: "refused, and told that a reach it does not have is required" (403 acknowledges existence); (c) "custody" wording: the custodian *keeps a copy*, it does not *own*; (d) the ENVELOPE entry gains "(the `ark` binary in code)".
- [ ] Un-`@wip` station 3b. Run `pnpm exec cucumber-js --dry-run` (or the a2o lint) to prove every step binds; run the blind-reader loop to READY. Commit: `story(a2o): station 3b — a stranger is refused; 3b-ii staged — a peer without standing receives nothing over the replication plane`.

### Task 4: risk check — does anything else depend on private rows crossing peers?

**Executor:** Haiku (`Explore`). Read-only, before Task 2 lands.

- [ ] Grep `genesis/a2o/features/**` and `elohim/elohim-storage/tests/**` for scenarios/tests where a `private`-reach row or blob is expected to be present on a peer other than its author's, other than via custody (station 2). Report each with file:line and whether custody covers it. Any hit is a plan amendment (a per-kind exemption is NOT allowed — the answer is a custody commitment for that flow, or the row is not `private`).

### Task 5: evidence, register, spec (orchestrator)

- [ ] `just gate elohim-storage` green; `just gate genesis-a2o` (or the a2o gate the manifest names) green.
- [ ] When the mesh is released: rebuild the storage slot with `--features "p2p p2p-iroh"`, `just mesh storage-restart matthew jessica james`, `just mesh prologue`, then `just test mesh '@concern:death-witness and (@station-2 or @station-3b)'`; then stage 3b-ii with `just mesh join-peer`. Receipt ids into the habit DELTA.
- [ ] Spec §6.5 addendum: M12 (iroh byte plane has no reach), M13 (inventory gossip advertises private hashes — advertise/serve asymmetry), posture decision; register item if any decision changed.
- [ ] Habit DELTA + `habits-project.py`; status stays red until station 4.

---

## Landing record

(filled at the end; commits, deviations, measurements, defects found only on the mesh)
