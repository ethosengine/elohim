---
title: Doorway Federation & Failover Sprint — anycast-ready doorways for alpha + elohim.host
id: doorway-federation-failover-sprint-plan
status: open
class: substrate
sprint: born 2026-07-31 from the resiliency-saga close-out planning session (ch04 frontier). Mixed plan — no doc-level requires_env by convention; every task here is exercisable against the live alpha pair (read-mostly probes) or in-repo; the only externally-gated legs are the operator ceiling items (§Operator menu).
cites:
  - doorway/doorway-service/src/services/federation.rs
  - doorway/doorway-service/src/auth/jwt.rs
  - doorway/doorway-service/src/conductor/chaperone.rs
  - doorway/doorway-service/src/conductor/provisioner.rs
  - doorway/doorway-service/src/routes/catching_up.rs
  - doorway/doorway-service/src/routes/storage_proxy.rs
  - doorway/doorway-service/src/projection/epr_router.rs
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - genesis/orchestrator/manifests/infra/alpha-coturn-shem.yaml
  - genesis/orchestrator/manifests/infra/alpha-coturn-operations.yaml
  - genesis/a2o/features/dataplane/resiliency-saga/README.md
  - genesis/a2o/features/federation/doorway-multi-address-failover.feature
  - genesis/a2o/features/dataplane/doorway-catching-up-page.feature
  - genesis/a2o/steps/dataplane/resiliency-saga.steps.ts
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
  - dual-wan-utility-plane-failover | Dual-WAN Utility-Plane Failover | sha256:86f425b0045ce6d0 | path: genesis/docs/superpowers/specs/2026-07-16-dual-wan-utility-plane-failover-design.md
  - doorway-catching-up-page | Doorway catching-up shed page | sha256:2dbde4d56b074a5e | path: genesis/docs/superpowers/specs/2026-07-19-doorway-catching-up-page-design.md
  - genesis/manifests/spine.yaml
informed-by:
  - genesis/data/timeline/backlog/doorway-utility-plane-and-borrowed-infra-exit-doctrine.md
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
---

# Doorway Federation & Failover Sprint — dispatch prompt

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development or
> superpowers:executing-plans, task-by-task, two-stage review per task. The
> **p2p-design-gate skill is MANDATORY** before designing any new entity in this sprint
> (Task 2.2 mints one — its gate classification is pre-run in §Design Gate below and must
> be re-confirmed in-session). Story-first: WS1's feature file IS the specification; land
> scenario + implementation together.

**Goal:** `elohim.host` and alpha survive the loss — or the catch-up shed — of either
doorway for read traffic, hosted users get *honest* (never silently-forked) session
behavior across a doorway switch, and the proof is a runnable a2o failover check wired
to a new spine node — closing the resiliency-saga frontier (ch04/ch06 recording + the
"two doorways, one name" arc).

**Architecture:** Federation-over-p2p, SDK-first. Every failover primitive lands at the
substrate seam, not as doorway-local plumbing: doorway peer awareness rides the already-
wired `DoorwayRegistration` DHT entries; cross-doorway trust rides the already-published
JWKS (consumption side is the new work); the shed-vs-dead contract is the already-tested
`catching_up.rs` surface (measurement glue is the new work); apex multi-A rides the
already-proven beacon shared-record mechanism. The doorway stays a projection — nothing
in this sprint makes it a P2P participant or adds blob fan-out to storage peers.

**Model tiers** (operator directive 2026-07-02 — narrow tasks to cheaper tiers):
Opus = feature/spine authoring, trust-boundary design, doctrine calls. Sonnet = Rust/TS
implementation with defined interfaces. Haiku = doc-drift fixes, config wiring.

## Global constraints

