# Code-reds the mesh produced for free (2026-08-21)

The mesh inventory run (`@e2e and not @wip and not @browser`, 194 eligible) returned 39 failures and 26
pendings. Joining it against the four classifier slices and subtracting the known environment classes
leaves **25 reds that are about the code, not the substrate**. They cost nothing to find — they fell out
of standing up the household mesh once.

Each row: the assertion as it failed, then a one-line hypothesis. Hypotheses are *hypotheses* — the
assertion is the durable artifact.

## What was subtracted first (14 env-reds, not defects)

| n | class | evidence |
|---|---|---|
| 6 | doorway admin bootstrap unset (`API_KEY_ADMIN`) | `403 Admin permission required` on `/admin/users`, `/admin/conductors`, `/admin/nodes`; `Admin bootstrap is not configured on this doorway` — operator-onboarding, user-management, conductor-visibility ×2, doorway-self-registration ×2 |
| 7 | household fixture manifest never emitted | `live household fixture missing storage peer "matthew"` ×4 (peer-loss-failover), `missing storage pool for doorway "alpha"` ×2 and `Cannot resolve doorway URL from: E2E_DOORWAY_APEX` ×1 (doorway-pool-degrade) |
| 1 | lamad SPA bundle not staged | `GET /lamad → 404 {"error":"App not found: lamad-spa"}` (protocol-omnibar-chrome) |

Remedies for all three are in `profiles.md` §2–§4. 17 further rows went `pending` for step debt or the
unset `E2E_STORAGE_<PEER>` vars and are also excluded — a pending is not a red.

---

## A. Genuine code-reds (17)

**1. `dataplane/doorway-catching-up-page.feature` — diagnostic probes are never answered with the doorway shed body**
> `Surface "/db/p2p/conductor-diagnostics" on alpha-A was breaker-shed by the doorway (503 catching-up) — diagnostic probes must bypass the breaker`

The diagnostic-bypass list does not cover `/db/p2p/conductor-diagnostics`, so the one endpoint an operator
reaches for *during* a catch-up is the one the catch-up shed hides. This is the highest-value red in the
set: it makes the substrate-trust-contract runbook's primary probe unavailable exactly when it is needed.

**2. `dataplane/resilience-identity-coherence.feature` — No household-member human on alpha-A carries a fossil agentPubKey**
> `GET http://localhost:8888/db/p2p/conductor-diagnostics returned 503: {"status":"catching-up","retryAfter":30,"cause":"upstream","circuit":"closed","errorStreak":0}`

Same defect as #1, reached from a second feature — note `circuit: "closed"`, `errorStreak: 0`: the doorway
shed a request while its own breaker reported healthy. Worth confirming these are one bug before fixing.

**3–5. `dataplane/reach-enforced-http.feature` — three reach-enforcement scenarios fail their own non-vacuity control**
> `NON-VACUITY CONTROL FAILED: "bdd-smoke-tests" was served to an anonymous caller (status 200). Either that row is no longer beyond anonymous reach — pick another control`

The control row is anonymously readable. Either the seed's reach for `bdd-smoke-tests` is wrong, or reach
is not enforced on this path. The scenarios cannot distinguish those, which is itself the finding: the
control needs to be a row whose reach the seed *guarantees*. Note that the three-way repeat means one
mis-seeded row silently disarms three security assertions.

**6. `auth/reach-commons.feature` — Anonymous reader is rejected for community-reach content (403 with requiredReach)**
> `AssertionError: Expected values to be strictly equal` (status/shape mismatch)

The sibling of #3–5 from the auth side: community-reach content did not answer with the expected
`403 + requiredReach` shape. Same suspicion — reach enforcement or reach seeding.

**7–8. `dataplane/content-sync.feature` — a freshly authored node has empty sync heads on both doorways**
> `/sync doc "node:e2e-45cef93f-…" on alpha-A: heads is empty or missing — document not yet synced`
> `… on elohim.host: heads is empty or missing`

