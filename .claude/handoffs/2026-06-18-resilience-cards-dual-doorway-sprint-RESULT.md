# SPRINT RESULT — Resilience cards + dual-doorway EPR (post-leak-unblock), 2026-06-18

Companion to `2026-06-18-resilience-cards-dual-doorway-sprint-handoff.md`. Honest DoD
split: what the sprint controlled & landed, vs what the operator fork gates.

Branch: `feat/frontend-eyes-sprint` (commit-only; integrator dev-merges → CI).

## Landed & committed (sprint-controlled, household-provable code — DoD bucket (i))

| # | Item | Commit | Verification |
|---|------|--------|--------------|
| Stage 2 #6 | Proof-suite baseline green | (pre-existing tree) | `household_resilience` 28 + `chaos_dataplane` 4 = **32 green** |
| Stage 1 #3 | **Bulk-seed anchor** — `dhtAnchorHash` at content ingest | `38d9b2247` | provenance suite 13/13 incl. new ingest-anchor acceptance test; ts-rs bindings regenerated; fmt clean |
| Stage 1 #4 | **HTTP reach enforcement** (local serve path) — close intimate-content leak | `d09928387` | full lib **1675 passed / 0 failed**; 5 new DB-backed reach tests incl. the security assertion (`intimate_denies_unrelated_human`); red-team-cleared as a tightening; P2P regression green; fmt clean |

**#3 detail.** Added optional content-derived `dhtAnchorHash` to
`CreateContentInputView` / `CreateContentInput` / `NewContent` so seed/import content
satisfies the `require_provenance` read gate (`dht_anchor_hash IS NOT NULL OR
p2p_published_at IS NOT NULL`) **at ingest** — no libp2p publish-drain round-trip — on
hub-optional / peer-starved stacks. This is the honest alternative to stamping
`p2pPublishedAt` (which asserts a DHT publication that never happened). Closes the
**storage half** of `seed-provenance-anchor-gap.md` + `ci-seeder-stamp-conductor-anchor-circularity.md`.
**Consumer follow-on (NOT done):** seeder computes the content-derived CID and passes
`dhtAnchorHash` (genesis/seeder TS); the seeder currently stamps `p2pPublishedAt`
(`seed-sqlite.ts:903`) — migrate that to the honest ingest anchor.

**#4 detail.** Extracted `EprService::authorize_reach_for_human` (the single-source reach
core, operates on a pre-resolved `Human`); wired into `GET /db/content/{id}` for tiers
above community, resolving the requester by the reliable `humans.id` (then agent key),
deny-by-default. `check_reach_authorization` (sole P2P caller) keeps its fast-path and
delegates to the same core. **p2p-design-gate** answered (content-derived CID for the
anchor; creator-read-exemption recommended YES-narrowly — see below).

## Operator-gated / surfaced (DoD bucket (ii) — NOT in the sprint's control)

### THE FORK — `GET /auth/me` is unresolvable from the dev environment
- My external probe → `401 "No token provided"` — that is the **no-token path**, NOT the
  decisive in-pod, post-reseed measurement the handoff specifies. It does **not** resolve
  the fork.
- North-star #1 (live card "N of X") stays gated on: integrator **dev-merge of this
  branch + reseed + the jemalloc conductor deploy**, THEN `GET /auth/me` on a healthy
  matthew `elohim-storage` pod:
  - **200 + agentPubKey** → deploy+reseed is the whole lever; the card can light.
  - **401** → the structural finding holds; a session/key-population path is needed — and
    that path is **p2p-design-gate + security-owned** (economic attribution), blocked by
    `2026-06-15-coherent-transport-identity-resolver-design.md`.
- **I did NOT, and per guardrail #2 will NOT, touch `agent_pub_key` population or session
  minting.** Surfaced, not built.
- **#4's live-alpha enforcement is coupled to the SAME `agent_pub_key` fork** — the
  steward/relationship arms need `humans.agent_pub_key` populated on a live pod.
  Household-provable now (the unit tests prove it); live-gated on the fork.

### F-BOOTSTRAP (Stage 4 #8) — shared mongo ✓, convergence PARTIAL
Probed live 2026-06-18 ~20:50Z:
- `alpha.elohim.host/admin/bootstrap-coherence` == `elohim.host/admin/bootstrap-coherence`
  == `{backend:"mongo", spaces:11, agents:1686}` — **identical → F-BOOTSTRAP core ✓.**
