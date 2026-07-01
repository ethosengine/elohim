# CRDT-Authoritative Content State, DHT-Notary Decoupled — a three-layer non-brittle content dataplane with proven resiliency + defense-in-depth per layer

**Status:** DRAFT (spec, not implementation) · 2026-07-01
**Author:** synthesized from six ground-truth surveys, three per-layer designs, one coherence/sequence design, and five adversarial code-verified verdicts
**Scope:** the elohim-storage content dataplane (`elohim/elohim-storage/`) — the convergence, notarization, and serving of `content` rows
**Companion docs this composes with:** the Automerge content-sync lighting plan (§Execution Outcome), `2026-06-27` resilience-facings §11–12 decision, `2026-06-15-coherent-transport-identity-resolver-design.md`, the seam-map concern-routing atlas §3.10/§3.13, and the versioned-HEAD-is-declared-dependency policy seed
**Load-bearing convention:** cites are `file:line` in `elohim/elohim-storage/src/` unless otherwise noted; `[VERDICT]` tags mark claims that survived or were refuted by adversarial code review and are carried here as spec requirements, never papered over.

---

## 1. Vision & Framing

### 1.1 The canon: the DHT is a NOTARY, not a database

The Holochain DHT does not *hold* content state and is not in the critical path of content-state convergence. It witnesses **provenance, authority, and trust** *over* state that lives and converges elsewhere. This is the P1 pattern (MEMORY `project_principle_p1_reconciliation_controller`) made concrete for content: **DHT = manifest/notary, libp2p = reconciliation controller, SQLite = read-optimized projection.**

Code confirms the canon today for *what it currently does*: notarization only *stamps* `dht_anchor_hash` onto a pre-existing SQL row via the `ContentCommitted` signal projection (`rea_projection.rs:142,694-699`; `content_diesel.rs:682-720`); the DHT is the queryable-authority, SQLite the queryable-store; reads gate on a provenance *marker*, not on the DHT being the query surface (`content_diesel.rs:171-177`). `GROUND[dht-notary]§5` and `GROUND[sqlite-serving]§3` both confirm: *the DHT authorizes; SQLite serves.* **[VERDICT L2/L3: SURVIVED]**

### 1.2 The two-jobs conflation is the brittleness

The live production failure — `elohim.host/` returns 404 "App not found" (`http.rs:5597-5605`, ~5601) — is a **conflation bug**, not a database bug. The landing row's `blobHash` is null, and *the only updater of an existing row's `blobHash`* is the notarized, conductor-gated PATCH (`patch_needs_conductor` → `update_via_conductor`, `http.rs:10080`, `content_service.rs:394-418`). When the conductor/notary is down, that PATCH is `503` fail-loud with no diesel fallback (`http.rs:4965-4972`). So **a state update (fill a field) is welded to a trust operation (notarize authority)**: the field cannot be corrected while trust is unconfirmable. One job (converge the value) is being held hostage by the other (attest the author). Decoupling these two jobs is the entire thesis.

### 1.3 The three layers

| Layer | Owner of | Substrate | Down means | In convergence critical path? |
|---|---|---|---|---|
| **L1 — Automerge CRDT plane** | authoritative **convergent content state** (the *value*) | libp2p swarm + sled DocStore + Automerge (`sync/`, `p2p/mod.rs`) | nothing converges *further*, but existing state is intact | **YES — this IS the critical path** |
| **L2 — Holochain DHT notary/controller** | **authority, provenance, HEAD-of-DAG selection** (the *witness*) | conductor + DNA signals (`rea_projection.rs`, `reconcile/controller.rs`) | trust **unconfirmed**, not state lost | **NO — must never be** |
| **L3 — SQLite serving projection** | **read-optimized serving** of converged state, stamped on confirmation | diesel `content` table (`content_diesel.rs`) | serving degraded/rebuildable; **losing the file loses nothing** | NO — a cache, never truth |

### 1.4 The non-brittleness law (four invariants)

- **LAW-1 — No layer brittle in-itself.** Each layer degrades gracefully and rebuilds from the others. Losing SQLite loses nothing; losing the notary loses trust-confirmation, not state; losing a peer loses convergence-progress, not the converged corpus.
- **LAW-2 — Convergence NEVER hard-depends on notarization.** State convergence (L1) and serving of converged state (L3) proceed with **zero notary reachable**. Notarization is a *late overlay*, never a precondition.
- **LAW-3 — HEAD stays DECLARED/notarized, never recency.** The CRDT converges the *version DAG*; the notary *elects HEAD* as a declared dependency (cid-pin = lockfile semantics — MEMORY `project_versioned_entity_head_is_declared_dependency`). Convergence ≠ binding-selection.
- **LAW-4 — Couple last.** Tight coupling between layers lands **only after** each layer is independently battle-ready and has a standing resiliency proof.

### 1.5 The couple-last posture (why sequencing is load-bearing)

The adversarial verdicts (below, §3 and §4) establish that most of the naive "just heal blobHash via CRDT" cure rests on **unbuilt code** and on an invariant (**HEAD-is-declared**) the current code **actively contradicts** via recency-advancing writes. Therefore the build is sequenced: **Phase A** builds and proves the converge-heal leg standalone; **Phase B** overlays notarization asynchronously; **Phase C** couples only after A+B are soak-green. Claiming "PROVEN" before the code exists is itself the anti-pattern this spec guards against.

### 1.6 The organizing analogy (architect, 2026-07-01): CRDT : HTTP :: CRDT+DHT-notary : HTTPS

The whole architecture collapses to one line every engineer already believes about the web:

> **The CRDT plane is to HTTP as CRDT + the DHT-conductor notary is to HTTPS.**

The transport (CRDT convergence) is the same in both; the notary adds an **additive trust layer** (its Ed25519 signature over the declared HEAD ≈ a TLS certificate), never a precondition for the bytes moving. This is not a loose metaphor — it *derives* four design commitments:

| Web | Here | Consequence |
|---|---|---|
| HTTP works with no cert | CRDT converges + serves with **no notary reachable** | LAW-2 non-brittleness; restated as web-obvious |
| HTTPS = HTTP + TLS (transport unchanged) | notarization = convergence + signature-over-HEAD (convergence unchanged) | couple-last; notary is a late overlay (§4.2) |
| Browser renders HTTP, shows **"Not secure"** — never 404s it | serve the **amber** (converged/unconfirmed) tier, mark `trust:"unconfirmed"` | REQ-F3 validated; the tri-state gate IS the security indicator |
| Padlock = validated cert | **green** = `dht_anchor_hash` notarized | trust tier legible to human (UI) AND machine (view `trust` field) — **REQ-F10** |
| HTTP content gets no cert-gated privilege | amber content is **never consumed for authority/attribution** | §4.3.1 tri-state; economic reads require green |
| **Certificate revocation (CRL/OCSP)** — revoked cert refused even though bytes fetch | **notary revocation dominates convergence** — OD-6/§9-R4 | two cases (see below) |
| **TLS handshake** authenticates the peer | **Ed25519 author-signature on the sync path** (REQ-N5) authenticates the author | author-sig is what turns amber into a *validatable* green |
| **Downgrade / SSL-strip attack** | a peer stripping the notarization to force a green row to serve as amber | new guarded anti-pattern (§7.3) |

**REQ-F10 (new, from the analogy) — trust tier is legible.** Every content view carries an explicit `trust: "notarized" | "published" | "unconfirmed"` field; the app surfaces it (a padlock/"unverified" affordance), and no authority/attribution/economic read consumes anything below `notarized`. This is defense-in-depth at L3 (the "browser") — the human is never silently served unconfirmed state as if it were confirmed.

**OD-6 sharpened by the CRL analogy — revocation has two classes:** *authority-revocation* (the author lost standing) → the row **downgrades to amber** (served, trust withdrawn), like an expired cert; *safety-revocation* (harmful/illegal content) → the notary assertion **hard-suppresses serving** (HSTS-style refuse), dominating L1 convergence entirely. The revocation reconciler (absent for Content today, §3.2) must distinguish them.

