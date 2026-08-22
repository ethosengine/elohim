# Surface census — mechanical WIRE/FIXTURE/IMPLEMENT/STRUCTURAL/DEFECT classification

_Generated 2026-08-22T01:35:15.953Z by `pnpm census` (`scripts/surface-census.ts`). Population: every `@wip` or failed Act I/host scenario, joined against a scoped `cucumber-js --dry-run` bind check and live read-only probes of the surfaces its steps name. Re-run with `pnpm census` (or `pnpm census -- --fresh` to bypass the dry-run cache) — the classifier is read-only against the mesh (GET only) and idempotent._

Mesh health: doorway UP, storage UP — full probe.

Latest mesh report(s) cross-referenced for DEFECT-STALE: /tmp/elohim-local-mesh/reports/mesh.json

## Totals

| class | count |
|---|---|
| WIRE | 311 |
| FIXTURE | 58 |
| IMPLEMENT-BOUNDED | 3 |
| IMPLEMENT-DESIGN | 0 |
| STRUCTURAL | 73 |
| DEFECT-STALE | 38 |
| UNCLASSIFIED | 92 |
| **total classified** | **575** |

## Wave1b re-run — manual reclassification (2026-08-22)

The mechanical classifier above cross-references failures against `mesh.json` but has no
way to tell "genuine defect" apart from "known scope mismatch" or "endpoint not yet
designed" from an assertion string alone. Reading the actual `mesh.json` failure bodies
surfaces two recurring patterns that were sitting in DEFECT-STALE for the wrong reason;
reclassified by hand below with the exact scenario each covers.

**Pattern 1 — missing corpus row, not a defect (`DEFECT-STALE` → `FIXTURE`).** Six
scenarios fail with "Content not found: manifesto" (or the same 404-instead-of-expected-status
shape on a different content id) — the same class as the wave1 lamad-spa gap: a seeded
corpus row the Prologue doesn't create, not broken code.

- `features/auth/reach-commons.feature` :: Anonymous reader can read the manifesto (earned commons reach)
- `features/auth/reach-commons.feature` :: Anonymous reader is rejected for community-reach content (403 with requiredReach) — got 404 not 403, i.e. the community-reach content item itself doesn't exist either; same missing-corpus-row shape, different content id
- `features/content/epr-content-addressing.feature` :: Blob content loads via CID
- `features/content/epr-content-addressing.feature` :: EPR Head carries three-pillar metadata
- `features/federation/peer-recovery.feature` :: A wiped device recovers its stewarded content from the mesh
- `features/resilience/chaos-peer-churn.feature` :: Simultaneous loss of two peers leaves the survivor degrading honestly

(`features/federation/peer-loss-failover.feature`'s three scenarios and
`features/resilience/chaos-peer-churn.feature`'s "A flapping peer never corrupts what the
mesh believes" show this SAME manifesto-404 in their live `mesh.json` failure body too, but
the mechanical classifier already parks them in FIXTURE for a missing-env-var reason that
fires first — no reclassification needed, just noting the underlying cause agrees.)

Two DEFECT-STALE scenarios in these same files stay put — their failures are a different,
real root cause, not the corpus gap: `features/federation/peer-recovery.feature` :: A wiped
peer's commitment projection reconciles from its own conductor (no way to clear a peer's
existing projection — a missing operator verb) and
`features/resilience/chaos-peer-churn.feature` :: Cascading peer loss degrades the
protection status honestly, step by step (a real provider-identity mismatch).