The Automerge producer did not create a document for an API-authored content node. Both legs fail
identically, so this is production-side (no doc authored), not convergence-side. Check the producer's
`h_app_id="elohim"` requirement — a producer that writes under the wrong app id leaves exactly this trace.

**9. `content/content-lifecycle.feature` — Read own content**
> `GET /db/content/e2e-d9d88af1-… returned 404: {"error":"Content not found: …"}`

Write-then-read of the author's own row 404s. Most likely the provenance gate refusing a row that was
never DHT-anchored (`dht-anchored-content: false` in Act I). If so it is an act boundary rather than a
defect — but "an author cannot read back what they just wrote" deserves an explicit decision, not a
silent 404.

**10–11. `dataplane/peer-mesh.feature` — alpha-A / elohim.host `p2p.caughtUp` is false**
> `alpha-A /health: p2p.caughtUp is false (expected true)`

Measured *after* the quiesce gate passed. Either the gate's predicate (which reads matthew/storage-A
only) and `/health`'s `caughtUp` disagree about what caught-up means, or catch-up regressed between the
gate and the suite. Two probes for one invariant that do not agree is the finding.

**12. `dataplane/resiliency-saga/07-custody-witnessed.feature` — At least one shard is witnessed as actively stocked**
> `labeled metric "elohim_custody_class_count{class="stocked"}" on alpha-A: 0 >= 1 — FAILED`

The custody manifest backfill produced no self-held evidence on a mesh where matthew demonstrably holds
the blob. First suspect is the documented `blobHash` vs `serverBlobHash` join key (saga README §Documented
residue) — a pair classified by the client hash never joins the custody fold.

**13–14. `delivery/content-addressing.feature` — X-Content-Address is a CID, the assertion wants a sha256**
> `Header "X-Content-Address" value "bafkreihokma4tfmwp7y6bj5qpj7v4lpe6x2upozxdbkh2mzeqrj3o7ftb4" does not look like a sha256 content address`

The doorway emits a CIDv1 (base32, `bafkrei…`); the step asserts 64 hex characters. The CID-first
migration moved the header and left the assertion behind. Fix the assertion — and check every other step
that pattern-matches a content address, because this one shape change disarms all of them silently.

**15. `delivery/delivery-diagnostics.feature` — Response headers indicate which cache layer served the request**
> `Expected serving layer "projection-cache" (HIT|HIT-COALESCED) but got X-Cache: "BYPASS" — BYPASS means the doorway never engaged its app-file cache at all`

The mesh doorway never engages its app-file cache. Either a config the mesh does not set, or the cache
declines to engage for locally-served blobs. Worth knowing before trusting any cache-layer measurement.

**16. `protocol/landing-page-dogfood.feature` — An in-kind REA Commitment declares Matthew's hosting agreement**
> `No commitment had inScopeOf containing "host:alpha.elohim.host"`

The hosting commitment is scoped to the *production* host name; on the mesh the doorway is
`alpha-elohim-host` at localhost. Either the scenario should read the host from the fixture, or the seed
should scope the commitment to the doorway it is actually seeding. Right now the assertion is
host-name-literal, which makes it un-runnable off the fleet.

**17. `resilience/doorway-footprint-convergence.feature` — Two doorways testify the same footprint for one commons EPR**
> `snapshot exposes no holder set`

The footprint snapshot carries no holder set at all — the two-doorway comparison never gets to compare.
Note this feature is one of the two that read the second doorway as `E2E_DOORWAY_BETA`; confirm it was
pointed at the mesh's doorway B and not the live production doorway before pursuing the holder set.

## B. Seed-reds — eight failures downstream of two swallowed non-zero seeder legs (8)

`seed-commitments` and `seed-delegates-compute` exited non-zero during bring-up **and the stage
continued**. Everything below is the shadow of that one swallowed error. The finding is the swallow: a
seeder leg that fails and does not fail its stage converts one honest red into eight misleading ones.