- `/health`: alpha (matthew, dev) `p2p.peerCount:13, caughtUp:true, discoveryComplete:true`,
  conductor 4/4 workers, 14/14 pools — **jemalloc leak mitigation holding** (no flap).
  apex (adam, prod) `peerCount:12, caughtUp:false, discoveryComplete:false` — **adam not
  yet at steady-state → genesis-pair convergence is PARTIAL, not confirmed.**
- Side evidence: peerCount 13/12 confirms the **F-EDGE** premise (the doorway `/p2p-peers`
  honesty fix should report ~13, not 1). `divergentAnchor:2` on alpha is a real
  **F-COHERENCE** signal (content heads diverging — the instrument #9 would measure).

## Residuals filed (red-team findings — NOT blockers, "no worse than prior all-200 state")
`genesis/data/timeline/backlog/http-reach-cross-node-fallback-bypass.md`:
- **FINDING 1 (MED / HIGH multi-node):** the P2P **cross-node fallback** serve path
  (`http.rs:~4700`) does not re-apply the reach gate against the HTTP requester (gates on
  the fetching node's identity). Latent on the single-household stack; activates
  multi-node. Remediation specced (forward requester identity into `resolve_and_fetch`, or
  a shared `enforce_http_reach` helper at both return sites).
- **FINDING 2 (deliberate):** community-tier left at coarse auth.
- Input hygiene: non-canonical reach values (legacy `invited`) map to index 0 → gate
  skipped; the reach-floor canonicalization must land.

## Design-gate decisions (self-answered, per operator's autonomous style)
- **Anchor value:** content-derived CID (content-addressed identity), NOT a `p2pPublishedAt`
  false-provenance stamp, NOT a UUID.
- **Creator-read-exemption (`require_provenance` for "read own content"):** RECOMMEND YES,
  narrowly — keyed on `content.created_by == resolved-requester`; separable from the
  additive field and from #4; supports the hub-optional floor. **NOT built this sprint**
  (no consumer demand surfaced; recommendation recorded for the operator).

## Remaining sprint items — ready to pick up (handoff Stages 2–4, unblocked, leak-independent)
- **Stage 3 #7** card-honesty `onlinePeerCount → {live, known}` num/denominator pair
  (CI-green-now; the `distributionState` half already shipped). Schema → Rust struct →
  `schema_contract` → codegen → snapshot component.
- **Stage 4 #9** F-COHERENCE detector (`doorway/.../routes/coherence.rs`, pure
  `router_fingerprint`, TDD `RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/dw-test`) — `divergentAnchor:2`
  above shows there is real divergence to measure.
- **Stage 4 #10** F-EDGE `/p2p-peers` honesty (1→~13; `federation.rs`) — **NOT the
  resilience card** (different crate/surface).
- **#3 seeder consumer wiring** (compute content-derived CID, pass `dhtAnchorHash`,
  migrate off the `p2pPublishedAt` stamp).
- **#4 FINDING 1 fix** (cross-node fallback reach enforcement).

## Operator next actions (the decisions only you can make)
1. **Dev-merge this branch + reseed + deploy jemalloc conductor**, then run the
   `GET /auth/me` probe on matthew's pod — the single discriminator for north-star #1.
2. Decide FINDING 1 (cross-node reach): fix now vs accept filed (it is not worse than the
   prior state).
3. Leak-track cleanup (non-sprint): revert the temp prof deploy `b8481f090` after a
   non-prof jemalloc conductor ships.

## Guardrails honored
Did not work the leak · ran the fork probe before any live-write reasoning (and refused to
false-resolve it) · measured on live alpha (`/health`, `/admin/bootstrap-coherence`) not
just CI · deep-proved on the household floor · snake_case stayed in Rust (ts-rs regen) ·
no `kubectl`, no edge-Jenkinsfile/manifest apply · F-EDGE kept distinct from the resilience
card · committed each increment durably; never pushed.

---

## ADDENDUM — continuation (CID-canonical sweep + F-COHERENCE Wave-F1)

