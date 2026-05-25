# Doorway Access-Tier Patterns

**Created:** 2026-05-23 alongside the dual-doorway landing-page shakeout.
**Status:** Pattern catalog. Names three patterns (A / B / Recovery) and one open question (anon-hint). Implementation is deferred to subsequent shifts.
**Audience:** rust-architect (warm-stream, hosting-agreement reads), angular-architect (UI surfaces for each tier), red-team (cache-scoping attack surface).

---

## Why this spec exists

The doorway sits between two truth layers:

1. **The peer-native P2P substrate** — where content lives content-addressed across peers, where stewards are first-class agents with conductor cells, where reach gating enforces who can read what.
2. **The web2 projection cache** — where MongoDB holds a denormalized view of content that the doorway has agreed to host, served over HTTPS to browsers that have no conductor, no DHT, no peer identity.

The shakeout work surfaced that the doorway's current behavior is partially honest about this boundary (the reach gate works, JWT auth works) and partially shaped by the substrate gap (content rows don't replicate peer-to-peer, so warm-stream's last-write-wins behavior randomly serves stale data). The user clarified the intended access model — three tiers, each with distinct facilitation by the peer-native runtime. This spec names the tiers, names the patterns that serve each, and surfaces the gaps that block the next sprint.

This is **descriptive**, not prescriptive. The next `/shift` picks one pattern to ship first based on the operator's call.

---

## Three access tiers

The doorway-mediated access surface is partitioned by who the requester is and how their identity is established.

### Tier 1 — Anonymous visitor

A browser hits `alpha.elohim.host` or `elohim.host` with no `Authorization` header. The doorway has no notion of the visitor's identity. The visitor sees only what the doorway is contractually agreed to host *and* what is reach-permitted for the public commons.

**What anon should see:**
- **Hosted content at commons reach.** Fully renderable from the projection cache. The landing page itself, public learning paths, public collective profiles.
- **Hints of unhosted edge content.** Content the doorway does *not* have a hosting agreement for — but knows exists via EPR-link metadata — surfaces as a hint (teaser title, link count, presence indicator) without the loadable payload. Navigation into the hint surface is gated; the anon visitor cannot load the content from a peer because they have no peer-native identity to authenticate the load.

**What anon must NOT see:**
- Content above commons reach, regardless of hosting status.
- Authenticated views (relationship-scoped, household-scoped, qahal-internal).
- Peer-private data leaked by cache-key collision with an authenticated requester.

**Where the gate lives:** `doorway/doorway-service/src/projection/app_auth.rs` — reads the `reach` field from cached content; allows when `reach == "commons"`, denies otherwise. This is the gate today. **It does NOT yet handle the hint-without-load case** (see Open question below).

### Tier 2 — Doorway-hosted user

A browser hits a doorway hostname with a JWT in the `Authorization` header. The doorway has a record of this user — typically because the user registered through the doorway's account flow and gets their conductor cell hosted by the doorway operator (the doorway's home steward). The doorway authenticates the JWT, resolves the `agent_pub_key`, and serves projection-cache views scoped to this user's relationships.

**What hosted users see:**
- Everything anon sees, plus reach-permitted content for their visibility (local, invited, peer-network).
- Their own household, qahal memberships, relationships, stewardship allocations.
- Write surfaces — comments, attestations, stewardship contributions — gated by reach and by the qahal/relationship structures the user participates in.

**Where the gate lives:** Same `app_auth.rs` but with `authenticated=true` and `agent_pub_key` populated. The doorway's JWT contract is in `doorway/doorway-service/src/routes/auth_routes.rs` and `doorway/doorway-service/RECOVERY-PROTOCOL.md`.

### Tier 3 — Hosted steward via web (recovery or remote access)

A peer-native steward — someone whose conductor cell normally runs on their own device — accesses the protocol through the doorway because their device is offline (recovery scenario), they are traveling and don't have their device, or they are using a borrowed browser. **The doorway facilitates and validates this access against the steward's peer-native identity.** The doorway does NOT host their conductor cell; it proxies their reads and writes to whichever peer holds it.