**Pattern 2a — fleet-size scope mismatch, not a defect (`DEFECT-STALE` → `FIXTURE`).** Five
scenarios assert `connectedPeers >= 3` or `>= 5`; this household mesh is a 3-peer fabric (2
connected from any one member's own view), and these assertions were written for the
7-peer alpha fleet. The scenario is honest about the mesh it's running on — the fixture
(fleet size) doesn't match what the scenario needs, which is the same shape as an
`E2E_*` precondition gap.

- `features/deployment/p2p-validation.feature` :: Storage pauses P2P sync during account import
- `features/deployment/p2p-validation.feature` :: Storage pauses P2P sync during bulk content creation
- `features/deployment/p2p-validation.feature` :: Sync auto-suppressed while drain backlog is large
- `features/deployment/p2p-validation.feature` :: Sync resumes even if bulk write fails
- `features/deployment/sync-control.feature` :: Without sync mode control, mobile device burns cellular data

**Pattern 2b — endpoint never designed, not a regression (`DEFECT-STALE` → `IMPLEMENT-DESIGN`).**
Five scenarios fail on `POST /p2p/sync-mode` or `GET /p2p/sync-mode/history` returning 404.
Each failure carries its own FINDING note: `P2PStatusInfo` (`elohim/elohim-storage/src/p2p/mod.rs`)
has no `syncMode`/`networkClass` field, and `elohim-storage/src/http.rs` has no handler for
either route — the endpoint was never built, so this is a route-absent design gap (the
census script's own `IMPLEMENT-DESIGN` class), not a wired path that broke. The mechanical
classifier missed it only because these scenarios drive the endpoint through a POST helper
step rather than a literal `I query "<path>"` GET the static prober recognizes.

- `features/deployment/sync-control.feature` :: Operator pauses sync explicitly
- `features/deployment/sync-control.feature` :: Operator resumes sync after pause
- `features/deployment/sync-control.feature` :: Sync mode transitions are logged for auditability
- `features/deployment/sync-control.feature` :: Wifi-only mode pauses sync when cellular is the active network
- `features/deployment/sync-control.feature` :: Wifi-only mode resumes sync when device joins wifi

One further `deployment/p2p-validation.feature` DEFECT-STALE row — "P2P status endpoint
exposes sync_paused state" (doorway `/health` never proxies the `syncPaused` field
elohim-storage's own `/p2p/status` already carries) — is a real gap outside both patterns
above; left classified DEFECT-STALE (arguably IMPLEMENT-BOUNDED once someone triages it,
but that call is deliberately left to a human, not inferred here).

**Pattern 3 — missing category-tagged corpus, not an allocator defect (`DEFECT-STALE` →
`FIXTURE`).** Five `content/stewardship-allocation.feature` scenarios fail on an empty or
under-populated content universe for a given category/tag ("No content found with tag
'value-scanner'", "No multi-steward allocation among the 1 scanned 'fct' items", "Average
stewards per content item is 1.00, expected > 1", etc.). Per the corpus seeder's live
investigation: the allocator already implements multi-steward `CATEGORY_STEWARD_MAP`
behavior correctly — this is a Prologue-seeding gap (the tagged corpus rows these
scenarios need aren't created yet), not a code defect, and not something that needs an
`IMPLEMENT` verb. The seeder is adding the tagged rows to the Prologue directly; expect
these to flip to passing on the next wave with no code change.

- `features/content/stewardship-allocation.feature` :: Faith content stewarded by pastoral affinity
- `features/content/stewardship-allocation.feature` :: No steward has exclusive ownership
- `features/content/stewardship-allocation.feature` :: Stewardship reflects human affinities
- `features/content/stewardship-allocation.feature` :: Uncategorized content falls back to bootstrap steward
- `features/content/stewardship-allocation.feature` :: Value-scanner content has multiple stewards

**Adjusted totals after manual reclassification:**

| class | mechanical | adjusted |
|---|---|---|
| WIRE | 311 | 311 |
| FIXTURE | 58 | 74 |
| IMPLEMENT-BOUNDED | 3 | 3 |
| IMPLEMENT-DESIGN | 0 | 5 |
| STRUCTURAL | 73 | 73 |
| DEFECT-STALE | 38 | 17 |
| UNCLASSIFIED | 92 | 92 |
| **total classified** | **575** | **575** |

## @wip staleness check against mesh.json (2026-08-22)

**Zero `feature:scenario` lines are provably stale** (a `@wip` scenario whose live run
reported `status === passed`). This isn't a clean bill of health on the existing `@wip`
backlog — it's a structural blind spot in this run's only surviving evidence:

- The `mesh` cucumber profile (`cucumber.mjs`) runs with `tags: '@e2e and not @wip and not
  @browser and not @browser-only'` — every `@wip`-tagged scenario is excluded from
  execution, and therefore from `mesh.json`, by construction. Verified directly: no `@wip`
  tag appears anywhere in `mesh.json`'s 264 scenario or 79 feature tag lists.
- The `saga` profile carries no such tag filter, so `@wip` scenarios under
  `features/dataplane/resiliency-saga/**` DO run there — but this run's scoped saga JSON
  was clobbered by the later full-lane write before the justfile fix (`5951ff8a8`) landed
  mid-run (01:20:20, after the saga stage had already finished at ~00:57). Only the
  aggregate console summary survived: 22 scenarios, 15 passed / 6 failed / 1 pending — not
  enough to attribute pass/fail to individual scenarios.
- The 92 `UNCLASSIFIED` rows split into two buckets, neither of which yields stale-`@wip`
  evidence: 50 are bind-class-a (fully wired) `@wip` scenarios with no `mesh.json` entry at
  all (excluded by the tag filter above — this is exactly the population `@wip` untagging
  would need saga/alpha/local-lane coverage to resolve, not `mesh.json`); 42 have no
  HTTP/metric/field surface the static prober can name in step text or glue at all (a
  prober blind spot, independent of report coverage).

Re-running `just test mesh features/dataplane/resiliency-saga` alone (now that the report
path fix survives a later full-lane run) is the next re-measurement that could actually
populate this check for the saga scenarios; the household mesh's `mesh` profile will never
be able to, by design.

## Fixture-aware scoping fix (2026-08-22)

`features/dataplane/resilience-identity-coherence.feature` :: No household-placed human on
alpha-A is missing its agent_pub_key — sits in `FIXTURE` this run (gated on
`E2E_SHEM_HOST`, so it never executed under the household mesh). Per the corpus seeder's
live investigation against alpha directly (not this run's evidence, but verified live, not
hypothesis): when it DOES run, it reports 4 offenders — `human-adam-firstman`,
`human-eve-firstwoman`, `human-gertrude-grandma`, `human-susan-household` — all narrative
household members with zero conductors on the 3-node household mesh. `seed-humans`
deliberately writes `agentPubKey` NULL for them (agent keys are conductor-minted; there is
no truthful seed source), so the blanket invariant was flagging fixture households as
identity-coherence violations. This is `STRUCTURAL`, not a defect.

Fixed directly (small, in-slice): `steps/dataplane.steps.ts`'s `no HOUSEHOLD-member human
on peer {string} is missing its agentPubKey` step now scopes the invariant to
DHT-observable households only — mirroring the sibling fossil-check step's existing
"observable" scoping (`every observable HOUSEHOLD-member human on peer {string} has a
non-fossil agentPubKey`, same file). A household with zero live members on this peer is out
of scope entirely; once any one member resolves to a real key, every other member of that
same household must too — that's the actual coherence bug the scenario guards. No
`@requires:` tag added — the scoping happens at the assertion layer, not the gate.

## First ten cheapest, per class

### WIRE (311 total)

- `features/auth/recovery/recovery-m5-portal-host-discovery.feature` — Add a portal host — write step glue (1 step(s) unbound, class b)
- `features/auth/recovery/recovery-m5-portal-host-discovery.feature` — Validator rejects http URL — write step glue (1 step(s) unbound, class b)
- `features/delivery/delivery-diagnostics.feature` — Without projection cache, browser load overwhelms storage — write step glue (1 step(s) unbound, class b)
- `features/delivery/landing-page.feature` — An SSR render that activates no route component sheds to the bundle fallback — write step glue (1 step(s) unbound, class b)
- `features/deployment/human-device-mapping.feature` — Every nodeTypes entry is in the allowed cluster vocabulary — write step glue (1 step(s) unbound, class b)
- `features/deployment/human-device-mapping.feature` — humanId matches the convention "human-<humanLabel>" — write step glue (1 step(s) unbound, class b)
- `features/lamad/love-map-negotiation.feature` — Love map path is invisible to non-participants — write step glue (1 step(s) unbound, class b)
- `features/resilience/resilience-dimensions.feature` — Content stewarded by no household reads at-risk, honestly — write step glue (1 step(s) unbound, class b)
- `features/resilience/resilience-dimensions.feature` — Two stewarding households lift content to partial — write step glue (1 step(s) unbound, class b)
- `features/auth/conductor-pool-recovery.feature` — Doorway exposes orphan-mapping count for operators — write step glue (2 step(s) unbound, class b)

### FIXTURE (58 total)

- `features/content/epr-content-addressing.feature` — EPR popover surfaces all three pillars when present — precondition missing: E2E_DEVICE_MODE
- `features/content/epr-content-addressing.feature` — Following an EPR link transfers reading context to the destination — precondition missing: E2E_DEVICE_MODE
- `features/dataplane/resilience-identity-coherence.feature` — No household-placed human on alpha-A is missing its agent_pub_key — precondition missing: E2E_SHEM_HOST
- `features/dataplane/resilience-identity-coherence.feature` — No household-member human on alpha-A carries a fossil agentPubKey — precondition missing: E2E_SHEM_HOST
- `features/delivery/client-resilience.feature` — Service Worker registers in browser — precondition missing: E2E_DEVICE_MODE
- `features/delivery/client-resilience.feature` — Service Worker registers in Tauri WebView — precondition missing: E2E_DEVICE_MODE
- `features/delivery/client-resilience.feature` — Cached app works offline — precondition missing: E2E_DEVICE_MODE
- `features/delivery/client-resilience.feature` — Cached app survives storage pod restart — precondition missing: E2E_DEVICE_MODE
- `features/delivery/client-resilience.feature` — SW probes peer capability before fetching assets — precondition missing: E2E_DEVICE_MODE
- `features/delivery/client-resilience.feature` — SW fetches individual files from a peer with warm extraction — precondition missing: E2E_DEVICE_MODE

### IMPLEMENT-BOUNDED (3 total)

- `features/browser/doorway-dashboard-health.feature` — Dashboard handles missing orchestrator gracefully — add: doorway_auth_token
- `features/peer-oauth-portal/rp-consent.feature` — User approves a per-claim consent — add: elohim_session
- `features/peer-oauth-portal/rp-consent.feature` — User declines consent — add: elohim_session

### STRUCTURAL (73 total)

- `features/auth/contributor-presence-claim-ceremony.feature` — The claim convening is convened and witnessed — needs shem (multi-tenant commons canvas — no household analog)
- `features/auth/contributor-presence-claim-ceremony.feature` — The negotiated backfill is recorded as append-only events — needs shem (multi-tenant commons canvas — no household analog)
- `features/auth/contributor-presence-claim-ceremony.feature` — A settlement is appealable through correcting events — needs shem (multi-tenant commons canvas — no household analog)
- `features/auth/recovery/recovery-shamir-optional.feature` — Recovery never asks Matthew to choose Path A or Path B — needs shem (multi-tenant commons canvas — no household analog)
- `features/auth/recovery/revocation-emergency-quorum.feature` — A person who is not Matthew's emergency contact cannot initiate a revocation — needs shem (multi-tenant commons canvas — no household analog)
- `features/content/landing-discovery.feature` — Start the journey carries the visitor into the onboarding learning path — needs shem (multi-tenant commons canvas — no household analog)
- `features/content/landing-discovery.feature` — Each epic story card resolves a living EPR head and opens its own address — needs shem (multi-tenant commons canvas — no household analog)
- `features/content/landing-discovery.feature` — A visitor who is unsure is carried to the who-are-you self-router — needs shem (multi-tenant commons canvas — no household analog)
- `features/content/landing-discovery.feature` — A curious visitor opens the evolution-of-trust explorable — needs shem (multi-tenant commons canvas — no household analog)
- `features/content/landing-discovery.feature` — A reference card whose body cannot be reached still renders a legible fallback — needs shem (multi-tenant commons canvas — no household analog)

### DEFECT-STALE (38 total)

- `features/auth/reach-commons.feature` — Anonymous reader can read the manifesto (earned commons reach) — AssertionError [ERR_ASSERTION]: anonymous read was not served: body: {"error":"Content not found: manifesto"}
- `features/auth/reach-commons.feature` — Anonymous reader is rejected for community-reach content (403 with requiredReach) — AssertionError [ERR_ASSERTION]: Expected values to be strictly equal:
- `features/auth/recovery/recovery-m5-defender-role-gate.feature` — Without role marker — coordinator rejects — AssertionError [ERR_ASSERTION]: Expected error mentioning "not a configured defender" but got: {"error":"Not Found","path":"/api/v1/account/specialist-revocation","hint":"Use WebSocket connection to /admin or /app/:port"}
- `features/auth/recovery/recovery-m5-defender-role-gate.feature` — With role marker — coordinator accepts — AssertionError [ERR_ASSERTION]: Expected 200 (ActionHash) but got 404
- `features/content/epr-content-addressing.feature` — Blob content loads via CID — AssertionError [ERR_ASSERTION]: Content manifesto not found
- `features/content/epr-content-addressing.feature` — EPR Head carries three-pillar metadata — AssertionError [ERR_ASSERTION]: Content manifesto not found in storage
- `features/content/stewardship-allocation.feature` — Value-scanner content has multiple stewards — AssertionError [ERR_ASSERTION]: No content found with tag "value-scanner" — allocator holds 8 allocation rows across 8 content items, 0 of them multi-steward. An empty content universe is a READER problem (reach/visibility or seeding), not an allocation problem.
- `features/content/stewardship-allocation.feature` — Stewardship reflects human affinities — AssertionError [ERR_ASSERTION]: No content found with tag "public-observer" — allocator holds 8 allocation rows across 8 content items, 0 of them multi-steward. An empty content universe is a READER problem (reach/visibility or seeding), not an allocation problem.
- `features/content/stewardship-allocation.feature` — Faith content stewarded by pastoral affinity — AssertionError [ERR_ASSERTION]: No multi-steward allocation among the 1 scanned "fct" items.
- `features/content/stewardship-allocation.feature` — Uncategorized content falls back to bootstrap steward — AssertionError [ERR_ASSERTION]: Expected type "curator", got "original_creator" for matthew-dowell

### UNCLASSIFIED (92 total)

- `features/auth/agency-pipeline-coherence.feature` — Matthew's pipeline shows hosted-steward as an in-between state — fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run)
- `features/auth/agency-pipeline-coherence.feature` — James's pipeline reflects no stewardship affordance — fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run)
- `features/auth/steward-login-portal-handoff.feature` — Matthew's login response carries his portal host URL — fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run)
- `features/auth/steward-login-portal-handoff.feature` — Doorway redirects Matthew to his portal host after auth — fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run)
- `features/content/contributor-presences.feature` — Learner sees contributor presences below the content — insufficient surface signal — no HTTP/metric/field surface named in step text or glue
- `features/content/landing-backing-claims.feature` — The landing surface is stewarded, and the steward is the one it declares — fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run)
- `features/content/relationship-idempotency.feature` — A spouse relationship authored by both parties is created once — insufficient surface signal — no HTTP/metric/field surface named in step text or glue
- `features/content/relationship-idempotency.feature` — Re-importing an account package does not error — insufficient surface signal — no HTTP/metric/field surface named in step text or glue
- `features/content/relationship-idempotency.feature` — Adam-Eve UNIQUE constraint does not fail the seed — insufficient surface signal — no HTTP/metric/field surface named in step text or glue
- `features/dataplane/contributor-presence-witnessed-holding.feature` — Every fixture resident exists as a commons-stewarded contributor presence — fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run)

## Surfaces the probe could not classify

Fetch failed or returned neither a recognizable ABSENT shape nor a usable sample —
these need a manual look, not a mechanical one:

- `/admin/cache/clear`
- `/admin/compute-events`
- `/admin/dev/portal-health`
- `/admin/oauth-clients`
- `/admin/seed/shard-manifest`
- `/api`
- `/api/peers`
- `/api/v1`
- `/api/v1/account/specialist-revocation`
- `/api/v1/blob`
- `/api/v1/commitments/facing/rea.`
- `/api/v1/households`
- `/api/v1/identity/heal`
- `/api/v1/peers`
- `/api/v1/peers/delivery.`
- `/api/v1/resilience`
- `/auth/callback.`
- `/auth/portal`
- `/db`
- `/db/content/manifesto`
- `/db/schemas/oauth_session.rs`
- `/health/metrics`
- `/p2p`
- `/p2p/mod.rs`
- `/p2p/status.connectedPeers`
- `/p2p/status.drain`
- `/p2p/status.drain.total`
- `/p2p/status.networkClass`
- `/p2p/status.pull`
- `/p2p/status.pull.pending`
- `/p2p/status.syncMode`
- `/p2p/status.syncPaused`
- `/sync`
- `/sync/projector.rs`

## Full census — every `@wip` or failed Act I/host scenario

| feature | scenario | bind | class | exists | absent | closes it |
|---|---|---|---|---|---|---|
| `features/auth/agency-pipeline-coherence.feature` | James's pipeline reflects no stewardship affordance | a | UNCLASSIFIED | `/auth/login` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/auth/agency-pipeline-coherence.feature` | Matthew's pipeline shows hosted-steward as an in-between state | a | UNCLASSIFIED | `/auth/account`, `/threshold/login`, `/auth` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/auth/conductor-pool-recovery.feature` | Doorway exposes orphan-mapping count for operators | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/auth/conductor-pool-recovery.feature` | Hosted human reconnects after the conductor pool composition changed | b | WIRE | `/admin` | — | write step glue (6 step(s) unbound, class b) |
| `features/auth/contributor-presence-claim-ceremony.feature` | A settlement is appealable through correcting events | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/auth/contributor-presence-claim-ceremony.feature` | The claim convening is convened and witnessed | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/auth/contributor-presence-claim-ceremony.feature` | The negotiated backfill is recorded as append-only events | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/auth/reach-commons.feature` | Anonymous reader can read the manifesto (earned commons reach) | a | DEFECT-STALE | `/db/content` | — | AssertionError [ERR_ASSERTION]: anonymous read was not served: body: {"error":"Content not found: manifesto"} |
| `features/auth/reach-commons.feature` | Anonymous reader is rejected for community-reach content (403 with requiredReach) | a | DEFECT-STALE | `/db/content` | — | AssertionError [ERR_ASSERTION]: Expected values to be strictly equal: |
| `features/auth/recovery/identity-key-lineage-recovery.feature` | A community-authorized key rotation preserves attribution, standing, and claims | c | WIRE | — | — | write step glue (14 step(s) unbound, class c) |
| `features/auth/recovery/identity-key-lineage-recovery.feature` | An un-authorized key rotation cannot capture the identity | c | WIRE | — | — | write step glue (10 step(s) unbound, class c) |
| `features/auth/recovery/recovery-m5-defender-role-gate.feature` | With role marker — coordinator accepts | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: Expected 200 (ActionHash) but got 404 |
| `features/auth/recovery/recovery-m5-defender-role-gate.feature` | Without role marker — coordinator rejects | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: Expected error mentioning "not a configured defender" but got: {"error":"Not Found","path":"/api/v1/account/specialist-revocation","hint":"Use WebSocket connection to /admin or /app/:port"} |
| `features/auth/recovery/recovery-m5-portal-host-discovery.feature` | Add a portal host | b | WIRE | `/api/v1/account/portal-hosts` | — | write step glue (1 step(s) unbound, class b) |
| `features/auth/recovery/recovery-m5-portal-host-discovery.feature` | Validator rejects http URL | b | WIRE | `/api/v1/account/portal-hosts` | — | write step glue (1 step(s) unbound, class b) |
| `features/auth/recovery/recovery-shamir-optional.feature` | Recovery never asks Matthew to choose Path A or Path B | c | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/auth/recovery/revocation-emergency-quorum.feature` | A person who is not Matthew's emergency contact cannot initiate a revocation | c | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/auth/recovery/revocation-self.feature` | Matthew cannot accidentally revoke his only remaining trusted key | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/auth/recovery/revocation-self.feature` | Matthew kills his stolen phone's key from his laptop | c | WIRE | — | — | write step glue (13 step(s) unbound, class c) |
| `features/auth/recovery/revocation-self.feature` | Matthew's other devices and relationships are unaffected | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/auth/recovery/revocation-self.feature` | The revoked key cannot sign new actions after revocation | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/auth/steward-login-portal-handoff.feature` | Doorway redirects Matthew to his portal host after auth | a | UNCLASSIFIED | `/api/v1/account/portal-hosts`, `/health`, `/auth/login`, `/admin/users`, `/threshold/login`, `/threshold` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/auth/steward-login-portal-handoff.feature` | Matthew's login response carries his portal host URL | a | UNCLASSIFIED | `/api/v1/account/portal-hosts`, `/health`, `/auth/login`, `/admin/users`, `/threshold/login`, `/auth` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/auth/visitor-boundaries.feature` | Registering as "visitor" phase is rejected — must choose hosted or higher | b | WIRE | `/auth/register` | — | write step glue (3 step(s) unbound, class b) |
| `features/auth/visitor-boundaries.feature` | Visitor can access commons content | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/auth/visitor-boundaries.feature` | Visitor can check doorway health | b | WIRE | `/health` | — | write step glue (3 step(s) unbound, class b) |
| `features/auth/visitor-boundaries.feature` | Visitor can see the landing page | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/auth/visitor-boundaries.feature` | Visitor cannot access network-reach content | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/auth/visitor-boundaries.feature` | Visitor cannot access private content | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/auth/visitor-boundaries.feature` | Visitor has no JWT token | b | WIRE | `/auth/me` | — | write step glue (2 step(s) unbound, class b) |
| `features/auth/visitor-boundaries.feature` | Visitor who registers as hosted gets an identity | b | WIRE | `/auth/register` | — | write step glue (4 step(s) unbound, class b) |
| `features/browser/doorway-dashboard-health.feature` | Dashboard handles missing orchestrator gracefully | a | IMPLEMENT-BOUNDED | `/threshold/dashboard` | `doorway_auth_token` | add: doorway_auth_token |
| `features/content/closure-posture.feature` | A card points at its vocabulary and never copies it | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/closure-posture.feature` | A closed-verdict axis must name the total function that decides it | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/closure-posture.feature` | A declared gap is a live obligation, not an excuse | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/closure-posture.feature` | Absence on the fact layer is unknown, never a negative fact | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/closure-posture.feature` | Absence on the verdict layer is a refusal, never a maybe | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/closure-posture.feature` | An open-verdict axis is not forced to name a classifier | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/closure-posture.feature` | Half a closure declaration is no declaration | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/closure-posture.feature` | One axis is known by one name | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/content-graph-resolver-constraints.feature` | A content-rooted graph never leaks contributor or identity nodes | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/content-graph-resolver-constraints.feature` | Computed discovery edges are never persisted (Category C) | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/content-graph-resolver-constraints.feature` | Incoming explicit edges survive the resolver graph source | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/content/content-graph-resolver-constraints.feature` | The panel degrades to explicit-only when the resolver graph endpoint fails | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/content/content-graph-resolver-constraints.feature` | The resolver clamps unbounded graph-query parameters at the HTTP boundary | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/contributor-presences.feature` | Learner sees contributor presences below the content | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/content/epistemic-standing.feature` | A threshold alone can never mint canon | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/epistemic-standing.feature` | Canon is conferred only by a referenced governance act | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/epistemic-standing.feature` | Contest dominates canonization | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/epistemic-standing.feature` | Contest routes to a referral, never to silence | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/epistemic-standing.feature` | Emergent truth is served honestly as emergent | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/epistemic-standing.feature` | Peer review accumulates into reviewed standing mechanically | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/content/epistemic-standing.feature` | Two peers folding the same reviews reach identical standing | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/epr-content-addressing.feature` | Blob content loads via CID | a | DEFECT-STALE | `/threshold/dashboard`, `/db/content` | — | AssertionError [ERR_ASSERTION]: Content manifesto not found |
| `features/content/epr-content-addressing.feature` | EPR Head carries three-pillar metadata | a | DEFECT-STALE | `/threshold/dashboard`, `/db/content` | — | AssertionError [ERR_ASSERTION]: Content manifesto not found in storage |
| `features/content/epr-content-addressing.feature` | EPR Head signature is verifiable end-to-end | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/content/epr-content-addressing.feature` | EPR link to a versioned-since-authored CID degrades gracefully | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/content/epr-content-addressing.feature` | EPR popover surfaces all three pillars when present | b | FIXTURE | `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/content/epr-content-addressing.feature` | Floor reaches bypass standing while commons demands the conservative fallback bar | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/content/epr-content-addressing.feature` | Following an EPR link transfers reading context to the destination | b | FIXTURE | `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/content/epr-content-addressing.feature` | Legacy standing-policy keys are inert — canonical vocabulary governs composition thresholds | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/content/landing-backing-claims.feature` | The landing surface is stewarded, and the steward is the one it declares | a | UNCLASSIFIED | `/db/content`, `/db/allocations` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/content/landing-discovery.feature` | A curious visitor opens the evolution-of-trust explorable | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/landing-discovery.feature` | A reference card whose body cannot be reached still renders a legible fallback | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/landing-discovery.feature` | A visitor who is unsure is carried to the who-are-you self-router | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/landing-discovery.feature` | Each epic story card resolves a living EPR head and opens its own address | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/landing-discovery.feature` | Start the journey carries the visitor into the onboarding learning path | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/omni-claims-links.feature` | An EPR with no recorded claims leaves the omnibar quiet | b | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/omni-claims-links.feature` | Expanding the omnibar reveals the claims backing this EPR | b | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/omni-claims-links.feature` | Nav links resolve client-side when the context island is silent | b | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/content/relationship-idempotency.feature` | A spouse relationship authored by both parties is created once | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/content/relationship-idempotency.feature` | Adam-Eve UNIQUE constraint does not fail the seed | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/content/relationship-idempotency.feature` | Re-importing an account package does not error | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/content/ssr_capability.feature` | an operator may opt in to authenticated SSR explicitly | c | WIRE | `/admin/capability` | — | write step glue (6 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | anonymous mode is always present even when override forgets it | c | WIRE | `/admin/capability` | — | write step glue (8 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | anonymous request renders against the always-present anonymous mode | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/content/ssr_capability.feature` | authenticated request renders when the doorway honors the auth mode | b | WIRE | — | — | write step glue (8 step(s) unbound, class b) |
| `features/content/ssr_capability.feature` | authenticated request to anonymous-only doorway falls back to CSR | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/content/ssr_capability.feature` | bundle absence falls back to CSR with bundle-not-loaded reason | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/content/ssr_capability.feature` | capacity overflow falls back to CSR rather than queueing | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | doorway exposes its derived capability at /admin/capability | b | WIRE | `/admin/capability` | — | write step glue (7 step(s) unbound, class b) |
| `features/content/ssr_capability.feature` | doorway returns null when no SSR claim is published | b | WIRE | `/admin/capability` | — | write step glue (6 step(s) unbound, class b) |
| `features/content/ssr_capability.feature` | operator override reduces (never inflates) the derived claim | c | WIRE | `/admin/capability` | — | write step glue (9 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | peer A inspecting peer B sees B's render capability | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | storage degrades honestly when doorway is unreachable | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | storage degrades honestly when DOORWAY_CAPABILITY_URL is unset | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | storage layers the capability into the peer-status view | c | WIRE | `/admin/capability` | — | write step glue (7 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | the derived claim does not implicitly offer authenticated SSR | c | WIRE | `/admin/capability` | — | write step glue (7 step(s) unbound, class c) |
| `features/content/ssr_capability.feature` | V8 fetch shim forwards the user's auth header to outbound storage fetches | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/content/stewardship-allocation.feature` | Faith content stewarded by pastoral affinity | a | DEFECT-STALE | `/admin`, `/db/content`, `/db/allocations` | — | AssertionError [ERR_ASSERTION]: No multi-steward allocation among the 1 scanned "fct" items. |
| `features/content/stewardship-allocation.feature` | No steward has exclusive ownership | a | DEFECT-STALE | `/admin` | — | AssertionError [ERR_ASSERTION]: Average stewards per content item is 1.00, expected > 1 |
| `features/content/stewardship-allocation.feature` | Stewardship reflects human affinities | a | DEFECT-STALE | `/admin`, `/db/content`, `/db/allocations` | — | AssertionError [ERR_ASSERTION]: No content found with tag "public-observer" — allocator holds 8 allocation rows across 8 content items, 0 of them multi-steward. An empty content universe is a READER problem (reach/visibility or seeding), not an allocation problem. |
| `features/content/stewardship-allocation.feature` | Uncategorized content falls back to bootstrap steward | a | DEFECT-STALE | `/admin` | — | AssertionError [ERR_ASSERTION]: Expected type "curator", got "original_creator" for matthew-dowell |
| `features/content/stewardship-allocation.feature` | Value-scanner content has multiple stewards | a | DEFECT-STALE | `/admin`, `/db/content`, `/db/allocations` | — | AssertionError [ERR_ASSERTION]: No content found with tag "value-scanner" — allocator holds 8 allocation rows across 8 content items, 0 of them multi-steward. An empty content universe is a READER problem (reach/visibility or seeding), not an allocation problem. |
| `features/dataplane/blob-replication.feature` | Declared reach maps to its replica target — no silent floor collapse | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/dataplane/content-sync.feature` | Author a content node and confirm it converges on a second peer within 30 s | a | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/dataplane/contributor-presence-witnessed-holding.feature` | Every fixture resident exists as a commons-stewarded contributor presence | a | UNCLASSIFIED | `/db/presences`, `/db/humans` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/dataplane/contributor-presence-witnessed-holding.feature` | No household-placed row is orphaned under the evolved invariant | a | UNCLASSIFIED | `/auth/contributor-presence-claim-ceremony.feature` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/dataplane/contributor-presence-witnessed-holding.feature` | Steward-authored witnessed ascriptions cover fixture-resident household residency | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/dataplane/contributor-presence-witnessed-holding.feature` | The holder relation counts fixture residents as witnessed, never verified | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/dataplane/doorway-catching-up-page.feature` | a cache-warm doorway renders the shell without a synchronous upstream fetch | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/dataplane/notary-authority.feature` | A declared canonical head rings the upgrade doorbell — every peer re-verifies and adopts within one sync window (RED — no doorbell) | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/dataplane/operator-commitment-gated-verbs.feature` | a peer serves its own runtime telemetry | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/dataplane/resilience-identity-coherence.feature` | No household-member human on alpha-A carries a fossil agentPubKey | a | FIXTURE | `/db/humans` | — | precondition missing: E2E_SHEM_HOST |
| `features/dataplane/resilience-identity-coherence.feature` | No household-placed human on alpha-A is missing its agent_pub_key | a | FIXTURE | `/db/humans`, `/db/p2p/conductor-diagnostics` | — | precondition missing: E2E_SHEM_HOST |
| `features/dataplane/resiliency-saga/05-co-steward-agreement.feature` | A stewardship pin with provide intent is active and caught up | a | DEFECT-STALE | `/api/v1/pins`, `/api/v1/commitments` | — | AssertionError [ERR_ASSERTION]: No active item pin references "elohim-host-landing" on alpha-A — no household has declared consent to steward it (5 pins listed). |
| `features/dataplane/resiliency-saga/08-capacity-reported.feature` | The cluster reports a non-negative stewarded-bytes aggregate | a | FIXTURE | `elohim_custodian_stewarded_bytes` | — | precondition missing: E2E_DOORWAY_METRICS_ALPHA, E2E_STORAGE_ALPHA, E2E_METRICS_, E2E_STORAGE_, E2E_SHEM_HOST |
| `features/dataplane/resiliency-saga/09-projectors-carry.feature` | The household resilience snapshot carries the co-steward's commitment-backed count | a | DEFECT-STALE | `/api/v1/resilience/elohim-host-landing/household`, `/db/humans`, `commitmentBackedReplication.commonsCommitments`, `commitmentBackedReplication.totalPledgedBytes` | — | AssertionError [ERR_ASSERTION]: Expected "commitmentBackedReplication.commonsCommitments" >= 1; got: 0 |
| `features/dataplane/resiliency-saga/10-card-tells-truth.feature` | Both doorways report the same non-zero stewarding count for elohim-host-landing | a | DEFECT-STALE | `/db/content` | — | AssertionError [ERR_ASSERTION]: stewardingCollectives on alpha-A is 0 — expected > 0 (the resilience card must not read zero) |
| `features/dataplane/resiliency-saga/10-card-tells-truth.feature` | The rendered resilience card shows the truth on elohim.host | a | UNCLASSIFIED | `/db/humans` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/dataplane/resiliency-saga/11-pull-queue-retires.feature` | a retired pin is re-admitted once a peer's inventory names its head_ref again | a | UNCLASSIFIED | `/api/v1/pins`, `elohim_acquisition_pin_retirements_total` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/dataplane/resiliency-saga/11-pull-queue-retires.feature` | an unsatisfiable pin retires once the peer-sized retry budget exhausts | a | DEFECT-STALE | `/api/v1/pins`, `/p2p/status`, `elohim_acquisition_pin_retirements_total` | — | AssertionError [ERR_ASSERTION]: this mesh's real acquisition fabric has 2 connected peer(s) at http://localhost:8090 — the chapter's worked example names a 6-peer alpha fabric; this is a household mesh with 3 total members (2 connected from any one member's own view). The retry budget the "probed on N distinct peers" step below actually checks is max(3, connectedPeers), so this mismatch alone does not prevent that step from passing. |
| `features/dataplane/resiliency-saga/11-pull-queue-retires.feature` | the household's pin for elohim-host-landing reaches caught-up within a bounded window | a | FIXTURE | `/api/v1/pins`, `/api/v1/commitments`, `elohim_acquisition_pins_retired` | — | precondition missing: E2E_DOORWAY_METRICS_ALPHA, E2E_STORAGE_ALPHA, E2E_METRICS_, E2E_STORAGE_, E2E_SHEM_HOST |
| `features/delivery/acquisition-pins.feature` | An error-class shard response releases its in-flight dispatch slot | a | UNCLASSIFIED | `/p2p/status` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/acquisition-pins.feature` | Retries of one item probe distinct peers, never one peer three times | a | UNCLASSIFIED | `/p2p/status` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/client-resilience.feature` | Cached app survives storage pod restart | b | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | Cached app works offline | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | Peer advertises delivery capabilities | b | WIRE | `/db/content`, `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/delivery/client-resilience.feature` | ready_content updates when extraction cache changes | b | WIRE | `/db/content`, `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/delivery/client-resilience.feature` | Service Worker registers in browser | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | Service Worker registers in Tauri WebView | b | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | SW extracts ZIP locally when peer only serves compressed | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | SW fetches individual files from a peer with warm extraction | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | SW invalidates cache when content is re-seeded | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | SW probes peer capability before fetching assets | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | WASM cache falls back to network when content not cached | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | WASM cache provides sub-millisecond content lookups | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/client-resilience.feature` | WASM cache unavailable degrades gracefully | a | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/content-addressing.feature` | CID URL serves same content without slug lookup | a | DEFECT-STALE | `/health`, `/db/content` | — | AssertionError [ERR_ASSERTION]: Header "X-Content-Address" value "sha256-ee5301c995967ff1e0a7b07a7f5e2de4f5f547bb3718547d33248453b77cb30f" does not look like a CID content address (expected bafkrei…) |
| `features/delivery/content-addressing.feature` | Re-seeded content with new CID invalidates old mapping | b | WIRE | `/health`, `/db/content` | — | write step glue (3 step(s) unbound, class b) |
| `features/delivery/content-addressing.feature` | Service worker caches by content address | b | WIRE | `/health`, `/db/content` | — | write step glue (3 step(s) unbound, class b) |
| `features/delivery/delivery-diagnostics.feature` | Capability summary explains what the network can actually deliver | b | WIRE | `/db/content` | — | write step glue (5 step(s) unbound, class b) |
| `features/delivery/delivery-diagnostics.feature` | Cold-cache HTML5 app loads via SW ZIP delivery without crashing storage | a | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/delivery-diagnostics.feature` | Operator can bypass Service Worker cache for diagnostics | a | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/delivery-diagnostics.feature` | Operator can query a peer's delivery capabilities | a | DEFECT-STALE | `/db/content`, `/health`, `/api/v1/peers/delivery` | — | AssertionError [ERR_ASSERTION]: no peer advertises "serves_extracted" — the delivery-peer row carries only [serves_compressed, serves_compressed]. A client choosing a peer to fetch individual files from cannot tell whether ANY peer can serve them file-by-file, so it must fall back to whole-blob download every time. |
| `features/delivery/delivery-diagnostics.feature` | Operator can see all peers and their delivery capabilities | a | DEFECT-STALE | `/db/content`, `/api/v1/peers/delivery` | — | AssertionError [ERR_ASSERTION]: this mesh advertises 2 delivery peer(s) from http://localhost:8090, not 3. The household triad gives any one member two connected peers; a scenario that names a different fabric size needs that fabric, not a rewritten expectation. |
| `features/delivery/delivery-diagnostics.feature` | Same-origin ingress enables SW interception for /apps/ requests | a | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/delivery-diagnostics.feature` | Service Worker reports delivery source for each request | a | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/delivery-diagnostics.feature` | With projection cache enabled, same load is absorbed | a | UNCLASSIFIED | `/db/content`, `/health` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/delivery-diagnostics.feature` | Without projection cache, browser load overwhelms storage | b | WIRE | `/db/content` | — | write step glue (1 step(s) unbound, class b) |
| `features/delivery/happ-coordinator-delivery.feature` | An operator can see which drift class the installer decided on | a | FIXTURE | `/health` | — | precondition missing: E2E_STORAGE_ |
| `features/delivery/happ-coordinator-delivery.feature` | Coordinator drift is healed by hot-swap without re-keying the agent | a | FIXTURE | `/db/p2p/conductor-diagnostics`, `/health`, `/db/content/elohim-host-landing` | — | precondition missing: E2E_STORAGE_ |
| `features/delivery/landing-page.feature` | An SSR render that activates no route component sheds to the bundle fallback | b | WIRE | — | — | write step glue (1 step(s) unbound, class b) |
| `features/delivery/landing-page.feature` | Landing page has proper SEO meta tags | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/delivery/landing-page.feature` | The deployed host serves the live epic-card deck | a | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/landing-page.feature` | The deployed host serves the redesigned hero and its start-the-journey call to action | a | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/landing-page.feature` | The landing page serves a hydratable surface, never a bundle-less empty shell | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/delivery/peer-mesh.feature` | Client resolves multiple delivery peers via EPR | b | WIRE | `/db/content` | — | write step glue (4 step(s) unbound, class b) |
| `features/delivery/peer-mesh.feature` | Doorway caching authority is backed by EPR agreement | b | WIRE | `/db/content` | — | write step glue (4 step(s) unbound, class b) |
| `features/delivery/peer-mesh.feature` | Doorway is not required when peers are available | b | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/peer-mesh.feature` | Fallback chain degrades gracefully | b | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/peer-mesh.feature` | LAN delivery uses direct HTTP between Tauri nodes | b | WIRE | `/db/content` | — | write step glue (5 step(s) unbound, class b) |
| `features/delivery/peer-mesh.feature` | Old peers handle QueryDelivery gracefully | b | WIRE | `/db/content` | — | write step glue (4 step(s) unbound, class b) |
| `features/delivery/peer-mesh.feature` | Peer scoring prefers LAN over doorway over remote | b | WIRE | `/db/content` | — | write step glue (3 step(s) unbound, class b) |
| `features/delivery/peer-mesh.feature` | QueryDelivery protocol returns delivery info | b | WIRE | `/db/content` | — | write step glue (5 step(s) unbound, class b) |
| `features/delivery/peer-mesh.feature` | Tauri node serves app files to household peer over LAN | b | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/peer-mesh.feature` | When all extraction peers fail, client extracts ZIP | b | FIXTURE | `/db/content` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/protocol-omnibar.feature` | Clicking the pill expands provenance details | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | Collapsing the expanded omnibar | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | Content viewer focused mode shows omnibar pill | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | Doorway response includes provenance headers | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/delivery/protocol-omnibar.feature` | EPR address is copyable | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | HTML5 app content renders as full-page iframe | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | Inspect EPR navigates to governance hub | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/delivery/protocol-omnibar.feature` | Markdown content renders as full page | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | Omnibar pill shows protocol-delivered status | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | Report action available from omnibar | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/delivery/protocol-omnibar.feature` | Unknown content shows 404 | c | WIRE | — | — | write step glue (3 step(s) unbound, class c) |
| `features/delivery/spa-bundle-delivery.feature` | /apps/ still serves html5-apps and is not intercepted by the root app | a | UNCLASSIFIED | `/health/startup`, `/db/content`, `/health` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | /health/startup returns live startup status as JSON | a | UNCLASSIFIED | `/health`, `/health/startup` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | /threshold is still accessible as the operator dashboard | a | UNCLASSIFIED | `/health/startup`, `/threshold` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | API routes are not caught by the root app catch-all | a | UNCLASSIFIED | `/health/startup`, `/db/content/evolution-of-trust`, `/threshold` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | Bootstrap page auto-navigates when rootApp becomes ready | a | FIXTURE | `/health/startup`, `/health` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/spa-bundle-delivery.feature` | Bootstrap page falls back to reload timer when /health/startup is unreachable | a | FIXTURE | `/health/startup`, `/health` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/spa-bundle-delivery.feature` | Bootstrap page is shown when SPA blob is not yet extracted | a | FIXTURE | `/health/startup` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/spa-bundle-delivery.feature` | CI uploading a new SPA blob causes doorway to serve the updated version | a | UNCLASSIFIED | `/admin` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | index.html is served with no-cache headers | a | UNCLASSIFIED | `/health/startup` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | Old SPA cached files are evicted when blobHash changes | a | UNCLASSIFIED | `/health/startup` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | Projection cache self-heals after storage pod restart | a | UNCLASSIFIED | `/health/startup`, `/health` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | Root app becomes available after warmup resolves the slug | a | UNCLASSIFIED | `/health/startup`, `/health` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | Root app serves index.html at / | a | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/spa-bundle-delivery.feature` | SPA routes fall back to index.html so Angular handles routing | a | UNCLASSIFIED | `/health/startup`, `/health` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | Static assets are served with immutable cache headers | a | UNCLASSIFIED | `/health/startup` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/spa-bundle-delivery.feature` | Without ROOT_APP_SLUG, / redirects to /threshold | a | FIXTURE | `/threshold` | — | precondition missing: E2E_DEVICE_MODE |
| `features/delivery/spa-bundle-delivery.feature` | Without ROOT_APP_SLUG, an unknown path returns 404 (not the bootstrap page) | a | UNCLASSIFIED | `/admin`, `/health/startup` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/delivery/transport-perf.feature` | A 50-MB lesson video downloads at near-parity regardless of stack | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/delivery/transport-perf.feature` | A connection pool that reuses iroh QUIC connections doesn't silently hang | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/delivery/transport-perf.feature` | A learner on a UDP-blocked school network still gets responsive learning | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/delivery/transport-perf.feature` | Two students co-editing a study guide don't feel the keystrokes lag | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/delivery/transport-perf.feature` | When my signature outgrows the protocol cap, identity onboarding fails clearly | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/delivery/web2-absorption.feature` | Cache entries include EPR agreement reference | b | FIXTURE | `/db/content`, `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/deployment/compute-commitment-bounds.feature` | Elohim accepts what substrate would have denied via exception | c | STRUCTURAL | — | `elohim_signature` | needs shem (multi-tenant commons canvas — no household analog) |
| `features/deployment/compute-commitment-bounds.feature` | Elohim adds discernment without invalidating substrate verdict | c | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/deployment/compute-commitment-bounds.feature` | Standing agreement fires deterministically without elohim | c | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/deployment/compute-commitment-bounds.feature` | Substrate denies a compute request that exceeds bounds | c | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/deployment/compute-commitment-bounds.feature` | Substrate handles all three trigger kinds without elohim | c | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/deployment/compute-commitment-bounds.feature` | Substrate negotiates a compute request without elohim | c | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/deployment/doorway-self-registration.feature` | Matthew sees his own node in the admin dashboard | a | DEFECT-STALE | `/admin` | — | Error: GET /admin/nodes returned 503: { |
| `features/deployment/doorway-self-registration.feature` | Matthew's node reports real hardware capacity | a | DEFECT-STALE | `/admin` | — | Error: GET /admin/nodes returned 503: { |
| `features/deployment/hub-topology.feature` | Church basement hub carries a hundred spokes through one Tier 3 | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | CollectiveHub adds spokes through governance consent | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Extended family with two hubs provides mutual relational backup | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | HouseholdHub adds spokes through trust without explicit consent | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Hub hardware must be physically and operationally accessible to its stewards | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Hub portfolio includes HouseholdHub and CollectiveHub implementations | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Hub remains usable through doorway projection when its Tier 3 goes offline | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Multi-generational household carries a hosted-only grandmother | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Phone-only solo household participates without owning a hub | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Refugee camp shared hub operates under constrained bandwidth | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Stewards can evict hardware that refuses access entirely | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Stewards can quarantine hardware that becomes inaccessible to them | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/deployment/hub-topology.feature` | Young family with one Tier 3 is the canonical Stage-4 household | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | Consolidated-pattern records reference a real manifest file | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | Every deployed human names a real device archetype | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | Every deployed human resolves in the humans registry | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | Every nodeTypes entry is in the allowed cluster vocabulary | b | WIRE | — | — | write step glue (1 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | humanId matches the convention "human-<humanLabel>" | b | WIRE | — | — | write step glue (1 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | Legacy-pattern records reference a real template file | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | Level-5 humans declare a level-5-capable archetype | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | Pod resources fit within the device archetype envelope | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/deployment/human-device-mapping.feature` | The six protocol humans are all represented in the deployment registry | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/ingress-body-size-budget.feature` | Different peer archetypes declare different body-size budgets | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/deployment/node-resource-tunables.feature` | A zero or garbage SQLite pool size falls back to the default, never a zero-size pool | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/deployment/node-resource-tunables.feature` | Breaches of these limits aggregate into the resource-shape report | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/deployment/node-resource-tunables.feature` | Each node resource limit is a named per-node tunable with a safe default | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/deployment/node-resource-tunables.feature` | The conductor arc factor is bounded to {0,1} — fractional is refused, not silently clamped | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/deployment/p2p-validation.feature` | P2P status endpoint exposes sync_paused state | a | DEFECT-STALE | `/health` | — | AssertionError [ERR_ASSERTION]: doorway /health p2p.syncPaused is undefined (type undefined) — expected a boolean. FINDING (2026-08-21): elohim-storage DOES expose sync_paused (camelCase syncPaused) on its direct GET /p2p/status (P2PStatusInfo), but the doorway-facing GET /health surface (doorway/doorway-service/src/routes/health.rs HealthP2P) never proxies it — an operator or agent watching only the doorway health endpoint cannot see backpressure state today. |
| `features/deployment/p2p-validation.feature` | Storage pauses P2P sync during account import | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: elohim-storage on doorway "alpha" /p2p/status.connectedPeers is 2, expected >= 3 |
| `features/deployment/p2p-validation.feature` | Storage pauses P2P sync during bulk content creation | a | DEFECT-STALE | `/db/content/bulk` | — | AssertionError [ERR_ASSERTION]: elohim-storage on doorway "alpha" /p2p/status.connectedPeers is 2, expected >= 3 |
| `features/deployment/p2p-validation.feature` | Sync auto-suppressed while drain backlog is large | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: elohim-storage on doorway "alpha" /p2p/status.connectedPeers is 2, expected >= 5 |
| `features/deployment/p2p-validation.feature` | Sync resumes even if bulk write fails | a | DEFECT-STALE | `/db/content/bulk` | — | AssertionError [ERR_ASSERTION]: elohim-storage on doorway "alpha" /p2p/status.connectedPeers is 2, expected >= 3 |
| `features/deployment/peer-diversity.feature` | Biometric fob provides strongest identity attestation | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Carrier-grade NAT devices must use relay | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Cliff-degradation devices need proactive replication | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Device portfolio covers the full capability gradient | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Each archetype declares a body-size budget for its ingress | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/deployment/peer-diversity.feature` | Environmental sensor provides place-based attestation | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Family node does not need backpressure for normal imports | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/deployment/peer-diversity.feature` | Family node is primary stewardship backbone | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Family node reports full health surface | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | K8s pod pauses sync during account import | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/deployment/peer-diversity.feature` | Modular devices schedule their own maintenance | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Offline-first devices stream to paired nodes | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Phone cannot accept stewardship requests | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Phone pauses sync during bulk content download | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/deployment/peer-diversity.feature` | Phone reports minimal health surface | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/peer-diversity.feature` | Raspberry Pi can steward modest content volumes | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/seeder-registry-coherence.feature` | Seeder is idempotent across reruns | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/sync-control.feature` | Each archetype has a sensible default sync mode | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/sync-control.feature` | Operator pauses sync explicitly | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: cannot establish precondition "the peer's sync mode is sync" — POST /p2p/sync-mode returned 404 (Not Found). FINDING (2026-08-21): elohim-storage has no sync-mode control surface. P2PStatusInfo (elohim/elohim-storage/src/p2p/mod.rs) carries no syncMode/networkClass field — only the AUTOMATIC backpressure flag sync_paused (camelCase syncPaused). No handler for POST /p2p/sync-mode, POST /p2p/network-class, or GET /p2p/sync-mode/history exists in elohim-storage/src/http.rs. The endpoint names probed here follow the /p2p/status naming convention as the most plausible target — this step fails with the REAL HTTP response from that (currently 404) route rather than fabricating a pass. |
| `features/deployment/sync-control.feature` | Operator resumes sync after pause | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: cannot establish precondition "the peer's sync mode is paused" — POST /p2p/sync-mode returned 404 (Not Found). FINDING (2026-08-21): elohim-storage has no sync-mode control surface. P2PStatusInfo (elohim/elohim-storage/src/p2p/mod.rs) carries no syncMode/networkClass field — only the AUTOMATIC backpressure flag sync_paused (camelCase syncPaused). No handler for POST /p2p/sync-mode, POST /p2p/network-class, or GET /p2p/sync-mode/history exists in elohim-storage/src/http.rs. The endpoint names probed here follow the /p2p/status naming convention as the most plausible target — this step fails with the REAL HTTP response from that (currently 404) route rather than fabricating a pass. |
| `features/deployment/sync-control.feature` | Sync mode transitions are logged for auditability | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: cannot establish precondition — GET /p2p/sync-mode/history returned 404 (Not Found). FINDING (2026-08-21): elohim-storage has no sync-mode control surface. P2PStatusInfo (elohim/elohim-storage/src/p2p/mod.rs) carries no syncMode/networkClass field — only the AUTOMATIC backpressure flag sync_paused (camelCase syncPaused). No handler for POST /p2p/sync-mode, POST /p2p/network-class, or GET /p2p/sync-mode/history exists in elohim-storage/src/http.rs. The endpoint names probed here follow the /p2p/status naming convention as the most plausible target — this step fails with the REAL HTTP response from that (currently 404) route rather than fabricating a pass. |
| `features/deployment/sync-control.feature` | Sync state is visible in the operator dashboard | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/deployment/sync-control.feature` | Wifi-only mode pauses sync when cellular is the active network | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: cannot establish precondition device "2019 Android Phone" with sync mode "wifi-only" — POST /p2p/sync-mode returned 404. FINDING (2026-08-21): elohim-storage has no sync-mode control surface. P2PStatusInfo (elohim/elohim-storage/src/p2p/mod.rs) carries no syncMode/networkClass field — only the AUTOMATIC backpressure flag sync_paused (camelCase syncPaused). No handler for POST /p2p/sync-mode, POST /p2p/network-class, or GET /p2p/sync-mode/history exists in elohim-storage/src/http.rs. The endpoint names probed here follow the /p2p/status naming convention as the most plausible target — this step fails with the REAL HTTP response from that (currently 404) route rather than fabricating a pass. |
| `features/deployment/sync-control.feature` | Wifi-only mode resumes sync when device joins wifi | a | DEFECT-STALE | `/p2p/status` | — | AssertionError [ERR_ASSERTION]: cannot establish precondition device "2019 Android Phone" with sync mode "wifi-only" — POST /p2p/sync-mode returned 404. FINDING (2026-08-21): elohim-storage has no sync-mode control surface. P2PStatusInfo (elohim/elohim-storage/src/p2p/mod.rs) carries no syncMode/networkClass field — only the AUTOMATIC backpressure flag sync_paused (camelCase syncPaused). No handler for POST /p2p/sync-mode, POST /p2p/network-class, or GET /p2p/sync-mode/history exists in elohim-storage/src/http.rs. The endpoint names probed here follow the /p2p/status naming convention as the most plausible target — this step fails with the REAL HTTP response from that (currently 404) route rather than fabricating a pass. |
| `features/deployment/sync-control.feature` | Without sync mode control, mobile device burns cellular data | a | DEFECT-STALE | — | — | AssertionError [ERR_ASSERTION]: peer /p2p/status.connectedPeers is 2, expected >= 5 — the regression this scenario documents (unconditional cellular gossip) needs a REAL multi-peer gossip fabric to be meaningful; this mesh is not connected enough. |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A commit's co-author roster reaches the produce event, sorted and additive | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A contested identity escalates the next acceptance to Audit | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A governance decision stamps who was acting when it was asked for | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A malformed identity is refused, never silently replaced by the author | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A missing package is honest absence, never a placeholder address | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A persona switch supersedes without rewriting history | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A run note is attributed to the claimed identity with the steward attached | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | A tampered identity log reads as unclaimed, never as the tamperer's identity | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | An unclaimed session falls through to author attribution with a notice | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | Ratification at dev-merge accepts the session's claims at Witness tier | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | Re-projection never records the same act twice, even when its vocabulary grew | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/devflow/agent-identity-claim-and-acceptance.feature` | Registering a claim records who is acting, dated by the tree | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/devflow/developer-valueflow-projection.feature` | A fulfilled commitment leaves the frontier | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/devflow/developer-valueflow-projection.feature` | A tampered sidecar line fails as an integrity error, never silent drift | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/devflow/developer-valueflow-projection.feature` | Projecting the repository derives flow records for every recipe stage | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/devflow/developer-valueflow-projection.feature` | Re-projecting the repository is idempotent | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/devflow/developer-valueflow-projection.feature` | Walking a spec shows lineage to its cited seeds and its produce event | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/devflow/developer-valueflow-projection.feature` | Walking forward from a spec surfaces its unfulfilled frontier | c | WIRE | — | — | write step glue (9 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A correction against work that cannot be found is refused, not filed loose | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A correction annotates an open commitment and never discharges it | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A correction written in one session is still there for the next one | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A register already over the fence is shown as exceeded, not renormalised | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A stale equilibrium reading says so instead of reporting an old rate | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A stock finishing work at least as fast as it takes it on passes | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A stock taking on work faster than it finishes it fails the check | c | WIRE | — | — | write step glue (9 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | A window with nothing to measure refuses rather than reporting equilibrium | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | An input the block cannot read costs one line, never the turn | c | WIRE | — | — | write step glue (9 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | Every turn opens with the fence, the frontier, and the newest correction | c | WIRE | — | — | write step glue (14 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | Finishing a promise drains the stock and never fills it | c | WIRE | — | — | write step glue (9 step(s) unbound, class c) |
| `features/devflow/run-plane.feature` | Writing the same correction twice at one commit leaves one record | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/doorway/native-epr-projection.feature` | An alias grant colliding with a reserved prefix is rejected at create time | a | UNCLASSIFIED | `/api/v1/commitments` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/doorway/native-epr-projection.feature` | Federation — same EPR projected on second doorway serves same content | a | FIXTURE | — | — | precondition missing: E2E_DOORWAY_PRIMARY, E2E_DOORWAY_HOSTED |
| `features/doorway/peer-conductor-connection-resilience.feature` | Auth-rejected peer conductor is retried with exponential backoff | a | STRUCTURAL | `/health`, `/db/content` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/doorway/peer-conductor-connection-resilience.feature` | Reconnect churn is visible to operators | a | STRUCTURAL | `/status.json` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/doorway/peer-conductor-connection-resilience.feature` | Reconnect cycles do not leak connection tasks | a | STRUCTURAL | `/status.json` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/doorway/peer-conductor-connection-resilience.feature` | Unstable sessions do not reset the backoff clock | a | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/doorway/self-healing-flow-control.feature` | An operator or agent can read the unified self-healing model | a | UNCLASSIFIED | `/health`, `/admin/self-healing` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/doorway/self-healing-flow-control.feature` | The storage breaker recovers after its cooldown | a | UNCLASSIFIED | `/admin/self-healing` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/elohim-core/chrome-preferences.feature` | Chrome follows the theme toggle | b | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/elohim-core/chrome-preferences.feature` | Dark-mode chrome is readable | b | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/elohim-core/chrome-preferences.feature` | Switching to Hebrew flips the chrome to RTL and persists | b | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/elohim-core/chrome-preferences.feature` | The lamad viewport carries no UA frame | b | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/elohim-core/chrome-preferences.feature` | Theme choice persists across the app boundary | b | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/elohim/compute-coordination.feature` | Compute cost recorded as economic event | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/elohim/compute-coordination.feature` | Request deferred when budget exhausted | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/elohim/content-reach-negotiation.feature` | Author accepts reduced reach | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Author re-negotiates reach with explanation | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Commons content gets full birth context | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Content carries birth context after creation | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Content without birth context is flagged | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | High-trust author gets lightweight pipeline | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Moderate-trust author gets standard pipeline | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | New author gets full pipeline | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Private content gets minimal birth context | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Recontextualization is detectable | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Reply carries chain of provenance | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Trust earned not purchased | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Trusted author receives recommended reach matching request | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | Untrusted author has reach reduced with explanation | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/content-reach-negotiation.feature` | User can inspect full transparency report | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | All peers are intermittent with no always-on nodes | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Debug-level health requires a compute:debug attestation | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Diagnostic attestation expires and access reverts | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Elohim agent incorporates network posture into resilience assessment | b | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/elohim/network-health-posture.feature` | Elohim agent requests diagnostic attestation to investigate degradation | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Info-level health is available without attestation | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Introspection without attestation defaults to info level | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Network posture degrades when always-on peers go offline | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Network posture informs compute routing decisions | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Network posture reflects compute exhaustion across the network | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Network posture shows storage pressure across the network | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Node diagnostics are reachable by URL regardless of navigation visibility | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Operator grants diagnostic attestation to a peer | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Operator revokes diagnostic attestation | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Operator sees network posture summary from neighbor table | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Peer rejects introspection request above granted attestation level | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Posture includes peer diversity health | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Same DetailLevel filtering serves both access models | b | WIRE | `/health` | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Single-node network has valid but minimal posture | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Stability view on a single node degrades honestly, never fabricating doorway-role state | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Stability view served through a doorway is the full composed self-healing model | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/elohim/network-health-posture.feature` | Trace-level health requires a compute:trace attestation | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/federation/cross-doorway-content.feature` | Content created on alpha is discoverable from staging | a | FIXTURE | — | — | precondition missing: E2E_DOORWAY_STAGING |
| `features/federation/cross-doorway-content.feature` | Content created on staging is discoverable from alpha | a | FIXTURE | — | — | precondition missing: E2E_DOORWAY_STAGING |
| `features/federation/doorway-pool-degrade.feature` | Empty everywhere is a genuine empty state, not a silent wipe | a | FIXTURE | `/db/rea_commitments`, `/api/v1/federation/coherence` | — | precondition missing: E2E_EPR_REFRESH_WINDOW_MS |
| `features/federation/doorway-pool-degrade.feature` | Router populates from a pool peer when the primary returns no rows | a | FIXTURE | `/db/rea_commitments`, `/api/v1/federation/coherence` | — | precondition missing: E2E_EPR_REFRESH_WINDOW_MS |
| `features/federation/doorway-pool-degrade.feature` | The apex front door serves through the degraded primary | a | FIXTURE | `/api/v1/federation/coherence`, `/db/rea_commitments` | — | precondition missing: E2E_DOORWAY_APEX |
| `features/federation/epr-cross-peer-resolution.feature` | Attestation-gated content requires prerequisite mastery | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/epr-cross-peer-resolution.feature` | Community-reach guide accessible only to consented collective members | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/epr-cross-peer-resolution.feature` | Cross-peer fetch surfaces transient peer-offline as a soft state | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/epr-cross-peer-resolution.feature` | Identity binding allows cross-peer fetches to attribute reach correctly | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/epr-cross-peer-resolution.feature` | Policy ceiling blocks content above the device's reach level max | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/epr-cross-peer-resolution.feature` | Recognition distributes proportionally to stewards on P2P delivery | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/epr-cross-peer-resolution.feature` | Steward sees recognition land for content delivered cross-peer | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/epr-cross-peer-resolution.feature` | Trusted-reach content requires standing relationship with steward | b | STRUCTURAL | `/p2p/status` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/federation/membrane-rate-limit.feature` | A disposable source exceeding the rate threshold is challenged then denied | b | WIRE | — | — | write step glue (8 step(s) unbound, class b) |
| `features/federation/membrane-rate-limit.feature` | A normal page load carries no membrane header and is counted as allow | b | WIRE | `doorway_membrane_verdict_total` | — | write step glue (4 step(s) unbound, class b) |
| `features/federation/membrane-rate-limit.feature` | Membrane metrics render at boot and move under a challenge probe | b | WIRE | `doorway_membrane_bans_active` | — | write step glue (7 step(s) unbound, class b) |
| `features/federation/peer-advertisement.feature` | Cache eviction removes content from announcement | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Cache warming updates readiness in announcement | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Compute budget exhaustion is reflected in announcement | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Doorway advertises its profile alongside storage peers | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Heterogeneous network handles mixed availability | a | UNCLASSIFIED | `/db/humans` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/federation/peer-advertisement.feature` | Home node advertises always-on family-serving profile | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Laptop peer advertises intermittent profile | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Network node advertises public-serving profile | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Network shows diverse peer profiles simultaneously | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Peer broadcasts capacity every 30 seconds | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Peer coming online announces immediately | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Peer going offline is detected by absence of heartbeats | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Receiving peer builds neighbor table from announcements | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Stale announcements are evicted from neighbor table | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-advertisement.feature` | Storage filling updates capacity announcement | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/federation/peer-loss-failover.feature` | A returning peer re-syncs without operator help | a | FIXTURE | `/p2p/status`, `/api/v1/diagnostics/inventory-parity` | — | precondition missing: E2E_PEER_RESYNC_WINDOW_MS, E2E_CONNECTED_PEERS_FLOOR |
| `features/federation/peer-loss-failover.feature` | A single device still functions without the mesh | a | FIXTURE | `/p2p/status` | — | precondition missing: E2E_CONNECTED_PEERS_FLOOR, E2E_PEER_DEGRADE_WINDOW_MS |
| `features/federation/peer-loss-failover.feature` | Reads still serve while one household peer is down | a | FIXTURE | `/p2p/status` | — | precondition missing: E2E_CONNECTED_PEERS_FLOOR |
| `features/federation/peer-recovery.feature` | A wiped device recovers its stewarded content from the mesh | a | DEFECT-STALE | `/health`, `/p2p/status`, `/api/v1/commitments`, `/api/v1/economic-events` | — | Error: GET http://localhost:8888/db/content/manifesto returned 404: {"error":"Content not found: manifesto"} |
| `features/federation/peer-recovery.feature` | A wiped peer's commitment projection reconciles from its own conductor | a | DEFECT-STALE | `/db/rea_commitments`, `/admin`, `/api/v1/commitments`, `/p2p/status` | — | AssertionError [ERR_ASSERTION]: Jessica's projection already holds 19 custody-blob rows. This drill needs a wiped projection, and elohim-storage has no verb to clear one (no DELETE on /db/rea_commitments, no /admin projection-reset) — establish it out of band or add the verb |
| `features/lamad/attention-analytics.feature` | Bounce view does not generate an economic event | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/lamad/attention-analytics.feature` | Content view generates an economic event after dwell threshold | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/lamad/attention-analytics.feature` | Duplicate views within session are deduplicated | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/lamad/attention-analytics.feature` | Learner sees their attention flow | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/lamad/attention-analytics.feature` | No external analytics scripts loaded | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/lamad/attention-analytics.feature` | Session end event on tab close | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/lamad/attention-analytics.feature` | Session start event on app initialization | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/lamad/attention-analytics.feature` | Steward sees content engagement metrics | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/lamad/deep-link-delivery.feature` | A shared markdown EPR link renders FORMATTED content, not the raw fallback | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/lamad/deep-link-delivery.feature` | View Resource Details crosses the bundle boundary | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/lamad/intimate-reach-household.feature` | A household member outside the couple scope cannot read couple content | a | STRUCTURAL | `/threshold/dashboard`, `/db/content`, `/auth/login`, `/db/human-relationships`, `/db/human-relationships.` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/lamad/intimate-reach-household.feature` | James stewards the couple's resiliency bytes without read access | b | STRUCTURAL | `/threshold/dashboard`, `/db/content`, `/auth/login`, `/db/human-relationships.` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/lamad/intimate-reach-household.feature` | James's steward view shows opaque stewarded bytes, never content | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/lamad/intimate-reach-household.feature` | Jessica reads the couple's love map; anonymous visitors cannot | a | STRUCTURAL | `/threshold/dashboard`, `/db/content`, `/auth/login`, `/db/human-relationships`, `/db/human-relationships.` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/lamad/intimate-reach-household.feature` | Matthew feels the intimate boundary on the content card | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/lamad/intimate-reach-household.feature` | Replicated shards are encrypted - senseless bits in the steward's pantry | b | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/lamad/know-thyself-discovery.feature` | First discovery assessment earns a milestone attestation | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/know-thyself-discovery.feature` | Jessica completes the Attachment Style assessment | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/know-thyself-discovery.feature` | Terrance completes the Values Hierarchy assessment | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/learning-journey.feature` | Earning Affinity through Navigation | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/learning-journey.feature` | Restricted Access (Attestations) | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/learning-journey.feature` | Starting a Journey | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/love-map-negotiation.feature` | Jessica accepts and emergent path is generated | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/lamad/love-map-negotiation.feature` | Love map path is invisible to non-participants | b | WIRE | `/threshold/dashboard` | — | write step glue (1 step(s) unbound, class b) |
| `features/lamad/love-map-negotiation.feature` | Love map requires intimate consent level | b | WIRE | `/threshold/dashboard` | — | write step glue (2 step(s) unbound, class b) |
| `features/lamad/love-map-negotiation.feature` | Love map requires mutual attestation | b | WIRE | `/threshold/dashboard` | — | write step glue (3 step(s) unbound, class b) |
| `features/lamad/love-map-negotiation.feature` | Matthew and Jessica can follow the love map path | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/love-map-negotiation.feature` | Matthew proposes a love map to Jessica | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/lamad/love-map-negotiation.feature` | Path shows mutual teaching structure | a | UNCLASSIFIED | `/threshold/dashboard` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/lamad/love-map-negotiation.feature` | Revoking attestation removes love map access | b | WIRE | `/threshold/dashboard` | — | write step glue (2 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Attestation gates are not bypassed by mastery | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Bloom level "remember" does NOT unlock steps | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Clicking "View Recommended Path" navigates to the path | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Dismissing a recommendation removes it from both surfaces | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Elohim recommends a path after discovery completion | b | WIRE | `/threshold/dashboard` | — | write step glue (4 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Failed quiz surfaces prerequisite content from content graph | b | WIRE | `/threshold/dashboard` | — | write step glue (7 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | New learner with mastery on scattered content | b | WIRE | `/threshold/dashboard` | — | write step glue (7 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Passing the gate clears recommendations for that section | b | WIRE | `/threshold/dashboard` | — | write step glue (3 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Pre-assessment skip-ahead unlocks section steps | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Prior mastery unlocks steps beyond the sequential window | b | WIRE | `/threshold/dashboard` | — | write step glue (6 step(s) unbound, class b) |
| `features/lamad/path-adaptation.feature` | Sequential guidance still shown for mastery-unlocked steps | b | WIRE | `/threshold/dashboard` | — | write step glue (5 step(s) unbound, class b) |
| `features/peer-oauth-portal/hosted-login.feature` | Wrong password preserves trust-indicator chrome | a | UNCLASSIFIED | `/db/rea_commitments` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/peer-oauth-portal/peer-conductor-login.feature` | Sign-in via doorway-routed peer-conductor | a | UNCLASSIFIED | `/auth/me`, `/auth` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/peer-oauth-portal/peer-conductor-login.feature` | Sign-in via Tauri direct (no doorway) | a | UNCLASSIFIED | `/auth/me`, `/auth` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/peer-oauth-portal/rp-consent.feature` | User approves a per-claim consent | a | IMPLEMENT-BOUNDED | `/auth/login` | `elohim_session` | add: elohim_session |
| `features/peer-oauth-portal/rp-consent.feature` | User declines consent | a | IMPLEMENT-BOUNDED | — | `elohim_session` | add: elohim_session |
| `features/protocol/landing-page-dogfood.feature` | The protocol-signal badge renders on the landing page | a | FIXTURE | — | — | precondition missing: E2E_DEVICE_MODE |
| `features/protocol/protocol-omni.feature` | The EPR nav-context endpoint serves a navigation projection | a | UNCLASSIFIED | `/api/v1/epr/elohim-host-landing/nav-context`, `cid`, `partOf`, `related`, `derivedFrom` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/qahal/collective-governance.feature` | Anonymous voting | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Block a proposal with justification | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Bridging statement surfaces common ground | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Challenge list shows SLA countdown | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Challenge response sets referenceable precedent | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Change a vote | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Community opinion clustering reveals distinct groups | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Community uses ranked-choice to pick a curriculum path | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Consensus strength indicator reflects agreement level | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Consent round with escalation on block | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Content reaches settled status through sustained consensus | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Controversy detected on divisive content | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Create a proposal | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Dot-voting allocates limited attention across proposals | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Elohim builds governance disposition from voting history | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Elohim responds to challenge within SLA | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Elohim selects feedback mechanism based on content context | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Elohim votes as proxy when human hasn't engaged | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Formal governance ballot renders via Psephos for ranked-choice proposal | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Governance disposition reflects consistent values | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Governance participation generates stewardship recognition | c | WIRE | — | — | write step glue (4 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Human overrides elohim proxy vote | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Human reviews proxy vote and confirms | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Learner appeals rejected challenge | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Learner challenges inaccurate content | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Learner contributes statement to sensemaking | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Learner provides graduated feedback on discussion content | c | WIRE | — | — | write step glue (9 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Learner reacts to learning content with emotional response | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Learner sees context menu only on constitutional content | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Sensemaking triggers bracket synthesis from bridging statements | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Sensemaking view accessible from gateway badge | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Signal accumulation triggers sensemaking readiness | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Signal aggregate shows community feedback distribution | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | SLA overdue triggers visual warning | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Stewards score competing content revisions | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/qahal/collective-governance.feature` | Vote on a proposal | c | WIRE | — | — | write step glue (5 step(s) unbound, class c) |
| `features/qahal/household-formation.feature` | All three members are affirmed participants | a | DEFECT-STALE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/db/collectives` | — | AssertionError [ERR_ASSERTION]: triad member(s) missing from participants: human-matthew-manager, human-jessica-spouse, human-james-son. Present (0 of 0 row(s)): <none>. A member is absent here when their conductor never affirmed membership, or when their Membership was authored on a peer whose projection has not reached this one. |
| `features/qahal/household-formation.feature` | Ceremony custody is anchored, fixture custody is marked | a | UNCLASSIFIED | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/qahal/household-formation.feature` | James's membership is sponsored, not self-granted | a | UNCLASSIFIED | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/db/collectives` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/qahal/household-formation.feature` | The household collective is coherent — family-layer, CID-stamped | a | DEFECT-STALE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/db/collectives` | — | AssertionError [ERR_ASSERTION]: collective "family-dowell" carries no canonical collective_cid on this storage peer (got: undefined). The row EXISTS — governanceLayer="family" — so the projection plainly ran. Row keys: ["id","name","description","governanceLayer","constitutionalParentId","reach","region","metadata","createdBy","createdAt","updatedAt","dissolvedAt"]. If no cid key appears there, the read view does not project the column and no ceremony can satisfy this; if the key is present and null, the cid is genuinely unstamped on THIS peer — check which conductor authored create_collective and whether projection_reconcile has gap-filled it. |
| `features/qahal/plural-mishpat-lenses.feature` | A malformed lens is surfaced but flagged, never silently dropped | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/qahal/plural-mishpat-lenses.feature` | Affinity ranks lenses by the distinct members who exercise them | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/qahal/plural-mishpat-lenses.feature` | An un-notarized lens never enters the market (fail-closed) | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/qahal/plural-mishpat-lenses.feature` | An unknown resource yields an empty but valid market | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/qahal/plural-mishpat-lenses.feature` | Rising contention is a call for renewal, not a verdict | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/qahal/plural-mishpat-lenses.feature` | Two schools author lenses over the same resource — both surface, no collapse | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/resilience/app-blob-heal-on-read.feature` | No peer holds the bytes — the 404 names the missing blob | a | UNCLASSIFIED | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/resilience/app-blob-heal-on-read.feature` | Proactive replication leaves a serve-blob delivery trail, like an on-demand heal | a | UNCLASSIFIED | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/pins`, `/api/v1/economic-events`, `/p2p/status` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/resilience/app-blob-heal-on-read.feature` | The heal books a serve-blob REA event for the source peer | a | UNCLASSIFIED | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/economic-events`, `/p2p/status` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/resilience/chaos-peer-churn.feature` | A flapping peer never corrupts what the mesh believes | a | FIXTURE | `/health`, `/api/v1/commitments` | — | precondition missing: E2E_CUSTODY_SETTLE_MS |
| `features/resilience/chaos-peer-churn.feature` | Cascading peer loss degrades the protection status honestly, step by step | a | DEFECT-STALE | `/health` | — | AssertionError [ERR_ASSERTION]: no custody-blob commitment names matthew (12D3KooWSN43tNScVjQS7W5aUaYKdvwHcCLX7AbuF3fNhQSXy8Pg) as provider for sha256-f85393f900eadcb6405ca5a7a1fb567acb812b9dec319882d4c2e13d2b06bfe0 — providers on record: uhCAkT8lH19d6YgXVwvryke_KfMfDqAoHSgd54Bm0DIQH8gUuWPKu, uhCAkkAYhKQH349-IIOJvIOgDtT7m4F4M-_lV6Kihw7jv5a44Jm7f |
| `features/resilience/chaos-peer-churn.feature` | Simultaneous loss of two peers leaves the survivor degrading honestly | a | DEFECT-STALE | `/health`, `/p2p/status` | — | Error: GET http://localhost:8888/db/content/manifesto returned 404: {"error":"Content not found: manifesto"} |
| `features/resilience/commitment-backed-card-lighting.feature` | A healed household's commons provide commitment lights the card | b | STRUCTURAL | `/api/v1/resilience/card-commons/household`, `/api/v1/resilience/grandma-album-1974/household`, `commitmentBackedCollectives` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/commitment-backed-card-lighting.feature` | Provide author skips rather than writing an unjoinable provider | b | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/commitment-backed-card-lighting.feature` | The card counts a commitment whose classification is a JSON list | b | STRUCTURAL | `/api/v1/resilience/card-list/household`, `/api/v1/resilience/grandma-album-1974/household`, `commitmentBackedCollectives` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/commitment-backed-card-lighting.feature` | Transport-id provider commitments never light the household card | b | STRUCTURAL | — | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/conductor-memory-soak.feature` | An operator can read the conductor's leak-vs-cache memory verdict | b | WIRE | — | — | write step glue (5 step(s) unbound, class b) |
| `features/resilience/conductor-validation-spin.feature` | A re-keyed household peer leaves the others at rest, not grinding | a | DEFECT-STALE | `/health`, `/db/content` | — | Error: /projects/elohim/app/elohim-app/scripts/hc-mesh-chaos-rekey.sh --peer james --tag a2o-70809 --phase rekey could not run: spawnSync bash ETIMEDOUT |
| `features/resilience/doorway-footprint-convergence.feature` | Two doorways testify the same footprint for one commons EPR | a | FIXTURE | — | — | precondition missing: E2E_COMMONS_EPR_ID |
| `features/resilience/governed-distribution.feature` | A bounded grant lets keyless Che drive governed distribution | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/resilience/governed-alpha/household` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/governed-distribution.feature` | A Che-facing doorway refuses to boot in an insecure posture | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/governed-distribution.feature` | An unbounded compute delegation is refused at grant time | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/governed-distribution.feature` | Observe mode forwards an ungranted write instead of blocking it | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/governed-distribution.feature` | Revoking the grant denies the next distribution request | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/grandma-photos-survive-node-loss.feature` | Accepting "who holds my photos" surfaces the holders and a revoke control | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | write step glue (3 step(s) unbound, class b) |
| `features/resilience/grandma-photos-survive-node-loss.feature` | Accepting an invite to help emits an observed-care economic event | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | write step glue (3 step(s) unbound, class b) |
| `features/resilience/grandma-photos-survive-node-loss.feature` | The "watching" message names which holder lapsed | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/resilience/summer-1974/household`, `feltStatus` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/grandma-photos-survive-node-loss.feature` | The Family Vault surface shows the holders by name | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | write step glue (4 step(s) unbound, class b) |
| `features/resilience/household-diversity-dataplane.feature` | Salvage candidates carry real households once humans are imagodei-populated | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Cluster page shows offline device with last-seen freshness | a | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Concept card hides badge when distribution is not yet known | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Concept card renders distribution badge when summary is hydrated | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Content-viewer header renders distribution and resilience together | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Content-viewer resilience fold-downs stay inside a phone viewport | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Content-viewer resilience tooltip is live | a | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Distribution badge defers details fetch until tooltip opens | b | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Doorway admin content list shows resilience snapshot icons | a | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/db/content` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Full placement across two households | a | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/db/humans`, `/api/v1/resilience/content-alpha/household`, `/api/v1/placement-gaps`, `placementGaps`, `protectionStatus` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Peer-topology surfaces resilience-cliff warning | a | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Placement gap when commitments are short | a | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/db/humans`, `/api/v1/placement-gaps` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/observable-distribution.feature` | Shefa signals card reflects current placement gaps | a | STRUCTURAL | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/placement-gaps`, `/db/humans` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/resilience/operational-weave.feature` | A deferred lens field is absent from the response, not zero | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/weave` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/operational-weave.feature` | An unsampled custodian does not zero the cluster capacity | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/weave` | — | write step glue (4 step(s) unbound, class b) |
| `features/resilience/operational-weave.feature` | The Prometheus gauge equals the view's placement-gap count | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/weave`, `elohim_placement_gap_count` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/operational-weave.feature` | The weave gap count is cluster-wide, not scoped to one app | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/weave` | — | write step glue (3 step(s) unbound, class b) |
| `features/resilience/operational-weave.feature` | The weave view reports cluster health from existing relations | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/weave` | — | write step glue (4 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | A commons-reach provide commitment does NOT back a household-reach content | b | WIRE | `/api/v1/resilience/dim-household-only/household`, `commitmentBackedCollectives` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | A household-reach content can be commitment-backed (not just commons) | b | WIRE | `/api/v1/resilience/dim-household-backed/household`, `commitmentBackedCollectives` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | A partial content shows the half glyph AND the partial status class | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | A peer's capacity view never wraps an over-pledge into compliance | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | A protected content shows the full glyph AND the protected status class | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | A stewarding household with an active provide commitment is commitment-backed | b | WIRE | `/api/v1/resilience/dim-backed/household`, `commitmentBackedCollectives` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | Content stewarded by no household reads at-risk, honestly | b | WIRE | `/api/v1/resilience/dim-orphan/household`, `protectionStatus`, `stewardingCollectives` | — | write step glue (1 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | Geographic distribution buckets stewards relative to the viewer | b | WIRE | `/api/v1/resilience/dim-geo/household` | — | write step glue (5 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | Stewards without region data are honest unknowns, not zeros | b | WIRE | `/api/v1/resilience/dim-noregion/household` | — | write step glue (3 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | The cluster page shows an honest free/used/committed triptych | b | FIXTURE | `/threshold/dashboard`, `/health` | — | precondition missing: E2E_DEVICE_MODE |
| `features/resilience/resilience-dimensions.feature` | The header connection chip shows a live peer count | a | UNCLASSIFIED | `/health` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/resilience/resilience-dimensions.feature` | The tooltip's peers-online number counts only stewarding households | b | WIRE | `/api/v1/resilience/dim-triad/household`, `details.onlinePeers.live` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | Three households with two live peers reach protected | b | WIRE | `/api/v1/resilience/dim-triad/household`, `protectionStatus` | — | write step glue (2 step(s) unbound, class b) |
| `features/resilience/resilience-dimensions.feature` | Two stewarding households lift content to partial | b | WIRE | `/api/v1/resilience/dim-pair/household`, `protectionStatus`, `stewardingCollectives` | — | write step glue (1 step(s) unbound, class b) |
| `features/resilience/salvage-placement.feature` | A blob already held at its target level triggers no salvage adoption | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | write step glue (5 step(s) unbound, class b) |
| `features/resilience/salvage-placement.feature` | A peer that has not opted in to salvage capacity is never conscripted | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | write step glue (7 step(s) unbound, class b) |
| `features/resilience/salvage-placement.feature` | A spare peer that is not among the closest holders defers to those who are | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | write step glue (6 step(s) unbound, class b) |
| `features/resilience/salvage-placement.feature` | An under-replicated blob is adopted by the closest opt-in peer and the replica count rises | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery` | — | write step glue (9 step(s) unbound, class b) |
| `features/resilience/salvage-placement.feature` | The family sees a thin blob become protected after a peer adopts it | b | WIRE | `/health`, `/api/v1/cluster`, `/api/v1/peers/delivery`, `/api/v1/resilience/family-album/household`, `feltStatus.reassurance` | — | write step glue (4 step(s) unbound, class b) |
| `features/resilience/substrate-reconciliation.feature` | A peer-discovered commitment converges from the own conductor | b | WIRE | — | — | write step glue (7 step(s) unbound, class b) |
| `features/shefa/human-resilience.feature` | Degradation — Matthew goes offline, Jessica's resilience drops | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Elohim discernment — institutional attestation for sensitive data | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Maria — cold start zero peers | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Matthew + Jessica — household reciprocation, partial protection | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Matthew alone — single conductor, at risk | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Operator flips useGraphqlTopology — human sees the same cluster numbers | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Per-content sensitivity — medical records vs shared media | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Recovery — after-action review when Matthew returns | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/human-resilience.feature` | Right to be forgotten — releasing expired content | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/limitarian-governor.feature` | A passed ratification writes the governed limit | b | WIRE | — | — | write step glue (3 step(s) unbound, class b) |
| `features/shefa/limitarian-governor.feature` | An out-of-wall gradient cannot be ratified | b | WIRE | — | — | write step glue (2 step(s) unbound, class b) |
| `features/shefa/limitarian-governor.feature` | Concentration friction relaxes only at the governed target | b | WIRE | — | — | write step glue (4 step(s) unbound, class b) |
| `features/shefa/m1-matthew-terrance-delivery.feature` | Cluster page shows Matthew's device tile with real metrics | b | STRUCTURAL | `/threshold/dashboard`, `/health` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/m1-matthew-terrance-delivery.feature` | Manifesto chapter content viewer shows distribution badge | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/m1-matthew-terrance-delivery.feature` | Manifesto chapter content viewer shows resilience snapshot | b | STRUCTURAL | `/threshold/dashboard` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/shefa/m1-matthew-terrance-delivery.feature` | Matthew's device tile shows free / used / stewarded compute breakdown | a | STRUCTURAL | `/threshold/dashboard`, `/health` | — | needs shem (multi-tenant commons canvas — no household analog) |
| `features/ssr/browser-hydrates-without-flash.feature` | Concept page hydrates seamlessly | a | FIXTURE | `/threshold/dashboard` | — | precondition missing: E2E_DEVICE_MODE |
| `features/ssr/compose-serves-the-projected-app.feature` | A projected app with its own server bundle composes its own selector | a | FIXTURE | `/health/startup` | — | precondition missing: E2E_SSR_PROJECTED_APP, E2E_SSR_SHED_ROUTE |
| `features/ssr/external-webfetch-renders-content.feature` | HTTP client without a JS engine fetches a concept page | a | UNCLASSIFIED | `/health`, `/db/content` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/ssr/render-trace-distinguishes-stall-from-empty.feature` | A healthy concept page reports a non-stalled terminal | a | UNCLASSIFIED | — | — | insufficient surface signal — no HTTP/metric/field surface named in step text or glue |
| `features/ssr/render-trace-distinguishes-stall-from-empty.feature` | A stalled upstream falls back fast and self-labels as stalled | a | FIXTURE | — | — | precondition missing: E2E_SSR_STALL_ROUTE |
| `features/ssr/render-trace-distinguishes-stall-from-empty.feature` | An empty-but-healthy render is labelled rendered-empty, not stalled | a | FIXTURE | — | — | precondition missing: E2E_SSR_EMPTY_ROUTE |
| `features/ssr/social-card-crawler-gets-rich-preview.feature` | Social card crawler previews a learning path step | a | UNCLASSIFIED | `/health`, `/db/path` | — | fully wired, named surfaces exist — no live mesh report this run to confirm pass/fail (re-run `pnpm census` after a mesh test run) |
| `features/storage/constitutional-ratio-enforcement.feature` | A first promise that fits the fair balance is accepted | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/storage/constitutional-ratio-enforcement.feature` | A promise made before the server's size is known is never silently trusted | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/storage/constitutional-ratio-enforcement.feature` | A promise that would crowd out her own family is gently refused | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/storage/constitutional-ratio-enforcement.feature` | The dashboard tells the truth about promised space versus space actually used | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/storage/declared-storage-policy.feature` | A declared wish never breaks a steward's promise | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/storage/declared-storage-policy.feature` | A video rests cheaply when nobody is watching | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/storage/declared-storage-policy.feature` | A watched video stays warm for the evening | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/storage/declared-storage-policy.feature` | An app cannot demand more warmth than the household pledged | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/storage/declared-storage-policy.feature` | Movie night starts within moments | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/storage/declared-storage-policy.feature` | The video cools back down after the gathering | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/storage/household-resiliency-handshake.feature` | A friend's contribution to our family album is still backed up by our steward | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/storage/household-resiliency-handshake.feature` | A scoped promise still matches the content it was made for | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/storage/household-resiliency-handshake.feature` | Both families keep their promise; both see their memories protected | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/storage/household-resiliency-handshake.feature` | One family never returns the promise; the network names them, gently | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | A card with no provide commitments names contracts-short with its window | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | A fail-closed catching-up shed declares anchor divergence with a live gauge | c | WIRE | — | — | write step glue (8 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | A fossil-key provider reads as stale-key, not offline | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | An anonymous read before commons reach is earned explains the earning | b | WIRE | — | — | write step glue (6 step(s) unbound, class b) |
| `features/trust/trust-legibility-atlas.feature` | Committed-but-unavailable providers are broken down by cause | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | Each interim state renders its why to a human | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | Household-blind placement declares its degraded strategy | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | Op-gate shadow verdicts are readable, not log-only | c | WIRE | — | — | write step glue (7 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | Rows excluded by identity-namespace mismatch are counted, not vanished | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
| `features/trust/trust-legibility-atlas.feature` | Two doorways on divergent heads self-report their disagreement | c | WIRE | — | — | write step glue (6 step(s) unbound, class c) |