After the spine (#3, #4) landed, the session continued on operator direction.

**Four commits this session (`feat/frontend-eyes-sprint`):**

| Commit | What |
|---|---|
| `38d9b2247` | #3 bulk-seed `dhtAnchorHash` at ingest (storage half of seed-provenance-anchor-gap) |
| `d09928387` | #4 HTTP reach enforcement (intimate-content leak closed; red-team-cleared; cross-node residual filed) |
| `3c1e706bf` | **CID-canonical addressing coherence** — rule + gospel + pillar-decomposition |
| `33f4e318c` | **F-COHERENCE Wave-F1 (Tasks 1-2)** — CID self-fingerprint detector + route |

**CID-canonical sweep (operator catch: "CID is canonical over sha").** A CID is the same
sha2-256 wrapped in a self-describing multihash+codec; "use a CID" = stop exposing the
bare hash, expose the CID that wraps it.
- **Rule** pinned in `.claude/skills/p2p-design-gate` Step 2 "Canonical address forms"
  (+ anti-pattern row): `bafyrei…` dag-cbor for atoms/content/fingerprints; `bafkrei…` raw
  for blobs; `uhCAk` for agent/action; cite `sha256:` + dedup/byte-verify left
  (non-addressing). The anti-recurrence anchor.
- **Gospel** reconciled (`elohim-storage/CLAUDE.md` + `doorway/CLAUDE.md`): CID-canonical
  declared; bare `sha256-<hex>` blob wire marked **legacy/in-migration** (current behavior
  kept honest; the blob-plane bare-hash→CID *code* migration is a named downstream arc, NOT
  started).
- **Corpus** (workflow): F-COHERENCE plan digest + ledger FS1 + pillar-epr-decomposition
  `cid="sha256-…"` conflation → CID. `commitment_id` sha256 relabeled as a pre-DHT
  idempotency key (notarized identity is the Mishpat `entry_hash uhCAk`). Descriptive-current
  blob examples annotated legacy, not rewritten-to-lie. Long-tail (byte-verify/dedup/cite-fp/
  agent-key) intentionally left. The plan+ledger CID edits live on those (still-untracked)
  federation docs — left for that work's owner.
- Flagged follow-up: `sprint1-zd-substrate-correct-deploy:891` ("agent CID = sha256 of
  pubkey" — agent identity should be `uhCAk`).

**F-COHERENCE Wave-F1 (Tasks 1-2), CID-first.** `GET /api/v1/federation/coherence` (was
404) now serves this edge's self-fingerprint: `CoherenceManifest` whose `digest` is a
**CIDv1 dag-cbor (`bafyrei…`)** over the sorted `(url_path, epr_id)` head set — NOT a bare
sha (the plan's `sha256-` deviated-from per operator direction). `build_id` carries the
deploy SHA so deploy-skew ≠ content-skew. Verified: `cargo test --lib coherence` 3/3
(`bafy…`, deterministic, order-independent, camelCase); whole lib compiles; fmt clean;
doorway clippy clean on the new files (only pre-existing ts-rs serde-attr noise).
**Honesty:** `router_fingerprint` (the pure logic) is unit-tested; the route *handler* is
compile-verified + pure-logic-tested only — no handler-invocation test, matching the
`bootstrap_coherence` precedent. Full `cargo test --lib --bins` + `clippy -D warnings` are
the integrator's pre-push gate.

**What remains (next sessions):**
- **F-COHERENCE Task 3-4** — cross-edge probe (`in_agreement`) + WARN alarm. Task 3 is
  invasive (threads params through `spawn_peer_discovery_task` + `main.rs` + `federation.rs`;
  the plan flags `main.rs` disjointness vs P-DIAGNOSTIC) → fresh session.
- **#10 F-EDGE** — `/p2p-peers` count honesty (1→~13; live `/health` confirms peerCount
  13/12). NOT the resilience card.
- **#7** — resilience-card `onlinePeerCount → {live, known}` denominators (CI-green-now).
- **#4 FINDING 1** — cross-node fallback reach enforcement
  (`http-reach-cross-node-fallback-bypass.md`).
- **#3 seeder wiring** — compute the content-derived CID, pass `dhtAnchorHash` (migrate off
  the `p2pPublishedAt` stamp).

**Still operator-gated (unchanged):** the `GET /auth/me` fork (north-star #1 card-light)
needs dev-merge + reseed + jemalloc deploy, then the in-pod probe. NOT auto-built.