- doorway-service native gate: `RUSTFLAGS="" cargo build --release && cargo test --lib --bins && cargo clippy -- -D warnings && cargo fmt --check` with `CARGO_TARGET_DIR` at the pool slot.
- Never `kubectl` from dev — manifests in `genesis/orchestrator/manifests/` are the surface; the live cluster is the operator's.
- One push per batch; a2o recording runs go through `[edge:validate-only]` (wired: `elohim/holochain/Jenkinsfile` `computeValidateOnly()`).
- New doorway HTTP routes need the match arm AND `is_service_path` AND a unit test (the `/auth/portal` shadow trap).
- a2o feature authoring is Opus work; step glue is Sonnet/Haiku (`genesis/a2o/CLAUDE.md`).
- Commit-only; the integrator pushes. No `git push` from sprint sessions.

## Grounded reality (2026-07-31 fan-out — what is WIRED vs not)

**Wired and reusable (do not rebuild):**
- `DoorwayRegistration` is a real infrastructure-DNA entry type with live coordinator fns (`register_doorway`, `get_all_doorways`, `record_health_attestation`) and boot-time registration + heartbeat + peer-health-probe loops (`services/federation.rs:161-356`).
- `FEDERATION_PEERS` peer discovery + cross-edge EPR-coherence probe (warn-only) runs on both alpha doorways (`alpha.yaml:284`, `alpha-b.yaml:309`).
- The EPR-projection router already does multi-peer fallback over DHT-discovered doorway endpoints (`epr_router.rs:177-238`) — the one legitimate multi-upstream surface.
- The 503 shed contract is complete and unit-tested (`catching_up.rs`: `ShedCause`, `Retry-After`, content-negotiated HTML/JSON; probes bypass the breaker).
- Multi-A DNS + client sticky failover is SHIPPED for `doorways.elohim.host` (beacon `--shared-record-name` on both coturn legs; ElohimClient + `apiBaseUrlInterceptor`, e2e-covered by `doorway-multi-address-failover.feature`).
- `VALIDATE_ONLY` edge mode is wired (`params.VALIDATE_ONLY` / `[edge:validate-only]` tag).