- `qahal/household-formation.feature` — *The household collective is coherent* → `collective "family-dowell" carries no canonical collective_cid on this storage peer (got: undefined). The row EXISTS — governanceLayer="family" — so the projection …`
- `qahal/household-formation.feature` — *All three members are affirmed participants* → `triad member(s) missing from participants: human-matthew-manager, human-jessica-spouse, human-james-son. Present (0 of 0 row(s)): <none>`
- `qahal/household-formation.feature` — *The ambient custody mesh emerged from the ceremony* → `No commitments listed — did the listing step run (and is anything seeded)?`
- `resilience/household-reciprocity.feature` — *Matthew and Jessica hold the M1 custody commitments for each other* → `No commitments listed …`
- `resilience/household-reciprocity.feature` — *The triad mesh — James is in the household's custody, both ways* → `No commitments listed …`
- `dataplane/resiliency-saga/09-projectors-carry.feature` → `Expected "commitmentBackedReplication.commonsCommitments" >= 1; got: 0`
- `dataplane/resiliency-saga/10-card-tells-truth.feature` → `stewardingCollectives on alpha-A is 0 — expected > 0 (the resilience card must not read zero)`
- `dataplane/resilience-identity-coherence.feature` — *No household-placed human is missing its agent_pub_key* → `4 human(s) on alpha-A carry a household_id but a NULL agent_pub_key (the all-zeros-resilience-card gap: ["human-adam-firstman","human-eve-firstwoman","human-gertrude-gr…`

The last one is a different leg — `seed-conductor-identities` seeded 2 of 7 — and it is the honest Act I
statement of `per-human-conductor: false`. The other seven should re-measure green once the commitment
seed succeeds; if any stays red, it is a genuine code-red that was hiding behind the seed failure.

## Reading these against the acts

All 25 are Act I reds: they were produced by a household mesh with no fleet, no shem and no observability
stack. That is the layering argument in one measurement — **a household run is a real proving ground, and
the suite has been paying fleet prices to learn less.**

## What Act I cannot witness

A held scenario is not always step debt. Some assertions are unwitnessable on a household mesh *by
construction*, and saying so is a layering fact worth as much as a red.