**The flow:**
1. Steward authenticates against the doorway using a recovery credential (Shamir share assembled from their intimate circle, or a single-use recovery token, or a webauthn passkey bound to their agent_pub_key — see `genesis/docs/superpowers/specs/2026-04-xx-recovery-protocol-design.md` and `socially_derived_security` memory).
2. Doorway resolves the steward's home conductor peer via DHT (which peer agreed to host their cell, or which peers replicate it).
3. Doorway proxies each read/write request through the home conductor cell, **signing the request with the steward's verified-at-recovery key** so the conductor returns the steward's full reach scope, not the doorway's.
4. Results travel back through the doorway's projection-cache layer, but the cache key includes the steward's agent_id (NOT just `{dna}:{type}:{id}`) so they don't leak into anon or other-user views.

**What this gives the steward:**
- Full reach scope as if they were on their own device.
- Write capabilities (gated by the conductor's reach validation, not the doorway's).
- Recovery operations (key rotation, device re-pairing) where the doorway acts purely as transport.

**What this distinguishes from Tier 2:**
- The cell is NOT hosted by the doorway. The doorway is transport-only.
- The steward owns their conductor's reach. The doorway has no override.
- The recovery surface is a peer-native primitive; the doorway implements the *web access* path for it.

**Where this lives today:** Mostly vision. `RECOVERY-PROTOCOL.md` describes the social-recovery flows. The conductor-proxy code path is **not implemented**. This is what Pattern Recovery (below) ships.

---

## Pattern A — Content-row P2P sync (substrate gap fix)

**Problem statement:** The Diesel `content` SQL table — backing `elohim/elohim-storage/src/cache_stream.rs` and consumed by doorway's warm-stream — does NOT replicate peer-to-peer. When the CI pipeline PATCHes `matthew`'s `elohim-host-landing.blobHash`, only `matthew` has the updated row. Warm-stream then fans across `matthew + 13 other peers` (sequentially per the actual code; not "indiscriminately" as an earlier handoff claimed). The stale-data peers are not unhealthy from their own perspective; they have no signal that their content row is stale. Last-write-wins at the doorway means the projection cache randomly serves stale data.

**The right shape:** Content rows are projections of ContentEntry on the DHT (or a libp2p sync stream, depending on what scope the project_three_layer_truth_model classifies content metadata as). Either DHT-notarized or libp2p-data-ops, **not** per-peer SQLite islands.

**Integration points:**
- `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md` — the blob-replication spec. Extend it to cover content metadata rows, OR write a sibling spec for metadata sync.
- `elohim/elohim-storage/src/db/cache_queries.rs` — where the doorway reads cacheable content; should observe a sync stream rather than poll per-peer.
- `project_inventory_exchange_not_byte_replication` memory — content-row sync is gossip+metadata; it does NOT need to ride the blob-replication path. Cheaper substrate.
- `project_three_layer_truth_model` memory — decide which layer owns content metadata. Likely libp2p (cheap, ops-shape) not DHT (expensive, notary-shape).

**Workaround in place today:** CI `stageSpaBlob` fans out the PATCH across all peers (Layer-A bypass at the CI level). Linear in peer count; fragile but functional. Acceptable as a stopgap until Pattern A lands.

**Acceptance signal:** Push a content-row change on one peer; observe it propagate to the other 13 within bounded gossip time, without CI fan-out, without a warm-stream restart.

---

## Pattern B — Hosting-agreement-aware warm-stream

**Problem statement:** Doorway's warm-stream reads peer URLs from `STORAGE_URLS` env (a CSV of peers in the env). It does NOT consult the DHT for which peers have agreed to host which content. As a result, it fans across every peer in the env regardless of whether that peer is actually responsible for serving the content being warmed.

**The substrate is already in place:** The `operate-doorway` REA Commitment exists.
- Schema: `elohim/sdk/schemas/v1/objects/operator-classification.schema.json` — fields `action: "operate-doorway"`, `capabilities[]`, `scopes[]`, `successionRole`.
- DHT entry type: existing `Commitment` (no new entry types needed).
- Storage read path: `elohim/elohim-storage/src/db/rea_commitments.rs::find_active_operator_binding(agent_id, doorway_id)` — already returns the active commitment for a given (agent, doorway) pair.