**The failover-critical gaps (this sprint's work):**
1. JWTs are HS256 with per-doorway k8s secrets (`alpha.yaml:294-300` vs `alpha-b.yaml:319-325`) — an A-minted token fails `verify_token` on B (`auth/jwt.rs:284-304`). JWKS endpoints are published but NOTHING consumes a peer's JWKS.
2. Cross-doorway landing for a hosted user either 502s (`chaperone.rs:143-153`, conductor-pinned token) or **silently provisions a fresh empty identity** (`provisioner.rs:271,372` — conductor-salted `generate_app_id` can never find the sibling's install).
3. `fetch_from_remote_doorway` (`services/federation.rs:373-549`) is fully implemented cross-doorway blob fallback with loop prevention — and has **zero callers**.
4. No a2o step classifies shed-vs-dead (the `status.json` `upstreams[].circuit`/`errorStreak`/`admission.shedTotal` fields exist on the wire; only an unimplemented `@wip` scenario pins them). No cross-doorway liveness comparator. No spine node for failover.
5. `elohim.host` apex is a single-owner A record (shem beacon, `alpha-coturn-shem.yaml:217-220`); the proven shared-record mechanism is not applied to the apex.
6. The structural ch04 red is NOT routing: adam's per-space arc-convergence ceiling under ~35 hosted agents (`self-heal-adam-projection-catchup-exhaustion-full-arc.md:180-220`) — an operator provisioning decision (partial mitigation `DOORWAY_MAX_AGENTS_PER_CONDUCTOR=32` applied).

**Honest anycast grading** (2026-07-16 design doc): §3a multi-A + client retry = the
mechanism this sprint completes (repo-controllable). §3b Cloudflare LB = operator-owned
new borrowed dependency (exit-doctrine ledger row required). §3c BGP anycast = filed
vision, not scheduled. Both WANs are Google Fiber — correlated failure; the 2-doorway
pair is the seed testbed for the N-doorway federated commons, not the destination.

## Design Gate (p2p-design-gate — pre-run; re-confirm in the Task 2.2 session)

### Entity: DoorwayRegistration
- **Classification**: Notarized (A) — EXISTS (infrastructure DNA). No new work; this sprint consumes it.
- **Source of Truth**: Holochain DHT. **Anti-pattern check**: none — sprint adds no fields.

### Entity: HostedAgentBinding (human → home-doorway/hosted-install resolution)
- **Classification**: Derived (A2) — a relationship of already-notarized entities (the human's imagodei identity ↔ the home doorway's `DoorwayRegistration`), NOT a standalone entry type. Link on the existing identity entry, tag carries `{doorway_id, installed_app_id}` (<256 bytes).
- **Content Address Strategy**: Agent-Scoped Composite — `(AgentPubKey, doorway registration hash, "hosted-at")`; the human's stance toward a doorway is the identity. Not content-derived (it's a mutable relationship), not slug.
- **Source of Truth**: Holochain DHT (the link). Storage projection: `hosted_agent_bindings` with `dht_anchor_hash` = parent identity entry's ActionHash. Mongo `conductor_registry` remains the doorway-local **operational** (Cat C) fast path; the link is what a *sibling* consults.
- **Coordinator Zome**: imagodei — link create/delete fns; exact names fixed in-session (check existing link-type headroom first; imagodei is at 28/~100 entry types but links need no new entry type).
- **HTTP Route (last)**: `GET /api/v1/federation/hosted-binding/{agent_pub_key}` on doorway, serving the projection.
- **Anti-Pattern check**: caught-and-corrected — the naive design (replicate Mongo `conductor_registry` between doorways) is a per-host scaffold extension; rejected for the A2 link. No UUID identity; no new entry type; no cross-namespace string-compare (binding keys by AgentPubKey, resolved via the canonical resolver where transport ids appear).

### Entity: doorway shed/admission state (semaphores, breakers, membrane guard)
- **Classification**: Operational (C) — CONFIRMED CORRECT as-is. Deliberately in-process, reset-on-restart, never shared. Siblings learn it by probing `/health` + `status.json` (the wired peer-health probe). This sprint must NOT replicate it — it only makes it *classifiable* (Task 1.2).

### Entity: doorway signing keys / JWT trust
- No new entity. The doorway Ed25519 node key is already published via `/.well-known/doorway-keys` and bound to the notarized `DoorwayRegistration`. Task 2.1 consumes it; key material stays operator-provisioned config.

### Design constraints discovered
- Task 2.2 (binding) depends on Task 2.1 (a sibling can only act on a binding for a *verified* foreign token).
- Task 3.3's doctrine call: `fetch_from_remote_doorway` is doorway→doorway HTTP on the federation seam — it does NOT violate the no-blob-fan-out rule (which forbids fanning out across *storage* peers); the rust-architect confirms or kills this reading before wiring.

---

## WS0 — Record the saga frontier (first, cheap)

### Task 0.1: ch04/ch06 recording run — **tier: any session / operator**
When `https://elohim.host/` serves 200 again (B out of its catch-up window — check
before pushing), push one empty commit tagged `[edge:validate-only]` (skips build/deploy,
runs Dataplane Validation only — no conductor restart, so it cannot reopen the window it
measures). **Verify:** `genesis/a2o/reports/sprint-report-dataplane.json` shows
`saga-04-doorway-serves` passed ≥1; note ch06 cross-node leg status in the sprint log.
One push per batch; do not race other sessions' pushes.

## WS1 — Truth & measurement (the spine-legal first move)

### Task 1.1: Author the failover red — **tier: Opus**
**Files:** Create `genesis/a2o/features/dataplane/doorway-failover.feature`; Modify `genesis/manifests/spine.yaml` (add node), `genesis/a2o/features/dataplane/resiliency-saga/README.md` (cross-reference).
**Spec:** New spine node `doorway-failover` (status `red`, runnable check = this feature; covenant: ≤12 nodes, currently 8). Feature `@e2e @dataplane @concern:doorway-failover`, three scenarios:
1. *Shed is not death*: when a doorway sheds (503 on a `/db/*` read), its `status.json` reports the cause (`upstreams[].circuit` open or `admission.shedTotal` rising) and the 503 carries `Retry-After` — the specified contract, not an outage.
2. *One of two doorways always serves*: `alpha-A` and `elohim.host` — at least one serves `GET /` → 200 with `app-root` (raw-body steps reused from ch04). Both-shedding is the red this arc exists to kill.
3. *Truth survives failover*: the surviving doorway serves the same declared head for `elohim-host-landing` (reuse the ch10/declared-head comparator steps verbatim).
Scenario 2/3 run against the live pair (organic shed windows are real red evidence — no synthetic chaos needed in v1; the `peer-loss-failover.feature` kill-on-purpose shape is the v2 escalation, noted in the feature header, not built now).
**Verify:** feature parses (`npx tsx` cucumber dry-run); scenarios execute (red is acceptable and expected); spine node lists the feature as its check.

### Task 1.2: Shed-vs-dead step glue — **tier: Sonnet**
**Files:** Create `genesis/a2o/steps/dataplane/failover.steps.ts`; Modify `genesis/a2o/src/framework/dataplane/surfaces.ts` (add `classifyDoorwayState(peerUrl): Promise<'serving'|'shedding'|'dead'>`), `genesis/a2o/features/dataplane/doorway-catching-up-page.feature` (de-`@wip` the circuit-state scenario its new step now implements).
**Interfaces:** `classifyDoorwayState` — `serving` = `GET /` 200; `shedding` = 503 whose JSON body is `{"status":"catching-up",…}` OR whose `status.json` shows an open upstream circuit / rising `admission.shedTotal`; `dead` = connect error/timeout on BOTH `/` and `/health`. Steps: `Then doorway {string} is serving|shedding (not dead)` and `Then at least one of doorways {string} and {string} is serving`.
**Verify:** run the feature against the live pair: `cd genesis/a2o && E2E_DOORWAY_ALPHA=https://doorway-alpha.elohim.host pnpm exec cucumber-js --tags '@concern:doorway-failover'`; scenario 1 must classify correctly during a live shed window if one is open, else against alpha-A serving. Existing `@dataplane` suite stays green.

### Task 1.3: Doc-drift fixes — **tier: Haiku**
**Files:** Modify `genesis/a2o/features/dataplane/resiliency-saga/04-doorway-serves.feature` (header comment "Status today: GREEN." → "Status: see the chapter table in ../README.md — the header is not the authority"), `doorway/doorway-service/FEDERATION.md` (top banner: the custodian-selection/blob-routing narrative (§ lines ~79-184) is SPEC-DRIFT contradicted by the enforced no-fan-out rule — `doorway/CLAUDE.md:32-57` is the authority; body left intact below the banner), `genesis/data/timeline/backlog/dataplane-peer-fallback-and-blob-replication.md` (frontmatter note: items 2 & 3 of "The fix" have since landed — `get_blob_or_heal`/`race_fetch` in `elohim/elohim-storage/src/http.rs`; remaining scope is item 1 pointer-propagation + item 4 crutch retirement).
**Verify:** `grep -n "Status today: GREEN" 04-doorway-serves.feature` → no match; each edited doc still parses (frontmatter intact). Managed-surface cite discipline: run cite refresh if any edited doc carries `cites:`.

## WS2 — Cross-doorway trust & session honesty

### Task 2.1: Verify sibling-minted JWTs via JWKS — **tier: Opus design + Sonnet implementation**
**Files:** Modify `doorway/doorway-service/src/auth/jwt.rs`, `doorway/doorway-service/src/routes/federation.rs` (JWKS response gains `kid` = doorway_id if absent), `doorway/doorway-service/src/services/federation.rs` (peer JWKS cache on the existing peer-discovery cache).
**Spec:** Keep HS256 for self-minted tokens (zero migration for existing sessions). Add an EdDSA (Ed25519, the node signing key already in JWKS) *dual signature path*: new tokens carry `alg=EdDSA, kid=<doorway_id>`; `verify_token` becomes: if `kid` absent or == self → HS256 (legacy) / self-EdDSA; else → look up peer JWKS (cache TTL ≥ 10min, sourced from the peer-discovery cache + `get_all_doorways` endpoints, never fetched per-request), verify EdDSA, reject unknown `kid`. `Claims` already carries `doorway_id`/`doorway_url` — no claim-shape change.
**Interfaces produced:** `JwtValidator::verify_token(&self, token) -> Result<Claims, JwtError>` unchanged signature; new `JwtError::UnknownIssuer` variant.
**Tests (write first):** (a) B-fixture JWKS + A-minted EdDSA token verifies; (b) tampered payload fails; (c) unknown `kid` → `UnknownIssuer`, never HS256 fallthrough; (d) legacy HS256 self-token still verifies.
**Verify:** doorway native gate green (build/test/clippy/fmt, pool `CARGO_TARGET_DIR`).

### Task 2.2: HostedAgentBinding (A2 link) + honest cross-doorway landing — **tier: Opus (rust-architect), gate re-confirmed in-session**
**Files:** Modify imagodei coordinator zome (link create on hosted provision; the DNA-hash rule applies — coordinator-only change → `update_coordinators` hot-swap path, no reinstall), `elohim/elohim-storage` projection (new `hosted_agent_bindings` table with `dht_anchor_hash`, migration comment `-- Source of truth: DHT`), `doorway/doorway-service/src/conductor/chaperone.rs`, `doorway/doorway-service/src/conductor/provisioner.rs`, new route in `server/http.rs` (match arm + `is_service_path` + unit test).
**Spec:** On hosted provision, the doorway (as the user's delegate) creates the link identity→`DoorwayRegistration` with tag `{doorway_id, installed_app_id}`. Chaperone cross-doorway landing (foreign but *verified* token, per Task 2.1): resolve binding → `307` redirect to the home doorway's `DOORWAY_URL` with a `X-Elohim-Hosted-At` header (browser-honest; no proxying, no re-provision). Binding unresolvable → `409 {"error":"hosted-elsewhere-unresolved"}` — never auto-provision on a foreign-issuer token.
**Verify:** doorway gate green; `cargo test export_bindings` regenerates TS types if any view changed (sha-diff clean otherwise); sweettest for the link fns if the imagodei suite has a harness slot (`RUN_SWEETTEST=1` on the push if not targeting dev).

### Task 2.3: Kill the silent identity fork (independent guard) — **tier: Sonnet**
**Files:** Modify `doorway/doorway-service/src/conductor/provisioner.rs` (`find_existing_app` / `auto_provision`), test in the same file's `#[cfg(test)]`.
**Spec:** Before provisioning a NEW hosted install for a `user_identifier`, consult the registry/Mongo for ANY existing mapping of that identifier (across all locally-known conductors) and — new — refuse when the incoming token's `doorway_id` ≠ self (`409 hosted-elsewhere`, matching Task 2.2's contract; this guard must land even if 2.2 slips — it converts the *silent fork* into an *honest refusal* with zero DHT dependency).
**Tests (first):** same `user_identifier`, foreign `doorway_id` claim → no provision call, 409; local token, unknown agent → provision proceeds (existing behavior).
**Verify:** doorway gate green.

## WS3 — Read-path anycast + the dead fallback

> **WS3 ground-truth revision (2026-07-31, in-sprint).** (a) Task 3.1 as
> specced is BLOCKED by encoded policy: `check-ingress-conflicts.sh` aborts any
> deploy where two differently-named ingresses claim one host in the namespace,
> and publishing apex multi-A before the operations leg can serve `elohim.host`
> would 404 half of apex traffic. Apex multi-A therefore needs the operator's
> ingress-topology decision FIRST (per-leg ingress classes or controller split)
> — demoted to Operator menu item 2 with this evidence; do not land the beacon
> diff ahead of it. (b) Task 3.2 is ALREADY SHIPPED: `environment.alpha.ts`
> carries `doorwayFallbacks: ['https://elohim.host']`, and a browser on the
> apex primaries its API calls to doorway-A via `resolveBaseUrl()` (origin ≠
> doorwayUrl → cross-origin primary) with sticky failover — data reads already
> survive B's shed for users holding the shell. The unclosed apex leg is the
> ROOT-DOCUMENT serve only. (c) New Task 3.4 (warm-boot shell cache) is the
> repo-side cure for that leg.

### Task 3.1: Apex multi-A manifest diff (operator-gated apply) — **tier: Sonnet**
**Files:** Modify `genesis/orchestrator/manifests/infra/alpha-coturn-shem.yaml`, `genesis/orchestrator/manifests/infra/alpha-coturn-operations.yaml`.
**Spec:** Add `--shared-record-name elohim.host --record-owner <shem|operations>` contribution to BOTH beacon legs (exact flag shape copied from the working `doorways.elohim.host` stanzas at `alpha-coturn-shem.yaml:227-234` / `alpha-coturn-operations.yaml:232-239`), REPLACING the shem leg's exclusive `--record-name elohim.host` ownership. Header comment documents rollback (revert to single-owner) and the ingress precondition: doorway-A's ingress must accept the `elohim.host` host (add the host to `alpha.yaml`'s ingress + TLS SAN — include that diff) or A answers 404 for apex traffic. **This lands as a committed, ready-to-apply diff; the operator decides when the pipeline reconciles it** (outward-facing DNS semantics change — §Operator menu item 2).
**Verify:** `check-ingress-conflicts.sh` passes against the rendered manifests; yaml parses; no other manifest claims the apex host.

### Task 3.2: Configure prod client fallbacks — **tier: Haiku**
**Files:** Modify the elohim-app production environment configs (`app/elohim-app/src/environments/` — the file(s) defining `environment.client.doorwayFallbacks`, currently unset in prod builds).
**Spec:** apex build → fallbacks `["https://alpha.elohim.host"]`; alpha build → `["https://elohim.host"]`. This activates the ALREADY-SHIPPED sticky-preferred failover (interceptor + ElohimClient); no client code changes.
**Verify:** `pnpm run build` green; the `doorway-multi-address-failover.feature` "no-fallback-configured" scenario now runs its *configured* siblings instead.

### Task 3.3: `fetch_from_remote_doorway` — wire or delete — **tier: Opus doctrine call, Sonnet implementation**
**Files:** Modify `doorway/doorway-service/src/cache/delivery_relay.rs`, `doorway/doorway-service/src/services/federation.rs`.
**Spec:** Doctrine question first (rust-architect): a single-sibling doorway→doorway HTTP fetch after local-storage 404 is federation-seam behavior, not storage fan-out — confirm against `doorway/CLAUDE.md:32-57`, then wire it as DeliveryRelay's final fallback tier exactly as the function's own doc comment (`federation.rs:364`) claims, with its existing loop-prevention header and a per-request budget of ONE remote attempt. If the doctrine call goes the other way: delete the function and its FEDERATION.md claims in the same commit (dead code with a lying doc comment is the worst state — the current one).
**Tests (first, if wiring):** local 404 + healthy sibling → bytes served, loop-prevention header set; sibling also 404 → original 404 preserved (no error masking); loop header present on inbound → no recursive fetch.
**Verify:** doorway gate green; a2o Task 1.1 scenario 3 exercises the path live.

### Task 3.4: Warm-boot shell cache — serve `/` through the catch-up window — **tier: Opus design + Sonnet implementation**
**Files:** Modify `doorway/doorway-service/src/render/registry.rs` boot path, `doorway/doorway-service/src/cache/app_file_cache.rs` consumers, the `/` serve path that currently sheds.
**Spec:** The 503 shed on `/` during post-restart catch-up exists because the SSR hot cache is in-memory (dies with the pod) while the declared-head fetch needs the still-catching-up upstream. The Mongo-backed `app_file_cache` (keyed `{slug}:{file_path}:{blob_hash}`) already holds the last reconciled bundle — derivable, content-addressed truth. At boot: hydrate the render registry's served-bundle head from the last reconciled `BundleHead` and serve the shell from cache (browser navigations only) with an explicit staleness marker header (`x-elohim-bundle: last-reconciled`) while catch-up runs; the existing reconcile loop upgrades to the declared head the moment the upstream answers, exactly as today. API/data routes keep the shed contract untouched — this changes ONLY the shell serve, honoring the pinned `x-ssr-fetches: 0` cache-warm invariant (doorway-catching-up-page `@wip` scenario — de-`@wip` it with this task). Serving the last content-addressed bundle is serving true (possibly one-behind, marked) content — not a violation of the honest-shed design, whose target is data reads.
**Verify:** doorway gate green; the born-red apex scenario (`doorway-failover.feature`) flips green on the next post-deploy catch-up window; the cache-warm scenario de-`@wip`'d and passing.

## Sequencing

```
WS0 (T0.1, immediate, gated on B's recovery)
WS1: T1.1 → T1.2   (T1.3 anytime)          ← the red exists before the cures land
WS2: T2.1 → T2.2   (T2.3 independent, early — smallest honest win)
WS3: T3.1, T3.2 anytime; T3.3 after T1.1 (its proof scenario exists)
```
Cross-workstream: nothing in WS2/WS3 blocks WS1; land the measurement first so every
cure flips a visible red. Story-harvest on branch finish (parameter-bearing discoveries:
JWKS cache TTL, remote-fetch budget, shed classification thresholds).

## Operator menu (ceiling items — decisions, not agent work)

1. **Adam hosted-agent provisioning** — the structural ch04 ceiling. Cap is 32; the apex-only hosted-provisioning policy (dc73c5d09) says spillover onto household conductors is a placement smell. Decide: hold at 32 / shard onto a second shem conductor / drain-and-rebalance.
2. **Apex multi-A flip timing** — apply Task 3.1's committed diff (DNS semantics change; both-doorways-healthy window recommended).
3. **JWT dual-alg migration window** — Task 2.1 ships dual-verify; decide when HS256 minting stops (token TTL horizon after EdDSA minting starts).
4. **Cloudflare LB (§3b)** — adopt-or-not; if adopted it needs its own exit-doctrine ledger row as a new borrowed dependency.
5. **Demand-autopin retirement semantics** — saga residue; policy on what `active` means once provably unfetchable.

## Disjoint packages — Codex-claimable (shared backlog, per the side-delegation queue convention)

Disjointness verified against this sprint's write-set (doorway-service + a2o + doorway/infra manifests + imagodei zome). P1 before P3/P4 (P3/P4 classify by the join key P1 canonicalizes).

| Pkg | Backlog entry | Tree | Note |
|---|---|---|---|
| P1 | `2026-07-30-blobhash-serverblobhash-duality-canonical-join-key.md` (high) | elohim-storage + seeder | Fully specified; the join-key spec-then-implement task |
| P2 | `gherkin-prepush-lint.md` | a2o tooling | Pure tooling; kills the parse-abort-drops-whole-suite class |
| P3 | `2026-07-30-custody-blob-first-commitment-auto-producer.md` (medium) | elohim-storage + mishpat | p2p-design-gate MANDATORY in-session (mints a notarized commitment) |
| P4 | `capacity-tier-pledge-producer.md` (new, this sprint) | elohim-storage facings + mishpat | Widens saga ch9 back to `totalPledgedBytes`; gate-mandatory like P3 |