---

## 2. Requirements & Goals

Each requirement is checkable. `[MUST-CLOSE]` marks a requirement that exists *because* an adversarial verdict refuted a load-bearing assumption; these are gaps converted to obligations, not assumptions carried forward.

### 2.1 Functional requirements

- **REQ-F1 — Converge without a notary.** A content field edit authored on peer A MUST converge to peer B over libp2p with no conductor/DHT reachable in the round. *(Basis: `GROUND[crdt-plane]§5` "None for convergence" — SURVIVED. Proof: G3 `doc_authored_on_a_converges_to_b`.)*
- **REQ-F2 — Heal an existing stale/null serving field from converged state.** A converged content doc MUST be able to re-derive a serving-critical field (`blob_hash`) back into the `content` SQL row. **[MUST-CLOSE: the DocStore→SQL reverse consumer does not exist today — `GROUND[crdt-plane]§2`, VERDICT L2 #1, VERDICT coherence "SURVIVED: no reverse consumer".]**
- **REQ-F3 — Serve converged-but-unconfirmed state.** A row whose value has converged but whose author-authority is not yet notarized MUST be servable (not 404), distinctly marked `trust: unconfirmed`. **[MUST-CLOSE: the current gate is a binary `dht_anchor_hash OR p2p_published_at` marker test (`content_diesel.rs:171-177`) that cannot distinguish CRDT-converged from un-vetted junk — VERDICT L2 #2. A distinct `crdt_converged_at` column and a tri-state trust tier MUST be added.]**
- **REQ-F4 — HEAD selection is declared/notarized.** The serving field applied MUST correspond to the *declared/notarized* version HEAD, never "latest Automerge heads." **[MUST-CLOSE: no declared-HEAD / version-DAG / cid-pin logic exists; the content doc is a FLAT single-value LWW map (`projector.rs:118-144`, `tx.put(ROOT,"blobHash",…)`); `upsert_with_anchor` is pure recency (`content_diesel.rs:682-720`) — VERDICT L1 R1, VERDICT L2 #3, VERDICT coherence R3. A real notarized HEAD pointer AND a multi-version doc structure MUST be added before any HEAD-is-declared claim holds.]**
- **REQ-F5 — The healed row targets the serving namespace.** Any Doc→SQL heal write MUST target `AppContext::default_lamad()` (`h_app_id="lamad"`), the namespace serving and the producer read under — NOT the sync doc's `"elohim"` namespace. **[MUST-CLOSE: written-under-X-read-under-Y dormancy trap — VERDICT L1 R3.]**
- **REQ-F6 — Late notarization stamps without re-converge.** When the notary returns, it MUST stamp `dht_anchor_hash` onto an already-serving row via the existing signal projection path, with no re-convergence and no row churn. *(Basis: `GROUND[dht-notary]§2`, `main.rs:1704-1729` late-connect re-notarize — SURVIVED.)*

### 2.2 Non-functional requirements

- **REQ-N1 — Per-layer robustness.** Each of L1/L2/L3 MUST have a Failure-Modes→Defenses table (§4) and a standing resiliency proof (§4) that runs in the environment capable of proving it (`GROUND[resiliency-proof-surface]§3`).
- **REQ-N2 — No brittle layer / graceful degradation.** LAW-1: each layer degrades to a defined reduced-function state, never to total failure, on the loss of any *other* layer.
- **REQ-N3 — Defense-in-depth per layer.** Every named failure mode MUST have a named defense that lives in the same layer or an explicitly-sequenced future item (never assumed).
- **REQ-N4 — Proven resiliency, not asserted.** No layer may be labeled "PROVEN" until the proof is a real test with teeth (RED when the mechanism is removed). Simulated in-process exchanges (`sync_integration.rs`, "simulate the sync protocol exchange", `GROUND[resiliency-proof-surface]§2`) do NOT satisfy this for transport claims. **[MUST-CLOSE: the two-node convergence proof `doc_authored_on_a_converges_to_b` asserts only `get_doc_field` on B's DocStore, never `get_content` from SQL — VERDICT L2 #1. Serve-path proofs MUST assert `get_content(require_provenance=true)`.]**
- **REQ-N5 — No poisoning via heal.** Stamping a serving-provenance marker (`p2p_published_at` or `crdt_converged_at`) on a converged value MUST NOT launder unauthenticated peer input into serving-provenance. **[MUST-CLOSE: the sync protocol has NO auth — any connected peer lists+pulls+injects `elohim` docs (`GROUND[crdt-plane]§4`); AgentPeerBindings carry `STAGE1_SIGNATURE_SENTINEL` (`identity_binding_gossip.rs:129`) — VERDICT L1 R2. Ed25519 author-signature verification on the sync path MUST land before the heal stamps any provenance marker.]**
- **REQ-N6 — Write arbitration is explicit.** Three writers race the same lamad `content` row — heal UPSERT, shard `bulk_create_content` insert-or-skip (`content_diesel.rs:443-446`), notary `upsert_with_anchor` (`content_diesel.rs:682-720`). Precedence MUST be defined, not assumed convergent. **[MUST-CLOSE: `upsert_with_anchor` mirrors `blob_cid`→`blob_hash` and can overwrite a healed value; "same value, no conflict" is asserted, not guaranteed — VERDICT L1 R5, VERDICT coherence UNCERTAIN.]**
- **REQ-N7 — Per-row degradation, never fail-closed collect.** Serving-side trust-tier filtering MUST degrade per-row (drop a malformed row, serve its siblings), never fail-closed collect + array-wrap → whole-router-empty (MEMORY `project_epr_router_empties_on_poisoned_scope`).
- **REQ-N8 — Rebuildability.** The SQLite serving projection MUST be reconstructible from L1 (DocStore) + L2 (re-notarization) + peer blob custody. **[MUST-CLOSE: step 1 (DocStore→SQL) is net-new; step 2 `backfill_content_docs` is env-gated OFF by default (`ELOHIM_DOCSTORE_BACKFILL`, `main.rs:2645`); "losslessly rebuildable" is aspirational until both land AND the empty-string-blob_hash hazard is closed — VERDICT L3 #5.]**

### 2.3 The central open goal (the hardest gap)

`blob_hash` is **class-A DNA-notarized** (`http.rs:10067-10082`, `4942`; `patch_needs_conductor` gates on it, `http.rs:10080`) AND is the field the naive cure wants the CRDT plane to own and heal. These cannot both be true unchanged. **GOAL-0: reconcile "CRDT owns the convergent value" with "`blob_hash` is notarized"** — either reclassify what the CRDT plane authoritatively owns, or carry `blob_hash` under a notarized HEAD-pointer that the CRDT converges as a *DAG of declared versions* rather than an LWW scalar. This is Open Decision **OD-1** (§7) and gates Phase C.

---

## 3. Current-State Ground Truth (corrected, per layer)

This section is deliberately accurate about what exists vs. what is aspirational, because every "cure" and "proof" in the naive design was found to rest on one or the other.

### 3.1 L1 — Automerge CRDT plane (as it exists)

- **Producer foundation LANDED** 2026-07-01 (commit `08b284fc8`): `spawn_content_projection_listener` (`projector.rs:238`) is event-driven on `ContentCreated/Updated`, idempotent via `doc_matches` (`projector.rs:93`), wired at startup libp2p-only (`main.rs:2628`). Namespace coupling is compile-enforced (`PROJECTION_NAMESPACE` both sides + `projection_namespace_matches_sync_timer` wire-contract test, `projector.rs:284,360-371`; `p2p/mod.rs:7001`). **[SURVIVED]**
- **Convergence is real and notary-free.** Full libp2p round trip: 60s `sync_interval` (`p2p/mod.rs:2271`) → `initiate_sync_round` (`:6982`) → `ListDocuments{h_app_id="elohim"}` (`:6995`) → head-diff (`:6490`) → `SyncChanges{have_heads}` (`:6511`) → `save_after` delta (`sync/mod.rs:129`) → `apply_changes` (`:6564`) → `load_incremental` → `doc_store.save` (`sync/mod.rs:81-90`). Merge is commutative/idempotent. G3 `doc_authored_on_a_converges_to_b` is a **real two-node libp2p test**, RED at 30s if `apply_changes` is skipped. **[SURVIVED]**
- **The doc is metadata-only and FLAT.** `project_content_doc` writes a single-value map (`tx.put(ROOT,"blobHash",…)`, `projector.rs:130-142`) — ONE `blobHash` per id, LWW-overwritten every projection. There is **no version DAG, no `binding_action_hash` in `sync/`, no `resolve_head`** (`node:{id}` keys the content slug, not a version). **[VERDICT L1 R1: REFUTED the "DAG is the lineage" claim.]**
- **`blobHash` is written as EMPTY STRING when null.** `projected_fields` does `FieldVal::S(content.blob_hash.clone().unwrap_or_default())` (`projector.rs:70-72`). On a conductor-less mesh where the PATCH never fired, **every peer's doc carries `blobHash=""`** — convergence propagates emptiness. **[VERDICT L3 #1: the cure presupposes the correct hash exists somewhere in L1; the coupling it cures prevents that.]**
- **No reverse projection.** `apply_changes` writes DocStore (sled) ONLY (`sync/mod.rs:90`); grep confirms no DocStore→SQL consumer. The projector header's "re-derive a lost blobHash by converging this doc" (`projector.rs:36-43`) is **aspirational**. **[VERDICT coherence: SURVIVED — genuinely net-new.]**
- **Inert/partial today:** backfill env-gated OFF (`main.rs:2645`); `ContentBulkCreated` ignored (`projector.rs:261`); iroh fills DocStore but has **no round driver** (`main.rs:2682-2686`, backlog `iroh-sync-round-driver-gap.md`); `GetChanges` stubbed to full-sync (`p2p/mod.rs:6369`); `StreamTracker` is `#[allow(dead_code)]` (`sync/mod.rs:41`).
- **No auth on the sync path.** Any connected peer can list+pull all `"elohim"` docs including body (`GROUND[crdt-plane]§4`). **[VERDICT L1 R2: poisoning vector.]**

### 3.2 L2 — DHT notary (as it exists)

- **Notarizes the Content entry** as a signed source-chain action carrying `action_hash` + entry payload (`rea_projection.rs:666-693`). `reach` and `blob_hash`/`blob_cid` are the **class-A DNA-notarized fields** (`http.rs:10067-10082`). Two distinct provenance markers: `dht_anchor_hash` (ActionHash) and `p2p_published_at` (Kad publish stamp) (`content_diesel.rs:156-159`). **[SURVIVED]**
- **Row gets its anchor via signal projection.** `ContentCommitted` signal → `upsert_with_anchor` UPDATEs the pre-seeded row and **always sets `dht_anchor_hash = action_hash`, even on re-projection** (`content_diesel.rs:682-720`). This is **pure recency by signal-arrival order** — there is NO declared-HEAD arbiter. **[VERDICT L2 #3: REFUTED the "never recency" claim.]**
- **Notarized fields are conductor-only.** `blob_hash`/`reach` route through `update_via_conductor`; no conductor → `503`, no diesel fallback (`http.rs:4965-4972`). Non-notarized fields (title/description/tags, `server_blob_hash`) take the diesel-direct path (`http.rs:4974`). **[SURVIVED — this is the bug's mechanism.]**
- **No content reconciler.** `reconcile/controller.rs` handles KeyRotation/Revocation/AgentPeerBinding/RevocationAttestation/PortalHost/Collective/Membership (`:324-888`) — **NOT Content**. So the repeated justification "a diesel-only blobHash write is reverted by the reconciliation controller (DHT wins)" (`http.rs:4968,10068`) is **stale/inaccurate for content**. A wrong/empty converged `blob_hash` is **not actively reverted**; it persists until the next `ContentCommitted` signal fires. **[VERDICT L3 #4 + VERDICT coherence UNCERTAIN: "DHT wins" is passive, not active — worse than a revert, it's LWW-in-SQL with no arbiter.]**
- **AgentPeerBinding is self-asserted.** Gossip carries `STAGE1_SIGNATURE_SENTINEL` — agent↔libp2p keys uncrossed/unsigned (`identity_binding_gossip.rs:129,170`; `controller.rs:623`). Peer-converged state carries **no cryptographic author proof**. **[SURVIVED — sound caveat; the resolver spec `2026-06-15-coherent-transport-identity-resolver-design.md` is blocked on exactly this.]**

### 3.3 L3 — SQLite serving projection (as it exists)

- **SPA mount keys on `blob_hash`** via `lookup_slug_blob_hash` → `slug_index` warm cache → DB fallback `list_content(...,true)` (`http.rs:5449-5605,5811,5795-5843`). The canonical 404 is `http.rs:5597-5605` (~5601), slug branch, `lookup_slug_blob_hash` returned `None`.
- **`require_provenance` gate is a binary OR:** `dht_anchor_hash IS NOT NULL OR p2p_published_at IS NOT NULL` (`content_diesel.rs:171-177,201-207,309-315,960`; comment `:305` "Either marker is sufficient"). External HTTP passes `true`; internal drain/sync/replication passes `false`. A libp2p-Kad-published replica passes with **`p2p_published_at` alone, no `dht_anchor_hash`** (`GROUND[sqlite-serving]§2` — SURVIVED). But there is **no "converged" bit** — you cannot distinguish CRDT-converged from un-vetted junk at this gate (`bulk_create_content` sets `dht_anchor_hash:None` and never writes `p2p_published_at`, `content_diesel.rs:459-466`). **[VERDICT L2 #2 / L3: MUST-CLOSE with a distinct column.]**
- **Heal-on-read covers BYTES, not PROVENANCE.** `get_blob_or_heal` race-fetches missing *bytes* from peers (`http.rs:5622-5649`), but a row missing *both* provenance markers 404s at the SQL gate before bytes matter (`GROUND[sqlite-serving]§4`). **[SURVIVED.]**
- **Rebuild story is partly aspirational.** `backfill_content_docs` pages all rows provenance-ungated (`projector.rs:162-211`, `list_all_content_rows` `content_diesel.rs:257-268`) but is env-gated OFF; the DocStore→SQL leg is absent. **[VERDICT L3 #5.]**

### 3.4 Shard/replication plane (adjacent, stays separate)

- **Two hops over `shard_protocol`** (distinct from `sync_protocol`): discovery `ListContent{limit:5000}` (`p2p/mod.rs:7024,7045`) → gap diff (`:4105`) → `GetContent` bounded by `MAX_REPLICATION_INFLIGHT=50` (`:7074`); fetch `ShardResponse::Content(record)` carries the full record but **not blob bytes** — bytes pull separately via `ShardRequest::Get{hash}` (`:4233`). Writes to `content` under `h_app_id="lamad"` via `bulk_create_content` (`content_diesel.rs:468`).
- **Insert-or-SKIP, NOT upsert** (`content_diesel.rs:443-446`): an existing row is skipped entirely — **a replicated record with a new `blob_hash`/`reach`/`title` for a present id is silently dropped**. The shard plane **heals absence** (missing row/bytes) but **cannot heal drift** (stale field on a present row). **[Corrected ground truth per task instruction (3): heals on INSERT, insert-or-skip. This is precisely why drift-healing is the CRDT plane's job — VERDICT coherence: SURVIVED division of labor.]**
- Freshly-replicated row lands with **neither** provenance marker (`dht_anchor_hash:None` at `mod.rs:4174`, `p2p_published_at` unwritten) → invisible to external reads until `drain_publish_queue`→`mark_published` republishes to Kad (`content_diesel.rs:1074-1088`, the sole writer of `p2p_published_at`).

---

## 4. Per-Layer Design

Each layer below carries a **Failure-Modes → Defenses** table (defense-in-depth) and a **Resiliency-Proofs** subsection. Aspirational defenses are marked `[SEQUENCED]` (a future item), never presented as present robustness (REQ-N3, honoring VERDICT L2 #4).

---

### 4.1 LAYER 1 — Automerge CRDT plane as authoritative convergent state

#### 4.1.1 Design

**Convergence contract (SURVIVED, keep as-is):** per-content doc `node:{id}` under `PROJECTION_NAMESPACE="elohim"`, converged head-diff/notary-free over libp2p+sled+Automerge. Merge is commutative/idempotent → dup/out-of-order/full-resend safe.

**The heal leg (net-new, REQ-F2):** add `spawn_docstore_heal_consumer`. After each `apply_changes` merge for `node:{id}`, read the doc's `projected_fields` and **field-merge UPSERT** into `content` via the real upsert (`content_diesel.rs:547-569`) — **NOT** `bulk_create_content`'s insert-or-skip. **Constraints that MUST hold (from verdicts):**

- **F5 — namespace pin.** The UPSERT targets `AppContext::default_lamad()` (`h_app_id="lamad"`), never the doc's `"elohim"` namespace. Mirror the compile-coupling: a `heal_namespace_matches_sync_timer` wire-contract test (twin of Task G4) fails the build on drift. **[MUST-CLOSE per VERDICT L1 R3.]**
- **N5 — no laundering.** The heal MUST NOT stamp `p2p_published_at` on a value received from an *unauthenticated* peer. Until Ed25519 author-signature verification lands on the sync path, the heal writes the **operational field only** and stamps the distinct `crdt_converged_at` marker (which admits only the *converged/unconfirmed* tier, never the notarized or published tiers). **[MUST-CLOSE per VERDICT L1 R2.]**
- **F4/OD-1 — empty-over-non-empty is forbidden.** Because `blobHash` is written as `""` when null (`projector.rs:70-72`) and Automerge resolves scalar conflicts by actor-id LWW (`sync/mod.rs:81-90`), a peer holding `""` can *win* the merge over a peer holding the real hash — converging a healthy peer *backward* to empty and re-introducing the 404. The heal rule is therefore **`write iff (local IS NULL OR local="") AND converged ≠ "" AND converged is well-formed`** — never propagate emptiness into a populated field. This is a *stopgap invariant*; the real fix is OD-1 (carry `blob_hash` under a notarized declared HEAD, not an LWW scalar). **[MUST-CLOSE per VERDICT L3 #1/#2, VERDICT coherence R3.]**

#### 4.1.2 Failure-Modes → Defenses (L1)

| Failure | Defense | Status |
|---|---|---|
| Namespace drift → silent inert | shared `PROJECTION_NAMESPACE` const + wire-contract test (both sync timer and heal consumer) | present (sync) + REQ (heal) |
| sled reset → empty DocStore | `backfill_content_docs` re-drives all SQL rows provenance-ungated (`projector.rs:162`); enable `ELOHIM_DOCSTORE_BACKFILL` on empty-DocStore detect | present but env-gated OFF `[SEQUENCED enable-on-detect]` |
| EventBus `Lagged` drops go-forward writes (`projector.rs:263`) | **backfill** re-reads SQL (NOT the heal leg — the doc is stale and was never sent to any peer, VERDICT L1 R4) | present (backfill), corrected attribution |
| Malformed peer change | non-32-byte heads filtered silently (`sync/mod.rs:113-125`) → safe full-resend; heal validates `projected_fields` shape, skips on parse-fail | present + REQ |
| Empty-over-non-empty LWW clobber | heal `write-iff-non-empty-and-well-formed` guard; **real fix = OD-1 notarized HEAD** | REQ (stopgap) + `[SEQUENCED OD-1]` |
| Unauthenticated peer injects arbitrary `blobHash` | Ed25519 author-signature verification on the sync path before any provenance stamp | `[SEQUENCED — REQ-N5, blocks provenance-stamping heal]` |
| ListDocuments limit 1000, no initiator paging (`p2p/mod.rs:6995`) | corpus >1000 tail never listed per round — scaling limit | `[SEQUENCED — add initiator paging]` |
| `sync_paused` auto-trip >100 pending (`p2p/mod.rs:2452`) | by-design backpressure; heal runs post-merge, not in drain | present |

#### 4.1.3 Resiliency-Proofs (L1)

- **P-L1-1 "converges with zero notary reachable":** extend G3 `doc_authored_on_a_converges_to_b` to assert **no conductor bridge registered** during the round. Che-provable (`GROUND[resiliency-proof-surface]§3`). *Replaces the simulated `sync_integration.rs` in-process exchange (REQ-N4).*
- **P-L1-2 "heals an existing stale/empty field":** seed a row with `blobHash=NULL`, converge a doc from peer B carrying a real, well-formed `blobHash`, assert (i) `content.blob_hash` now set, (ii) `crdt_converged_at` stamped, `dht_anchor_hash` and `p2p_published_at` still NULL, (iii) `lookup_slug_blob_hash` now resolves under the **converged tier** → SPA mounts. **RED if the reverse consumer is absent.** This is the `elohim.host/` 404 as a red-to-green. **[MUST assert `get_content`/serve-path, not `get_doc_field` — VERDICT L2 #1, REQ-N4.]**
- **P-L1-3 "empty never wins":** two peers, one with real `blobHash`, one with `""`; force B's actor-id to sort higher; assert the healthy peer's SQL row is NOT converged backward to `""`. **RED without the write-iff-non-empty guard. [MUST-CLOSE per VERDICT L3 #2.]**
- **P-L1-a2o:** promote `content-sync.feature` convergence from `@wip`; add `grandma-blobhash-heals-without-notary` mirroring `grandma-photos-survive-node-loss.feature`.

---

### 4.2 LAYER 2 — Holochain DHT as notary / controller (decoupled from convergence)

#### 4.2.1 Design

**What the notary asserts (three assertions, SURVIVED as intent):** (1) identity/signature over HEAD — the authoring `agent_cid` held standing at the declared reach, anchored by `dht_anchor_hash`; (2) reach authorization on class-A fields; (3) **HEAD-of-DAG selection as a DECLARED dependency.**

**Async overlay (REQ-F6):** serving depends only on L1/L3; notarization is a *late overlay* stamping `dht_anchor_hash` onto an already-serving row via the `ContentCommitted` signal path (`rea_projection.rs:694`) and the late-connect re-notarize wiring (`main.rs:1704-1729`). Convergence NEVER waits on it.

**MUST-CLOSE reframes carried from verdicts:**
- **The declared-HEAD selector does not exist and `upsert_with_anchor` contradicts it.** Before any "never recency" claim, implement a real declared-HEAD (lockfile/cid-pin) selector; today the anchor advances by signal-arrival recency (`content_diesel.rs:682-720`). **[VERDICT L2 #3, OD-1.]**
- **A distinct `crdt_converged_at` column is required** to distinguish converged from junk at the serving gate (§4.3). Relaxing the binary OR-gate without it serves ALL both-NULL rows indiscriminately, including shard-replicated and un-notarized-seed rows. **[VERDICT L2 #2.]**
- **Arc-incoherence and cross-signed-binding defenses are UNBUILT** — they are sequence items, not present defense-in-depth. **[VERDICT L2 #4.]**

#### 4.2.2 Failure-Modes → Defenses (L2)

| Failure | Defense | Status |
|---|---|---|
| Conductor down → notarized PATCH | `503` fail-loud on authority-mutating PATCH ONLY (`http.rs:4970`); reads + convergence + serving unaffected; late-connect re-notarize re-wires when registry populates (`main.rs:1704`) | present `[SURVIVED]` |
| Kad down | `put_record` fails, `mark_published` never runs, `p2p_published_at` stays NULL (`p2p/mod.rs:3600-3620`) — convergence-independent | present |
| Forged authority / self-asserted binding | never consume bindings for economic attribution until a cross-signed Ed25519 control proof replaces the structural `signer_is_known_agent` check; treat unstamped convergence as *content-trusted, authority-unconfirmed* | `[SEQUENCED — the real hole, blocks trust-consumption]` |
| Arc-incoherence (different DNA hash → different DHT → partition) | notary stamp records the DNA hash; consumer rejects a stamp minted under a foreign arc | `[SEQUENCED — UNBUILT, not present defense]` |
| Stale notarization (anchor at superseded HEAD) | HEAD is declared/notarized; a converged newer DAG head shows `trust: unconfirmed` until re-stamped | `[SEQUENCED — depends on OD-1 declared-HEAD]` |
| "DHT wins reverts a bad diesel write" — **stale for content** | **there is no content reconciler** (`controller.rs` has no `blob_hash` handling); precedence MUST be an explicit Doc-vs-Notary rule, not an assumed revert | `[MUST-CLOSE — VERDICT L3 #4, coherence UNCERTAIN]` |

#### 4.2.3 Resiliency-Proofs (L2)

- **P-L2-1 "notary down ⇒ state converges + serves":** extend the real two-node libp2p convergence proof to assert serve-path visibility of an **unstamped** (converged-tier) row — no conductor in the harness. Che-provable. **Assert `get_content(require_provenance=…)` returns the converged `blobHash` under the converged tier — currently impossible (REQ-N4, VERDICT L2 #1).**
- **P-L2-2 "notarization eventually stamps":** sweettest `two_agent_conductors()` + `await_consistency` proves `ContentCommitted` → `upsert_with_anchor` advances `dht_anchor_hash` on the pre-serving row (`GROUND[resiliency-proof-surface]§2`, `#[ignore]`→standing pipeline gate). **Assert the row was readable BEFORE the stamp and stamped AFTER** — the async overlay proven.
- **P-L2-3 "foreign-arc stamp rejected":** `[SEQUENCED]` — un-ignore/extend a sweettest to inject a stamp minted under a mismatched DNA hash; assert the consumer rejects it. Depends on the arc-hash-in-stamp item.

---

### 4.3 LAYER 3 — SQLite serving projection (read-optimized cache)

#### 4.3.1 Design

**Law restated:** L3 is a cache, never truth; losing it loses nothing; serving must not hard-depend on notarization.

**Tri-state visibility (REQ-F3, replaces the binary gate):** evolve `require_provenance` (binary marker) into `require_min_trust` (tiered), backed by a **new distinct column `crdt_converged_at`**:

| Marker present | Tier | Serving behavior |
|---|---|---|
| `dht_anchor_hash` | **notarized** (green) | full trust; admitted to economic/attribution reads |
| `p2p_published_at` | **published** (blue) | peer-attested custody; admitted to serving |
| `crdt_converged_at` only, others NULL | **converged/unconfirmed** (amber) | serve bytes; view stamps `trust:"unconfirmed"` |
| all NULL | invisible (unchanged) | local-stack seed gap |

External SPA mount admits **`converged` and up** (a functioning app beats a 404). Economic/attribution reads still require **`notarized`** — an unconfirmed row is NEVER consumed for authority/attribution (honors `GROUND[identity-authority]§3-4`; the AgentPeerBinding is still sentinel-signed).

**MUST-CLOSE reframes carried from verdicts:**
- **`blob_hash` IS class-A notarized** — the reverse projector writes a notarized field by definition, so the naive "reverse projector writes non-notarized tier only, can't clobber green" invariant is **self-contradictory** (`http.rs:10080`, comment "`blob_hash` is class-A notarized"). Resolution: the reverse projector writes `blob_hash` **only into the amber tier** and **only when the local value is NULL/empty** (never over a green-stamped populated value); precedence with the notary is the explicit rule of REQ-N6/OD-1, not an assumed tier-isolation. **[VERDICT L3 #3.]**
- **Amber is NOT actively reverted to green.** Because no content reconciler exists, a wrong/empty amber row persists until a fresh `ContentCommitted` signal fires — which on a conductor-less stack never comes. The amber→green upgrade is event-driven, not timer-driven. The write-iff-non-empty guard (P-L1-3) is therefore the only thing preventing a bad amber row from serving indefinitely. **[VERDICT L3 #4.]**

#### 4.3.2 Failure-Modes → Defenses (L3)

| Failure | Defense | Status |
|---|---|---|
| Row missing bytes (hash points at bytes that never landed) | heal-on-read `get_blob_or_heal` race-fetches from `peer_blob_inventory` (`http.rs:5622-5649`); `NotFound`/`FinalizeFailed` preserve 404 | present `[SURVIVED]` |
| Row missing provenance (both markers NULL) | **NOT** healed by heal-on-read (bytes only); healed by the amber tier + reverse projector stamping `crdt_converged_at` | REQ (new) |
| Stale row (shard insert-or-skip drift, `content_diesel.rs:443-446`) | reverse projector UPSERTs on changed serving field (`doc_matches`/`blobHash` force, `projector.rs:403`), never insert-or-skip | REQ |
| Reverse projector clobbers a green stamp | write `blob_hash` into amber **only when local NULL/empty**; explicit Doc-vs-Notary precedence (REQ-N6), NOT assumed tier-isolation | `[MUST-CLOSE — VERDICT L3 #3]` |
| Poisoned scope row (EprRouter-empties) | per-row `require_min_trust` filtering degrades per-row; drop malformed, serve siblings; never fail-closed collect + array-wrap | REQ-N7 |
| Projection lag / EventBus `Lagged` | reverse projector is a convergence consumer (idempotent, replayable from DocStore heads) → self-heals next round | REQ |
| Thundering-herd extraction | extraction cache with coalescing (`http.rs:5466-5566`); slug_index warm cache (`:5820-5836`) | present |

#### 4.3.3 Resiliency-Proofs (L3)

- **P-L3-1 "serves-converged-immediately":** author on peer A (no conductor), converge to B via the real two-node libp2p proof, assert B's SPA mount returns **200 amber** with `blob_hash` set and `trust:"unconfirmed"`, notary never reachable. **RED if the reverse projector absent.** **[Asserts serve-path, REQ-N4.]**
- **P-L3-2 "rebuilds-losslessly-from-L1":** delete the SQLite `content` table, run the reverse projector over the DocStore, assert byte-identical serving rows (sha256) for every `blob_hash`/slug; then stamp the notary, assert amber→green with **no row churn**. **[Gated on P-L1-3 empty-guard AND OD-1 — until those land, "lossless" faithfully reproduces the 404, VERDICT L3 #5.]**

---

## 5. Cross-Layer Coherence

### 5.1 Source-of-truth bifurcation (named precisely, corrected)

Three owners, and the corrected fact that they **overlap on `blob_hash`**:

- **DHT/notary owns** authority, provenance, HEAD-of-version-DAG selection. Stamps provenance columns.
- **CRDT doc owns** convergent operational field state (title, body, metadata) — **and, contested, `blob_hash`**.
- **SQLite projects** read-optimized serving, gated on trust-tier not truth.

**The contract-law correction (VERDICT coherence R1):** the naive "Doc→SQL writes operational columns ONLY; Notary→SQL writes provenance columns ONLY — never each other's" is **false**, because `blob_hash` is written by BOTH the notary path (`upsert_with_anchor`, `content_diesel.rs:716,730-735`) AND proposed as the CRDT-owned convergent field, AND `patch_needs_conductor` routes it through the conductor precisely because it is class-A notarized. **The clean column-partition does not hold.** Coherence therefore requires an explicit **arbiter** for `blob_hash`, resolved by OD-1:

- **Interim rule (Phase A):** Doc→SQL writes `blob_hash` into the **amber tier only, only when local NULL/empty, only well-formed non-empty converged value** (P-L1-3 guard). Notary→SQL (`upsert_with_anchor`) is **authoritative** and upgrades amber→green, overwriting the amber value with the notarized one (they should agree; if they diverge, notary wins and the divergence is logged). This is the explicit precedence REQ-N6 demands; it does NOT assume the (nonexistent) content reconciler reverts anything.

### 5.2 Handoff contracts (three edges)

1. **SQL→Doc** — EXISTS (`spawn_content_projection_listener`, `projector.rs:238`).
2. **Doc→SQL** — **MISSING, the whole bug.** Phase A adds the field-merge UPSERT (operational fields, amber tier, `crdt_converged_at` stamp, empty-guard, lamad namespace).
3. **Notary→SQL** — EXISTS (`upsert_with_anchor`), authoritative for `blob_hash` and the sole writer of `dht_anchor_hash`.

### 5.3 P2P-design-gate output (content convergent-state entity)

- **Category:** A (notarized), **bifurcated** — authority (who-authored-at-what-reach, HEAD selection) is notarized; the convergent *value* is CRDT-owned state the notary *witnesses*, not stores. **[OD-1 tension: `blob_hash` is itself a notarized class-A field, so the bifurcation is not clean — see §2.3.]**
- **DHT entry exists?** Yes — lamad Content (~73/100). No new entry type.
- **Identity:** CIDv1 (`bafyrei…`, dag-cbor/sha2-256) over the canonical entry — distinct from the ZIP `blob_hash` (`GROUND[sqlite-serving]§1`) and the `action_hash` provenance marker.
- **Version DAG:** CRDT should converge a DAG; **notary picks HEAD** as a declared selection. **[MUST-CLOSE: today the doc is a FLAT LWW scalar with no DAG and no HEAD pointer — VERDICT L1 R1. This is a net-new substrate item, not a property of the current code.]**
- **Coordinator fn:** `ContentCommitted` signal creates/advances the anchor. **Signal that projects it:** `rea_projection` → `upsert_with_anchor`.
- **Key constraint:** the bifurcation + the explicit `blob_hash` arbiter (§5.1).

### 5.4 Identity/authority coherence

`agent_cid` (`uhCAk…`) is the canonical join key across all layers; transport ids resolve via the `peer_transport_manifest` Category-C projection (`GROUND[identity-authority]§1`). The CRDT plane converges from any connected peer (safe — commutative merge), but **the notary stamp requires a real Ed25519 cross-signed binding before `dht_anchor_hash` is trusted for reach/authority** — the binding today is `STAGE1_SIGNATURE_SENTINEL` (`GROUND[identity-authority]§4`). The HTTP reach-enforcement gap (backlog `http-reach-enforcement-gap.md`) is **inherited and flagged, NOT widened**: Doc→SQL writes only operational fields the read-side reach gate already governs.

### 5.5 Shard-plane composition — DECISION: STAY SEPARATE

No merge (`GROUND[shard-replication]§5`, resilience-facings §12 decision 2026-06-27). **Shard heals *absence*** (missing row/bytes, insert-or-skip, `h_app_id="lamad"`); **CRDT heals *drift*** (changed field on a present row, `h_app_id="elohim"`). Shard's insert-or-skip is *precisely why* it cannot heal a stale `blobHash` (§3.4) — that is the CRDT plane's job. Merging would force the shard plane to upsert, re-opening the notarized-field-revert hazard the `503` gate protects. **[SURVIVED division of labor.]** Do NOT re-propose doc-sync as a resilience-facings §11 server fold — it is a client-composed sibling signal (determinism §5/§7 async-boundary reasons); a server fold would mask at-risk custody (§12).

---

### 5.6 Reach lifecycle through the edit→republish cycle (the CRDT ↔ reach-gate resolution)

**[architect, 2026-07-01 — a new design space]** The "Google-Docs-like" concurrent-edit nature of the CRDT plane resolves against the reach-gate epic via a **three-moment reach lifecycle**, keyed to the version DAG (§5.3). Reach is *inherited* between forks and *re-earned* at republish — never re-evaluated per change.

1. **Fork (edit begins) — reach is INHERITED, not re-evaluated. (REQ-F11)** When an agent opens an EPR for CRDT editing, the new working version (a DAG child of the current notarized HEAD) **inherits the origin snapshot's reach** — the reach of the last-published (green/notarized) version it forked from. The amber working copy is never reach-less; it carries the HEAD's reach for the duration of the edit.

2. **Concurrent edit — participation is reach + attestation gated. (REQ-F12)** The CRDT edit-state is open to exactly the cohort that held **reach + attestation to edit** that EPR on the origin snapshot — NOT to any peer on the wire. **This is the resolution of the REQ-N5 poisoning vector:** the "TLS handshake" of the sync path is *membership in the origin's reach+attestation edit-cohort*, enforced by `reach_authorization` (author-earn + receiver-preauth), not open gossip. Concurrent editors are the co-authorized — exactly as a shared Google Doc is open to its named collaborators, not the public. This **tightens C5**: author-authentication is proof-of-cohort-membership, not a bare Ed25519 sig on arbitrary peer input.

3. **Republish (agreement to republish) — the RE-NOTARIZATION checkpoint. (REQ-F13)** The *agreement to republish* is the single moment the CRDT edit-state is re-notarized. Atomically: (a) the SQL projection is **affirmed/updated** to the new version; (b) DHT notarization is **re-earned** — the new version's `action_hash` becomes the declared HEAD (`declare_content_head`, C1/C3); (c) reach is **re-certified** — any elohim reach-validations that must run, run HERE, at submit-for-republish, governing and re-certifying the new HEAD's reach. Republish IS the amber→green transition: the HTTPS handshake that re-issues the cert.

**Per-layer consequences:** L1 — a working version is a DAG child inheriting HEAD reach; convergence among the reach-cohort is the collaborative edit (amber = under-edit / converged-not-yet-republished). L2 — `declare_content_head` at republish is where reach re-validation + re-notarization happen atomically (the reach-gate enforcement point, NOT per-keystroke). L3 — an amber version serves under its INHERITED (origin) reach; only a republished (green) version serves under freshly-re-certified reach.

**OD-7 (new, reach-gate-epic decision):** the exact "agreement to republish" trigger — author-alone, a quorum of the edit-cohort, or a governance action — is unresolved. Sequences with C1/C3/C4/C5 (the republish/notary-overlay). Until decided, the deploy-producer (OD-2) is a *system-authored* republish (admin-gated), which is the trivial single-author case.

**Deferred sub-process (architect, 2026-07-01) — sub-publishing flows (cohort expansion, no full commons re-convergence).** The three-moment lifecycle above is complete and performant as the base; NONE of this is needed for resilience. A future sub-process layer can add **sub-publishing flows**: operations that *invite and authorize new stewards* into the edit-cohort **without forcing a full commons-level reach re-convergence** of what is already published on the most-exposed (commons-reach) surface. Cohort-membership growth is then a lightweight, scoped re-authorization — adding a contributor costs **O(the invite)**, not **O(the commons corpus)** — so collaboration scales without re-running commons-wide reach validation on every join. This composes ON the base (a sub-process under the §5.6 republish model, gated by the same `reach_authorization`), never a redesign of it. **DEFERRED — not in the base build; a named future design space, not a requirement.**

---

## 6. Sequenced Build Plan

**LAW-4 across all phases:** no layer brittle in-itself; convergence never hard-depends on notarization; HEAD stays declared/notarized never recency; coupling only after each layer is battle-ready and proven.

### Phase A — converge-heal leg (smallest fix for the elohim.host class; no new coupling)

**Scope:**
- Add the Doc→SQL field-merge UPSERT (handoff edge #2), **operational fields only**, driven from `apply_changes` (`sync/mod.rs:90`), re-projecting only changed serving-critical fields (`projector.rs:403`).
- **F5:** pin `AppContext::default_lamad()`; add `heal_namespace_matches_sync_timer` wire-contract test.
- **F4-stopgap / P-L1-3:** write-iff-`(local NULL/empty) AND (converged non-empty, well-formed)`.
- Add the `crdt_converged_at` column and the `require_min_trust` tri-state gate (amber admitted to SPA mount, notarized required for attribution).
- **N5 pre-req:** stamp `crdt_converged_at` (converged tier) — NOT `p2p_published_at` (published tier) — so unauthenticated-peer input is never laundered into peer-attested provenance.

**Acceptance gate:** a peer with NULL/empty `blobHash` on `elohim-host-landing`, after one 60s round from a peer holding a converged well-formed doc, serves the SPA — **no 404 at `http.rs:5601`**. **[VERDICT coherence R2 correction: this gate is met ONLY because `require_min_trust` now admits the amber tier; filling the operational field alone would NOT satisfy the old OR-gate. The tri-state gate is therefore IN Phase A, not deferred.]**

> ⚠ **Phase A is demonstrable only with a hand-seeded fixture until OD-2 resolves — see §9-R1.** On a *live* conductor-less mesh the cure is **inert**: every peer's doc carries `blobHash=""` (`projector.rs:70-72` `unwrap_or_default()`), because the only writer of the real hash is the DHT-gated PATCH that is down. There is nothing to converge. Phase A green in the two-node fixture test proves the *mechanism*; it does NOT by itself un-404 the live `elohim.host` until a non-notarized producer (OD-2) puts a real hash into at least one peer's L1. This is the single most important thing to understand before treating Phase A as "ships the fix."

**Resiliency proof:** P-L1-1, P-L1-2, P-L1-3, P-L3-1 — a **real** two-node libp2p heal test asserting **serve-path** (`get_content`) visibility. RED if Doc→SQL skipped, RED if empty-guard removed. Promote the SIMULATED `sync_integration.rs` to the real transport test (REQ-N4).

### Phase B — notary-overlay (async provenance stamping)

**Scope:**
- After convergence, async-stamp `dht_anchor_hash` when the notary confirms via the re-notarize path (`main.rs:1704`). Convergence NEVER waits on it.
- Require the real cross-signed Ed25519 binding (§5.4) before the stamp is trusted for reach/authority.
- Ed25519 author-signature verification on the **sync path** (REQ-N5) — this is the gate that lets the heal stamp `p2p_published_at` (published/blue tier) safely, upgrading amber rows whose author is verified.
- Explicit Doc-vs-Notary precedence for `blob_hash` (§5.1 interim rule → REQ-N6): notary authoritative, amber never over green.

**Acceptance gate:** a converged amber row serves via `crdt_converged_at`, later gains `dht_anchor_hash` (green) with **no re-converge and no row churn**; notary-down still serves the amber row (trust-unconfirmed, not state-lost).

**Resiliency proof:** P-L2-1 (notary-down converges+serves, Che), P-L2-2 (sweettest: readable-before-stamp, stamped-after — un-ignore the DHT sweettests as a standing pipeline gate, `GROUND[resiliency-proof-surface]§4`), P-L3-2 (lossless rebuild).

### Phase C — battle-ready tight-coupling (ONLY after A+B soak-green)

**Scope:**
- **OD-1 resolution:** couple HEAD-selection to the notarized declared DAG HEAD — add the multi-version doc structure and the `resolve_head(id)→declared_head_action_hash` selector (net-new substrate, VERDICT L1 R1). Replace the LWW scalar `blob_hash` with a HEAD-pointer-carried value so convergence stops being recency.
- Arc-hash-in-stamp + foreign-arc rejection (P-L2-3).
- Close the iroh sync-round-driver gap (backlog `iroh-sync-round-driver-gap.md`).
- `/sync` auth posture (inherits `http-reach-enforcement-gap`; operator review at merge).
- Corpus back-fill of already-seeded rows (separate idempotent gated migration; enable `ELOHIM_DOCSTORE_BACKFILL`).

**Acceptance gate:** each layer's standalone proof green across a **full soak** before any coupling lands; chaos-peer-churn + a multi-doorway federation harness closing the elohim.host RED (T3/T4, `federation-deploy`/`blob-replication`/`epr-projection-fallback`).

**Resiliency proof:** chaos-peer-churn, multi-doorway federation harness, un-ignored DHT sweettests as standing gates, the OD-1 declared-HEAD selector test (a stale higher-clock peer does NOT overwrite the declared HEAD).

---

## 7. Open Decisions / Paths-Not-Taken / Anti-Patterns Guarded

### 7.1 Open decisions

- **OD-1 (the hardest): `blob_hash` is class-A notarized AND the field the CRDT wants to own.** Options: (a) reclassify — the CRDT owns only *non-notarized* operational fields, and `blob_hash` heals via a *different* non-notarized producer that populates the real hash at deploy time (decoupling field-population from the PATCH) — VERDICT L3 #1's suggested close; (b) carry `blob_hash` under a notarized declared-HEAD pointer that the CRDT converges as a DAG of versions, not an LWW scalar.
  **✅ RESOLVED 2026-07-01 (architect): option (b) — FULL 1c NOW.** The version-DAG + notarized declared-HEAD substrate moves from Phase C **into the core build** (see re-sequenced §6). `blob_hash` is carried under a notarized declared-HEAD pointer the CRDT converges as a DAG of versions, never an LWW scalar. The interim empty-guard/amber-only stopgap is SUPERSEDED by the real HEAD selector (it may still land first as a scaffold, but the target is the DAG, not the stopgap).
- **OD-2: where does the "real `blob_hash`" originate on a conductor-less mesh?** If every peer's doc carries `""` (because the PATCH never fired), there is nothing to converge (VERDICT L3 #1). A non-notarized producer that writes `blob_hash` into SQL/DocStore at deploy/build time is likely required.
  **✅ RESOLVED 2026-07-01 (architect): the deploy-time non-notarized producer.** `blob_hash` origination is decoupled from the notarized PATCH — a build/deploy-time producer writes it into SQL/DocStore so the mesh has a real hash to converge during a notary outage. This is a **Phase-A prerequisite** (it originates the value the HEAD-DAG then carries + the CRDT converges). It also means the elohim.host live 404 is fixable during a notary outage — the origination that §9-R1 said was missing.
- **OD-3: sync-path auth scope.** Full Ed25519 verification (REQ-N5) blocks the safe published-tier heal. Interim: converged (amber) tier only, no published stamp. Is amber-only acceptable for the elohim.host cure in the interim? (Yes per §6 Phase A — amber is SPA-admitted.)
- **OD-4: `/sync` GET auth posture** (Phase C; operator review at merge).

### 7.2 Paths not taken

- **Merging the shard and CRDT planes** — rejected (§5.5): forces shard to upsert, re-opens the notarized-field-revert hazard.
- **Folding doc-sync into the custody resilience verdict** (resilience-facings §11 server fold) — rejected: would mask at-risk custody; doc-sync is a client-composed sibling signal (§12, determinism §5/§7).
- **Relaxing the binary OR-gate directly** — rejected (VERDICT L2 #2): serves un-vetted junk indiscriminately; the distinct `crdt_converged_at` column is mandatory.
- **Stamping `p2p_published_at` on unauthenticated converged values** — rejected (VERDICT L1 R2, REQ-N5): a poisoning vector strictly worse than a diesel hand-write.

### 7.3 Anti-patterns guarded

- **Recency-overwrites-declared-HEAD** (p2p-design-gate): the current doc is an LWW scalar (VERDICT L1 R1/L3 #2/coherence R3). Guarded by the empty-guard (interim) and OD-1 declared-HEAD (final). Convergence ≠ binding-selection.
- **"PROVEN" before the code exists** (VERDICT L2 #1): every serve-path proof MUST assert `get_content`, not `get_doc_field`; the DocStore→SQL consumer is net-new.
- **Assuming a reconciler reverts bad writes** (VERDICT L3 #4, coherence UNCERTAIN): there is NO content reconciler in `controller.rs`; precedence is explicit (REQ-N6), never assumed.
- **Written-under-X-read-under-Y dormancy** (VERDICT L1 R3): heal pins `default_lamad`.
- **Fail-closed collect + array-wrap → whole-router-empty** (MEMORY EprRouter): per-row degradation (REQ-N7).
- **Converging emptiness** (VERDICT L3 #1): `unwrap_or_default()` writes `""`; never propagate empty into a populated field.

---

## 8. Composition Notes

**Composes with (existing docs/plans):**
- The **Automerge content-sync lighting plan** (§Execution Outcome, §Global Constraints, Tasks G3/G4) — this spec picks up its explicit deferrals: iroh sync-round driver gap, corpus back-fill, `/sync` security posture, dev-serve wasm-bundler-entry gap, G7g shem cross-tenant proof, "N of M converged" aggregate (needs `StreamTracker::StreamPosition`, currently `#[allow(dead_code)]`).
- **resilience-facings §11–12** (decision 2026-06-27) — inherits the plane-separation decision and the "doc-sync is a client sibling, not a server fold" ruling; inherits the facings substrate (pure fold + `elohim-facings` diesel-free crate split).
- **`2026-06-15-coherent-transport-identity-resolver-design.md`** — the transport-id→`agent_cid` resolver this spec's Phase B depends on (blocked on the same cross-signed-binding item).
- **The seam-map concern-routing atlas §3.10/§3.13** — peer-hoster dataplane = durable availability by peerId, confidentiality is its own plane (notarized ≠ encrypted).
- **The versioned-HEAD-is-declared-dependency policy seed** (MEMORY `project_versioned_entity_head_is_declared_dependency`) — LAW-3 and OD-1.
- **MEMORY `project_automerge_content_sync_plane_lit`** — the producer LIT state; this spec adds the missing consumer.

**What the lighting plan deferred that this picks up (explicit):**
1. iroh sync-round driver (Phase C).
2. Corpus back-fill of seeded rows (Phase C, gated migration).
3. `/sync` auth posture (Phase C, operator review).
4. The "N of M converged" aggregate — noted as needing net-new substrate (`StreamTracker::StreamPosition` → new route); NOT in scope, flagged.

**Key files (implementation surface):**
`elohim/elohim-storage/src/sync/{mod,projector,doc_store,stream}.rs` · `src/p2p/mod.rs` · `src/db/content_diesel.rs` · `src/http.rs` (5449-5843, 4942-4974, 10067-10082) · `src/sync/rea_projection.rs` · `src/reconcile/controller.rs` · `src/main.rs:2619-2698,1704-1729` · `tests/sync_integration.rs` (simulated → promote) · `elohim/holochain/tests/sweettest/src/tests/{rea_commitment_replication,replicates_commons_round_trip_test,recovery_m3}.rs` · `genesis/a2o/features/{resilience,dataplane,federation}/`.

---

## 9. Completeness-critique refinements (folded — tracked, not dumped)

A completeness critic ran over the synthesized draft; each item below is a concrete tightening carried as a spec-grade requirement/decision. None is optional polish — several change what "shippable" means.

- **§9-R1 — Phase A is inert on a live conductor-less mesh (the big one).** Phase A's acceptance gate assumes "a peer holding a converged well-formed doc," but §3.1 establishes every conductor-less peer carries `blobHash=""`. **Requirement:** Phase A is demonstrable only with a hand-seeded fixture until **OD-2** resolves the real-hash origin; on a live mesh the cure is inert. Do NOT report Phase A as un-404'ing production without OD-2. (Reflected inline at the Phase A gate.)
- **§9-R2 — Two divergent NON-empty hashes is an unguarded Phase-A hazard.** The empty-guard (P-L1-3) stops empty-over-non-empty only. Two peers with different well-formed `blob_hash` for one id → actor-id LWW serves the wrong amber row, unguarded until OD-1. **Add:** L1 failure-row + proof **P-L1-4** (RED when a stale higher-actor-id peer's divergent hash wins). Do not defer the whole divergent case to Phase C — the *empty* case is guarded in A, the *divergent-nonempty* case is an explicit Phase-A known-gap escalated to OD-1.
- **§9-R3 — Name the `require_min_trust` call-site as a MUST (REQ-F7).** Coherence R2 hinges on `lookup_slug_blob_hash` → `list_content(require_provenance=true)` (`http.rs:5811`, `content_diesel.rs:171-177`) being migrated to `require_min_trust` admitting amber. **REQ-F7:** that exact call-site migration is a Phase-A MUST; without it the `:5601` gate is unmet even with the field healed.
- **§9-R4 — Revocation must dominate convergence (REQ-F8 + OD-6).** No tombstone/revoke convergence story exists; a converged doc can resurrect notary-revoked content (`controller.rs` has a RevocationAttestation path for other entities, none for Content, §3.2). **REQ-F8:** a notary revocation MUST dominate any converged value (never resurrect). **OD-6:** revocation-convergence mechanism (tombstone doc vs. notary-gated serving suppression) — architect decision.
- **§9-R5 — Confidentiality is unaddressed (OD-5).** The sync path pulls full doc *body* unauthenticated (§3.1); REQ-N5 covers integrity (author-sig) not confidentiality. The atlas flags "notarized ≠ encrypted" (§3.13). **OD-5:** scope a confidentiality requirement for private-reach content on the sync path, or explicitly bound the CRDT plane to public-reach content only (interim).
- **§9-R6 — Amber admission must be conditioned on byte availability (REQ-F9).** A row can pass `require_min_trust=amber` yet `get_blob_or_heal` finds no bytes → SPA mounts, content 404s downstream. **REQ-F9:** amber admission degrades gracefully with byte availability (serve only when bytes are local or peer-fetchable; else a distinct "converging, bytes pending" state, never a silent broken mount).
- **§9-R7 — Observability + SLO (REQ-N9).** Battle-ready needs metrics on the amber (converged-but-never-notarized) backlog, heal rate, and convergence lag on the existing storage `/metrics` surface (see `project_storage_metrics_surface_and_leak_verdict`). **REQ-N9:** emit `crdt_amber_backlog`, `crdt_heal_rate`, `crdt_convergence_lag` + an SLO/alert on amber-backlog growth (an amber row that never greens is a stuck notary — the ethosengine class, surfaced not silent).
- **§9-R8 — `crdt_converged_at` migration mechanics.** Add the diesel migration + down-migration; heed the timestamp-collision trap (MEMORY `feedback_diesel_migration_timestamp_collision` — same `YYYY-MM-DD-HHMMSS` collapses under `embed_migrations!`).
- **§9-R9 — REQ-N7 needs a proof with teeth (P-L3-3).** Add a poisoned-amber-row test asserting the router serves siblings (per-row degradation), RED under fail-closed collect.
- **§9-R10 — L3 failure table is logical-only.** Add physical failure rows: SQLite corruption / disk-full / partial write → defense is rebuild-from-L1 (P-L3-2) + a WAL/integrity-check on open; these are LAW-1 cases (losing the cache loses nothing) and must appear in the §4.3.2 table.

---

*End of spec. The decoupling thesis (convergence is genuinely conductor-free) survived adversarial review; every load-bearing cure/proof that rested on the unbuilt DocStore→SQL consumer, the nonexistent declared-HEAD selector, the misattributed "DHT-wins" revert, or the empty/divergent-hash convergence hazards has been converted into a MUST-CLOSE requirement or an Open Decision, and sequenced so that no "PROVEN" label precedes the code that earns it. The single most load-bearing caveat: **Phase A proves the mechanism but does not un-404 the live host until OD-2 gives the mesh a real hash to converge (§9-R1).***