**What's missing:** The doorway warm-stream doesn't call any of this. It loops `for storage_url in &storage_urls` (sequential per-peer, with backoff, per `doorway/doorway-service/src/projection/warm_stream.rs:288`) and stops when at least one peer projects content. Hosting agreements are not consulted.

**The right shape:**
1. On warm-stream startup, doorway reads its own `operate-doorway` commitments from the DHT — which scopes it hosts (e.g. `doorway:alpha-elohim-host`).
2. For each scope, doorway queries the DHT for which peers have agreed to host content under that scope.
3. Warm-stream consults those peers in priority order (the operator's own storage first if listed, then deputies, then fallback peers). Skip peers that have no commitment.

**Layer C concern (load-balancing):** Even with hosting-agreement awareness, picking ONE peer per content unit (rather than warming from all qualified peers simultaneously) needs a capacity-aware selector. The system_metrics projection (`project_node_metrics_vs_hub_aggregation_boundary`) provides the inputs. This is a refinement on top of B, not a separate ship-on-its-own.

**Acceptance signal:** Doorway warm-stream startup logs lines like "consulting peer X for scope doorway:alpha-elohim-host per commitment SHA Y"; peers not listed in any commitment are not consulted.

---

## Pattern Recovery — Steward web access via conductor proxy

**Problem statement:** A peer-native steward needs to access their full reach scope through the doorway when their device is offline. The doorway-hosted-user path (Tier 2) doesn't apply — they don't have a doorway-hosted cell; they have their own cell on their own peer. The current code has no shape for "doorway authenticates a peer-native identity and proxies their conductor calls."

**The right shape (sketch — implementation work in next sprint):**
1. **Recovery authentication.** Doorway endpoint `/recovery/web-session` accepts a recovery credential (the recovery primitive is gospel; the web-binding is what's new). Issues a session that carries the steward's `agent_pub_key` + signing key material derived from the recovery credential. Session is short-lived and re-authenticates against fresh recovery proofs on extension.
2. **Conductor resolution.** Given the steward's `agent_pub_key`, doorway resolves which peer(s) hold their conductor cell via DHT (cell-replication metadata) or via the `operate-doorway` commitment graph (the steward's home steward may operate a doorway that hosts deputies).
3. **Request proxy.** Each read/write the steward issues is signed by the session key, sent to the resolved conductor peer over P2P, and the response is returned to the browser. The doorway is transport — no validation of reach happens at the doorway layer.
4. **Cache scoping.** Cache entries for proxied requests are keyed by `{dna}:{type}:{id}:{agent_id}` to prevent leakage into anon or other-user views.

**Distinct from Tier 2:**
- Tier 2 doorway: the doorway hosts the cell, validates reach itself, may modify (with consent) which cells it serves.
- Tier 3 (Recovery): the doorway does NOT host the cell; it is a transport gateway with no authority over the steward's data.

**The graduated-recovery authority memory:** `project_graduated_recovery_authority` — intimate circle → qahal → global witness; absolute lockout is a failure. The web-binding flow must respect this gradient.

**Acceptance signal:** A steward whose device is unreachable can log in through `alpha.elohim.host/recovery/web`, see their household feed (private reach), post a comment (write), and the comment lands in the DHT as authored by their `agent_pub_key`, NOT by the doorway operator's.

---

## Open question — Anonymous-visitor hint without load

**Status:** Not implemented. No code path exists for it today. Flagged for a later sprint.

**The vision:** An anon visitor browsing `alpha.elohim.host` lands on the landing page. The page shows hints of activity in the wider network — qahals forming, content being authored, conversations happening — without exposing the full payload of any of it. The hints are *invitations* to participate: "12 households are discussing reciprocity; sign in to read", "a learning path on water stewardship was authored 3 days ago — preview available with login", etc.

**Why this is hard:**
1. **What's a hint?** The reach gate is binary today (commons → allow, anything higher → deny). Hint needs a third option: metadata-only response, with the renderer choosing to show a teaser surface instead of the full content.
2. **What does the doorway have access to?** Edge content (content NOT under the doorway's hosting agreement) doesn't live in the projection cache. The doorway can only hint at content it has *some* signal about — typically via EPR-link metadata or DHT-announced presence indicators. Need a "hint surface" projection separate from the full-content projection.
3. **How does navigation work?** Clicking a hint surfaces a "log in to continue" prompt. Anonymous visitors cannot navigate into peer-native loading. The hint *exists* but the *load* is gated.

**Why this depends on B + Recovery:** The hint surface is naturally derived from the hosting-agreement graph (Pattern B knows what the doorway hosts vs what it knows about); the load gating is naturally a recovery-or-account-creation prompt (Pattern Recovery's web-session flow). Anon-hint shipping ahead of B and Recovery would either duplicate that infrastructure or ship a degraded UX.

**Decision for this spec:** Surface as a known design space. Do not ship in the next sprint. Revisit after B and Recovery have proven the underlying mechanisms.

---

## Pattern Z — Substrate-correct EPR Head republish (the foundation under A / B / Recovery)

**Surfaced by:** the 2026-05-24 shakeout. App #1464 mechanically succeeded — blob uploaded via `PUT /admin/seed/blob`, content rows PATCHed, all verifies green — yet `alpha.elohim.host` still served stale content. The new blob hash was visible in the SQLite content row (`GET /db/content/elohim-host-landing` returned the fresh CID) but doorway's `/apps/{slug}/index.html` route resolved through `app_file_cache.slug_index` which reads from MongoDB. MongoDB had the stale hash. The PATCH had updated SQLite *only*. There was no propagation.

The user named the diagnosis: *"a blob update should trigger a response from the EPR — it should recognize that something it's attesting to changed."* That is the substrate-correct framing.

### The two anti-patterns this names

**Anti-pattern Z.1 — `PATCH /db/content/{slug}` as a deploy primitive.**
`elohim/elohim-storage/src/http.rs:3810` registers `PATCH /db/content/{id}` as an authenticated SQLite mutation. The handler calls `ContentService::update` which writes to the diesel content table and returns the new view. It does NOT republish the EPR Head, does NOT emit a projection signal, does NOT advertise via Kad, does NOT touch the graph projection. The SQLite row — which is supposed to be a *projection* of substrate truth — is being treated as the authority. Every downstream cache (doorway MongoDB, doorway in-memory slug index, doorway extraction cache) silently diverges from it.

**Anti-pattern Z.2 — `/db/*` hard-coded short-circuit in doorway.**
`doorway/doorway-service/src/server/http.rs:1635` matches `(_, p) if p.starts_with("/db/")` and dispatches through `routes/db.rs::handle_db_request` — a 175-line proxy file that hardcodes which HTTP methods to forward (initially missed PATCH; fix landed in commit `7dcefeacd` 2026-05-24 23:03 UTC). Per `doorway/CLAUDE.md`: *"We deleted 13 identical proxy files that violated this rule. They must never come back."* This is the 14th instance. The route registry (`route_registry.rs`) already knows how to dispatch any method storage's manifest declares — through `forward_to_storage`, which supports GET/POST/PUT/PATCH/DELETE/HEAD. The short-circuit is dead-code shape masquerading as live code.

### The canonical primitive that already exists

`PUT /api/v1/epr/{cid}` (`elohim/elohim-storage/src/api/epr.rs:484`, dispatched via `epr.rs:158`). What it does:

1. **Validates content-addressed contract:** path CID must equal envelope CID.
2. **Rehydrates the full EPR envelope** — kind, reach, coupling, claims, supersedes, Ed25519 signature.
3. **Calls `FederatedEprStore::put`** — diesel persistence + Kad `StartProviding` (P2P advertisement that *this peer holds the bytes at this CID*).
4. **Graph projection (feature-gated `graph-native`)** — projects the new EprHead into CozoDB, writes SUPERSEDES edges if this atom supersedes a predecessor (the substrate's "this *replaces* that" primitive).
5. **Fan-out (Phase 3.5)** — FeedbackSignal propagation through the back-prop pathway.

That is the substrate's correct "republish this content with a new attestation" primitive. It is content-addressed, signed, P2P-advertised, graph-projected, and signal-emitting. Everything PATCH was trying to fake.

### The shape of the bridge (this sprint or next)

Migrating `stageSpaBlob` to `PUT /api/v1/epr/{cid}` is the destination but it has prerequisites: a deploy signing key, JSON envelope construction, CID computation. Those need a sprint of their own (see [Plan stageSpaBlob migration to PUT /api/v1/epr/{cid}](task #14)).

Until that migration lands, the minimum substrate-correct behavior is: **`ContentService::update` emits a projection signal on every PATCH**, so doorway's subscriber refreshes the MongoDB projection and the `slug_index` picks up the new hash automatically — eliminating the manual `/admin/cache/clear/{slug}` + `/admin/cache/warm` dance that 2026-05-24 shakeout had to perform by hand. This is **not** substrate-correct — it's a bridge that keeps the projection caches honest while the operational PATCH path still exists. It does not republish a new EprHead, does not gossip via Kad, does not write a SUPERSEDES edge. Future signal subscribers (P2P peers receiving the projection event) only know that *something changed*, not what the new attestation says. That's the cost of the bridge.

### What ships in this round of the long-term fix

| Step | Lands where | Status |
|---|---|---|
| Z.A — Document Pattern Z (this section) | this spec | done with this commit |
| Z.B — Doorway subscribes to storage's `/api/v1/events` SSE | `doorway/doorway-service/src/projection/storage_events_subscriber.rs` (new) + `main.rs` wiring | in progress; storage side already complete (see discovery note) |
| Z.C — Remove `/db/*` short-circuit; `routes/db.rs` deleted | `doorway/doorway-service/src/{server/http.rs, routes/db.rs, routes/mod.rs}` | done with this commit |
| Z.D — Plan stageSpaBlob → `PUT /api/v1/epr/{cid}` | dedicated spec at `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` | deferred to next sprint; design only |
| Z.E — Deprecate `PATCH /db/content/{slug}` | `elohim-storage/src/http.rs`, route registry | blocked by Z.D landing |

**Discovery note on Z.B (added 2026-05-25 mid-thread):** Storage already emits `StorageEvent::ContentUpdated { id }` from `ContentService::update` (`elohim-storage/src/services/content_service.rs:203`) and exposes it as `event: content.updated` on `GET /api/v1/events` (`elohim-storage/src/sse.rs:49`). The real gap is doorway-side — no subscriber exists. `doorway/doorway-service/src/projection/subscriber.rs` listens to the **conductor's WebSocket** (DNA post_commit signals), NOT to storage's HTTP SSE event bus. `doorway/doorway-service/src/projection/warm_stream.rs` consumes `cache.*` events ONCE at startup from `/api/v1/cache/stream` but does NOT keep listening for live updates. Z.B as originally scoped ("emit a signal on PATCH") is already shipped; what's missing is a long-running doorway-side subscriber that translates `content.updated` → `app_file_cache.refresh_app(slug)` + projection store invalidation. This is a new file plus `main.rs` background-task wiring, ~150 LoC. Reframing in the table above.

### Why the order matters

Z.B + Z.C are the bridge that closes the visible delivery gap (alpha.elohim.host stops serving stale content after PATCH). Z.D is the migration that makes the bridge unnecessary. Z.E is the cleanup that lets us delete the bridge.

Inverting this order would break shakeout delivery: if we ship Z.E first (delete PATCH /db/content), stageSpaBlob has nothing to call. If we ship Z.D before Z.B, we still need the signal-on-PATCH because other PATCH callers (the avodah API, UI inline edits) will exist for some time.

### Out of scope (named here so they don't drift)

- **The seed-script PLACEHOLDER pattern.** `sha256-PLACEHOLDER_REPLACED_BY_SEED_SCRIPT` literal sentinels in seed JSON are themselves an anti-pattern (storage-side state coupled to a sed-replace at boot). Pattern Z's bridge does not address this; the seed pipeline replaces the seed-time write entirely.
- **EPR Head republish authority and identity.** Who signs the deploy-time republish — Jenkins, a coordinator zome, a steward agent? That is the central question Z.D must answer. Today's PATCH bypasses the question by claiming admin authority via an X-API-Key. The substrate-correct answer is "an Ed25519 keypair owned by a named agent" but *which* named agent (the operator? a deploy service account? the EPR's original author?) is what the spec needs to resolve.
- **Two-doorway federation.** elohim.host (alpha-b federation peer) reading content from alpha.elohim.host depends on Pattern A (content-row P2P sync). Pattern Z does not solve federation — it makes the source-of-truth update propagate to ONE doorway's projection caches. Federation is sibling-but-orthogonal.

---

## Cross-cutting concern — Reach-aware cache scoping

**The leak risk:** Today the projection store in `doorway/doorway-service/src/projection/store.rs` keys cache entries by `{dna}:{type}:{id}`. A hosted steward (Tier 2) and an anon visitor (Tier 1) requesting the same content ID share a cache entry. If the steward's request lands first and populates the cache with their authenticated view (e.g., relationship-scoped metadata), the anon visitor's next request hits the cache and sees the authenticated view. This is a privacy bug.

**The fix:** Cache keys must include a *requester class* component: `{dna}:{type}:{id}:{reach_class}` where `reach_class` is one of `anon | hosted_user | steward(agent_id)`. Authenticated views never share keys with anon. Hosted-user views never share keys across distinct agents.

**Where it lands:** Whichever pattern ships first picks this up as a sub-task. B is the natural candidate because hosting-agreement-aware warm-stream already needs per-agreement scoping. Recovery also needs it because conductor-proxy responses are inherently per-agent.

**Red-team note:** This is the single highest-priority security finding from the access-tier walkthrough. Even ahead of the patterns above, the cache-scoping fix can ship as a defensive baseline.

---

## What this spec does NOT do

- It does not prescribe order. B, A, and Recovery are siblings; any can ship first depending on operator priority.
- It does not estimate cost. Each pattern needs a sprint-shape spec of its own to scope.
- It does not address the genesis-seed admin-promotion silent-skip (`genesis/docs/handoffs/2026-05-23-followup-3-genesis-seed-admin-promotion-degraded.md`) — that's a separate concern about CI hygiene and is mostly tangential to the access-tier model.
- It does not address the `alpha.elohim.host` SPA-blob PATCH credential — that's resolved at the CI layer (the `stageSpaBlob` change made the PATCH optional). The credential remains supportable; it is no longer a blocker.

---

## Related artifacts

- `doorway/CLAUDE.md` — "No Blob Fan-Out — Doorway is Single-Target Dispatch" (data-path; not warm-stream)
- `doorway/REACH.md` — the reach-gate primitive
- `doorway/doorway-service/RECOVERY-PROTOCOL.md` — recovery vision
- `genesis/docs/superpowers/specs/2026-04-19-self-healing-p2p-dataplane-design.md` — blob substrate; Pattern A extends or sibling-specs this
- `elohim/sdk/schemas/v1/objects/operator-classification.schema.json` — operate-doorway commitment schema (Pattern B substrate)
- `elohim/elohim-storage/src/db/rea_commitments.rs` — commitment query primitive (Pattern B substrate)
- `doorway/doorway-service/src/projection/warm_stream.rs` — current warm-stream (Pattern B target)
- `doorway/doorway-service/src/projection/app_auth.rs` — reach gate (Anon-hint target)
- `doorway/doorway-service/src/projection/store.rs` — cache store (reach-aware-scoping target)
- `genesis/docs/handoffs/2026-05-23-followup-1-doorway-warm-stream-architecture.md` — three-layer framing this spec resolves
- `genesis/docs/handoffs/2026-05-23-followup-2-k8s-handoff-summary.md` — k8s credential discussion (now optional after Jenkinsfile change)

---

## For the next /shift

The Objective is: **pick one of Pattern A, Pattern B, or Pattern Recovery and ship it.**

The operator decides at shift kickoff which pattern goes first. The sprint-shape spec for that specific pattern is written as part of the shift's first iteration (before any code). The shift produces a sprint-shape spec + the first ship-able code increment. Anon-hint is out of scope.

Pre-shift recommendation: ship the reach-aware cache scoping (cross-cutting concern above) as a defensive baseline before any of A/B/Recovery, since it's prerequisite for B and Recovery anyway and closes a privacy bug.