| assertion | why Act I cannot witness it | act that can |
|---|---|---|
| a content item stewarded by **2 distinct collectives** (`observable-distribution.feature`, "Resilience snapshot reports the diversity shortfall") | the household IS one collective — matthew, jessica and james share one fate, which is the whole reason copies inside it barely count. A single-collective mesh can never produce a 2-collective placement. | **Act II** (neighbourhood: adam's household federates, so a second collective exists) |
| a content item meeting the **floor of 3+ distinct collectives** (same feature, "coverageShortfall is a present 0 when the floor is met") | same reason, one further out: the standard tier's floor is 3, and Act I can supply exactly 1. | **Act II** |

Measured 2026-08-21: a search across 40 content items on the Act I mesh found **none** with a
`stewardingCollectives` count other than the household's own. Both scenarios are therefore tagged
`@act:ii` — the runtime gate holds them on an Act I lane and names the act in its skip line, and
`scope-reconcile` resolves them against the neighbourhood contract rather than reading them as
household work that never got done.

The general rule this instances: **diversity floors are an Act II property.** Any assertion whose subject
is "how many distinct collectives / households / fates hold this" needs more than one household to be
true or false at all, and belongs to the act that has them. Act I can prove that placement, gap
detection and the shortfall ARITHMETIC work; it cannot prove a shortfall against a floor it cannot reach.

## Disposition (2026-08-21, same day — all on branch `fix/doorway-breaker-trial-theft-and-apps-extraction-herd`)

| # | verdict | commit | note |
|---|---|---|---|
| 1–2 | **doorway defect, fixed** | `21b59b654` | honor-backpressure branch re-minted storage's informative 503 as `catching-up`; diagnostic probes now relay upstream verbatim. Residual: `/db/p2p/conductor-diagnostics` reports only the *embedded* conductor — blind for externally-run conductors (mesh; check alpha) |
| 3–6 | **seed defect, fixed** | `7aae6c9d9` | `seed.ts` wrote a literal `reach: 'public'` for every row (mesh AND fleet); gate enforces correctly. Open operator decision: ungraded default stays `public` (`UNAUTHORED_CORPUS_REACH`) — flipping it takes the anonymous corpus from ~3400 rows to ~90 |
| 7–8 | **test-side, fixed** | `95a2e5941` | docId pinned to an alpha-only node; producer healthy (same head on both peers in ~2 s). "Converged" now means head-set equality, not two non-empty arrays |
| 9 | **test-side, fixed** | `95a2e5941` | `maxAttempts` halved the step's own 15 s budget; rows anchor 8–19 s after POST inside storage's documented drain window. Gate exonerated |
| 10–11 | re-measure | — | doorway B was mid-catch-up behind the 3,400-head seed; not a defect until re-measured quiesced |
| 12 | held | — | dead custody-announce plane, already shelved |
| 13–14 | **test-side, fixed + new storage defect** | `658372c56` | CID assertion fixed; uncovered `is_content_address()` knowing only `sha256-` → CID `/apps/` 404 (storage agent dispatched) |
| 15 | open | — | `X-Cache: BYPASS` on the mesh — investigate before trusting cache-layer measures |
| 16 | **test-side, fixed + storage defect** | `658372c56` | host literal was correct; real bug is `normalize_create_input` double-wrapping `inScopeOf` (storage agent dispatched) |
| 17 | **genuine, open** | — | footprint snapshot has an empty holder set on both doorways — gap between "commitment exists" and "holder populated" |
| §B | **seeder defect, fixed; 3/7 green; ROOT of the rest = Prologue cast** | `822c32f16`, `666e97a53` | two legs hard-required CI-only env; remaining: `collective_cid` unstamped (genuine), participants (Act I `per-human-conductor: false`, honest), ch09/ch10 wait on the landing self-custody pair (seeder follow-up in flight); custody seeder also found NON-idempotent (6→12 rows per run) |
| new | **storage defect, open** | — | reach-carrying `PATCH /db/content/{id}` returns 200 and applies nothing (storage agent dispatched) |
| §B root | **Prologue cast error** | backlog `mesh-prologue-cast-and-env-gaps` | `seed-conductor-identities` is hostname-affine; on loopback it cast adam onto james's conductor → participants 0, no MEMBER_OF, `hasHouseholds 0`, ch10 red. Named `CONDUCTOR_URLS` fix in flight; this mesh must be regenerated to re-cast. Saga in order on the mesh: 17 pass / 2 fail (ch09, ch10) / ch07 custody-witnessed GREEN |
| new | **storage defect, fixed** | `3dd5c5de1` | duplicate node/stewardship POST returned 500 `UNIQUE constraint`; now 201/200/409 with `ON CONFLICT DO NOTHING` + re-read; ALSO `/db/nodes*` handlers were never declared in `build_manifest()` so every doorway-proxied call 404'd — the real cause of `seed-nodes`' first-run failure |
| new | **storage defects, fixed** | `39534f9fe` `10cc53ca8` `9b470fcc9` `b36e1efaa` | CID `/apps/` identifiers resolve; `inScopeOf` no longer double-wrapped (3 legacy rows on matthew still carry the old shape); reach PATCH on an anchored row → 400 (honest refusal), on an unanchored row the committed reach is mirrored to SQL |
| alpha | **conductor spin — cure 1 in the fork** | fork `b9c7458ae` `c9a6c4439` (branch `fix/sys-validation-unfetchable-deps-backoff`, pointer NOT bumped) | unbounded `join_all` local re-lookup of known-missing deps + no per-dep backoff + per-attempt saturation log; mesh before/after with the chaos re-key in progress (`hc-mesh-chaos-rekey.sh`) |

