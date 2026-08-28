---
title: "Ratchet to delivery — the dataplane campaign as verifiable progress lanes (resiliency · peer diversity · federation · delivery · SDK)"
id: ratchet-to-delivery-dataplane-sdk-lanes
tier: spec
status: Draft
created: 2026-08-28
maintainers: Matthew Dowell + Claude Fable 5
class: substrate
context-tier: disclosed
steward: rust-architect
graduation-trigger: every lane's delivery rung (R11, P10, F10, D7, S10) locked OR superseded-by-implementation
domain: peer-hoster dataplane × confidentiality seam × doorway projection × SDK grammar × evidence ladder
habits: [dataplane-convergence, identity-cross-signed, reach-enforced-everywhere]
topic: [ratchet, progress-lanes, dataplane, reactive-load, trust-priced-sync, dual, iroh, federation, delivery, sdk, evidence-ladder]
cites:
  - "substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - "transport-self-awareness-diversity-harness-design | Transport self-awareness and the two-peer diversity harness | sha256:96730ca15491ce76 | path: genesis/docs/superpowers/specs/2026-08-24-transport-self-awareness-diversity-harness-design.md"
  - "evidence-ladder-push-left | Evidence Ladder + Push-Left Pressure | sha256:ac39aeb003dada60 | path: genesis/docs/superpowers/specs/2026-08-10-evidence-ladder-push-left-design.md"
  - "platform-one-sdk-many-apis-design | THE ELOHIM PLATFORM MODEL | sha256:a15b10c68787a460 | path: genesis/docs/superpowers/specs/2026-06-14-platform-one-sdk-many-apis-design.md"
  - "latency-valueflow-chain | The Latency Valueflow Chain | sha256:9b3106cb3707e838 | path: genesis/docs/superpowers/specs/2026-08-20-latency-valueflow-chain-design.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - elohim/elohim-storage/.epr-meta/identity-cross-signed.habit.md
  - .epr-meta/reach-enforced-everywhere.habit.md
  - genesis/a2o/LAYERS.md
---

## Context (why)

The 2026-08-28 shift closed `done` (genesis E2E 6→2, #1515/#1516). What it exposed is that the dataplane's remaining reds are **mechanical and unmeasured**, not algorithmic — and that our evidence is concentrated on one lane (the household mesh) while the fleet lane is structurally silent (edge #1381: 3 passed / 1 failed / **83 skipped**, every `@act:i` concern held by design). The destination is an SDK where an app developer persists, syncs, accounts (REA) and projects objects and *just knows* each one carries its governance and economic surface — ambient by construction, never remembered. That confidence is only honest when the invariants behind it are **green habits with gates that run**, not prose. So the plan is built as **ratchet lanes**: ordered checkpoints, each with the command that verifies it, its current reading, its target, and the pawl that locks it once green. "How much is left" is a count you can re-run.

Grounded facts the lanes rest on (file:line in the lane tables):

- **Resiliency**: local quiesce reproduces the fleet plateau (W4: sweep 21 re-found actionable 65, FAIL(deadline), 3,452 rows). Fleet `caughtUp=false` on storage-A every run; matthew's content heal leg (`projection_reconcile.rs:3997`) hits 3 never-answered 25 s calls (`HEAL_ATTEMPT_TIMEOUT` `:461,:475`) and its per-invocation `HealCircuit` (`:912-968`) yields → `healed:0`, conductor reporting 247 s receipt latency (operator ceiling). Shem trio exit-139 several times an hour. The one live recovery-matrix series (13 rows, 08-24) was lost to a `/tmp` wipe (`backlog/mesh-recovery-timeline-not-durable.md`); only fixture numbers survive.
- **Peer diversity**: two device rungs of 15 are real (household on-prem; shem/apex pods — same storage container). Sovereign-peer join PROVEN 08-28 but what it authors the fleet never serves (P1 gap). Tauri steward builds in CI, runs nowhere; `steward/node` inert; `lvi` M0 unstarted. `homo-iroh` fails P3/P4 — `drain_acquisition_queue` (`p2p/mod.rs:9550-9590`) is libp2p-only; the iroh byte path (`fetch_blob_over_iroh` `node.rs:242`, `iroh_fetch_leg()` `gossip_receive.rs:86`) is never reached from the pull queue. `peer_class` reads `public` for every edge: handshake stub (`p2p/mod.rs:5150-5156`, `:6533-6543`), `verify_trust_context` zero callers, four verifiers `Ok(vec![])` (`trust_verification.rs:176-230`). The binding minter **exists** (`p2p/binding_mint.rs`, default-on) — the `identity-cross-signed` `first_move` is stale.
- **Federation**: `federation-deploy` sc2 (elohim.host resolves the landing head without the per-host blob crutch) is the top red's actual RED; sc3 `@wip`. `view_federation.rs` federates cluster/topology/inventory/head-record, reach-gated (`:472-485`), 5 s responder budget (`:66,:73`). `reach-enforced-everywhere`: HTTP cured, admin socket closed (#1386); CRDT/shard/head-record planes still serve community-tier anonymously. `bridges/atproto` / `activitypub` do not exist.
- **Delivery**: the App pipeline's "E2E Testing - Alpha Validation" stage is a Cypress ghost (binary missing, `catchError`, prints ✅, never ran a2o). Doorway-B refuses the apex seed with 403 since 08-25 (`API_KEY_ADMIN` vs `API_KEY_SEED` conflation — operator credential). "A human reaches the landing page through both doorways" is green on the household mesh and *unmeasured* on the fleet.
- **SDK**: `storage-client-ts` (ts-rs, 446 views) and the manifest grammar (9 domain manifests; `avodah` a genuine second composition, internal) are real and gate-enforced. `crates/elohim-sdk` README: *"neither reading nor writing works against a public endpoint today — both require an `elohim-storage` you run yourself"* (edges 1, 2, 8). No tenancy guard between apps on the content table (`contentType` collisions unpoliced; `h_app_id` exists only on the sync plane, hardcoded `"elohim"`/`"lamad"` at `http.rs:1572,4863`). Apps cannot declare a view/projection (fixed `INTERFACE_FILES`, Rust PR). CLAIM/BECOME/RENEW are manifest-only (`elohim/sdk/README.md:98-119`). The platform spec's four "smallest first steps" (grammar dir, catalog, corpus map, `create-elohim-app`) are unlanded. `app-port` is plan-only. Khan Academy has a design draft; WordPress is a horizon doc; Jenkins/GitHub have none.

## The ratchet — how a lane works

- A **rung** = one check: a command, an `@concern`, or a habit conjunction. It has a **reading** (evidence with a build/run id) and a **target**.
- A rung is **locked** when green AND its check sits in a **pawl** that runs without anyone remembering: T0/T1 `just gate` (pre-push), T2 `just test mesh` (+ the T2 receipt banner), T3 `just dev conductor profile=alpha` + `@concern:sovereign-peer-join`, T4 `[edge:validate-only]` byConcern report. A green rung with no pawl is *unlocked* and counts as open.
- Only the **lowest open rung** of a lane is in flight; a regression on a locked rung is the top red and pre-empts. Habits flip only with evidence (covenant rule 4); `latency-scoreboard`'s `best_observed` ratchets numbers.
- The **top rung of every lane is a delivery reading**: something a person or an app developer sees through a doorway.
- Progress = open rungs per lane, re-counted at every shift close (`just status habits`, `just status saga`, the lane tables' commands).

Legend: 🟢 locked · 🟡 green-unlocked (no pawl) or partial · 🔴 red · ⚪ unmeasured.

## Lane R — Resiliency (habits: `dataplane-convergence`, `blob-durability`; saga)

| # | Rung — check | Pawl | Reading (evidence) | Target |
|---|---|---|---|---|
| R1 | `cargo test --test sync_libp2p_convergence`, `--test household_resilience` (elohim-storage) | T1 `just gate` | 🟢 storage gate green (08-28, lib 3,009/3,009) | hold |
| R2 | saga 11/11 — `just test mesh` `--profile saga` → `just status saga` | T2 | 🟢 11/11, run `20260828T044708Z-6e4fa438` | hold; re-prove per transport pair (P2) |
| R3 | warm/cold recovery `time_to_recover` durable — `just mesh recovery <shape> <peer>` → `recovery-timeline.jsonl` | T2 (needs durable path) | 🟡 dual/dual warm 58 s (08-25 journal); series lost to `/tmp` wipe | timeline under `genesis/a2o/reports/`, N=3 per shape |
| R4 | local quiesce sustained on the seeded corpus — `just mesh quiesce` (io_baseline within 2×) | T2 | 🔴 FAIL(deadline) sweep 21 actionable 65 (W4, 08-28) | PASS N=3 |
| R5 | plateau rows have names — `/p2p/status.syncVerdicts`, `elohim_sync_verdict_total{reason}` | T1 test + T2 read | ⚪ does not exist (`sync_gate.verdict()` is the pause verdict) | reason counts recorded at the plateau |
| R6 | heal under injected admission pressure — `healed`/sweep > 0 every sweep | T2 (pressure profile) | ⚪ never exercised locally | > 0 N=3 |
| R7 | fleet A `caughtUp=true` — `@concern:inventory-convergence` | T4 validate-only | 🔴 false every run (#1381 … #1516) | green ×2 consecutive fresh triggers |
| R8 | fleet measure always banks — byConcern report carries `churn_state`, never DID-NOT-MEASURE | T4 | 🔴 3 DID-NOT-MEASURE in one evening (#1367-69) | 2 consecutive labelled reports |
| R9 | blob-durability S3 curve (holder-count vs wall-clock) — `fanout-1`/`fanout-2` rows | T2 matrix | ⚪ fixture values only | real N=3 |
| R10 | shem exit-139 legible — faulting thread in Loki | T1 (`kill -SEGV` test) → operator | 🔴 no signal captured | handler proven locally; first fleet capture |
| R11 | **delivery**: `relay-capacity` + reconnect-storm on an operator-attached fleet roll | T4 (operator) | ⚪ `@wip` / needs process control | one attached run |

Open: 9 of 11 (R1, R2 locked).

## Lane P — Peer diversity (habits: `dataplane-convergence` transport-parity, `identity-cross-signed`)

| # | Rung — check | Pawl | Reading | Target |
|---|---|---|---|---|
| P1 | three homogeneous transports converge — `just mesh matrix` / `@concern:transport-parity` | T2 | 🟢 3/3/3, p50 56.6/58.6/56.7 s (08-25) | hold |
| P2 | all 8 recovery scenarios present and truthful — `transport-recovery-measurements.feature` | T2 (after R3) | 🔴 feature `@wip`; series lost | un-`@wip`, N=3 |
| P3 | `homo-iroh` warm/cold P0–P4 green | T2 matrix | 🔴 P3/P4 fail (pull queue libp2p-only) | green |
| P4 | `PathObservation`/`select_path` live — `elohim_transport_route_total{reason}` on both planes | T1 contract tests + T2 | ⚪ spec-only (rows 1–4) | non-zero both planes; C4 honest-absence test green |
| P5 | sovereign peer joins alpha and is visible — `@concern:sovereign-peer-join` sc1+5 | T3 | 🟡 proven by hand 08-28; scenarios `@wip` | scenarios pass |
| P6 | sovereign peer's authored node served by the fleet — sc3 | T3 | 🔴 404 for 4 min (P1 gap) | 200 through doorway-alpha |
| P7 | `peer_class` ≥ 2 classes on the mesh — `elohim_sync_request_outcomes_total{peer_class}` | T2 | 🔴 all `public` (handshake stub) | household `trusted+`, stranger `public`, unbound `unverified` |
| P8 | cross-signed bindings examined on the fleet — `bindings_examined{enforce} > 0 ∧ unverified{enforce} == 0` | T4 | 🔴 reads zero (joins absent vs bindings absent — undecided) | conjunction true |
| P9 | a second binary participates (Tauri steward or `steward/node`) in the household mesh | T2 | ⚪ builds in CI, runs nowhere | one steward peer in a matrix row |
| P10 | **delivery**: a workspace peer authors and a browser reads it through both doorways | T3 → T4 | 🔴 | sc3 + `served-projected-head` green |

Open: 9 of 10.

## Lane F — Federation (habits: `notary-authority`, `reach-enforced-everywhere`, `identity-cross-signed`, `doorway-failover`)

| # | Rung — check | Pawl | Reading | Target |
|---|---|---|---|---|
| F1 | content authored on alpha-A converges to elohim.host ≤ 30 s — `@concern:content-sync` | T2 | 🟢 4/4 (08-28 household) | hold |
| F2 | notary answers, never LWW — `@concern:notary-authority` | T2 (fleet skips) | 🟢 3p/0f #1362; 17/20 household 08-21 | hold |
| F3 | doorway pair survives loss/shed — `@concern:doorway-failover` | T2 | 🟢 10/10 (08-25b, household authority) | hold |
| F4 | elohim.host resolves the landing head with no per-host blob crutch — `federation-deploy` sc2 | T2 → T4 | 🔴 404 "App not found", blobHash null | green; `stageSpaBlob` CI stage retired |
| F5 | seed authority ≠ admin authority — `federation-deploy` sc3 | T4 | 🔴 `@wip`; doorway-B 403 since 08-25 | green on a deployed build |
| F6 | head-record federation serves within budget — `view_federation` hash-only vs record ratio | T4 metric | 🟡 wired; adopt-before-author evidence-starved (phantoms 61 %) | `budget_elapsed` share ↓ after R6 |
| F7 | reach enforced on the CRDT plane — `@concern:reach-enforced-sync` (scoped doc absent on an unverified peer) | T2 | 🔴 exclusion only (`reach_is_distribution_safe`) | scenario green |
| F8 | reach enforced on head-record/shard/`/apps/{cid}` egresses | T2 byte-route scenario → T4 | 🔴 community-tier served anonymously | green on a fleet build |
| F9 | fleet lane measures federation at all — `@act:ii` siblings for read-only concerns | T4 | 🔴 83/87 skipped (#1381) | ≥ 7 federation/notary scenarios banked |
| F10 | **delivery**: one landing, two doorways, same declared head, reach-honest — `served-projected-head` + `doorway-catching-up-page` on the fleet | T4 | ⚪ green household, skipped fleet | green ×2 fresh triggers |

Open: 7 of 10.

## Lane D — Delivery (the `/deliver` verdict; `operator-runtime-surface`)

| # | Rung — check | Pawl | Reading | Target |
|---|---|---|---|---|
| D1 | genesis E2E failures ≤ 3, both owned | T4 | 🟢 2 (#1515, #1516) | hold |
| D2 | every push touching `src/{p2p,sync,reconcile}` carries a fresh household report — T2 receipt banner | pre-push | ⚪ no such leg | banner live (warn → strict) |
| D3 | a browser regression on elohim.host is seen by SOME lane — replace the Cypress ghost | App Jenkinsfile | 🔴 ghost prints ✅, never ran a2o | stage runs `@act:ii @browser`-safe a2o or is deleted and routed to genesis |
| D4 | apex seed accepted by doorway-B — `PUT /admin/seed/blob` 200 | T4 (operator credential) | 🔴 403 since 08-25 | 200; App build stops going UNSTABLE |
| D5 | `/deliver` verdict `delivered` ×2 (one fresh trigger) for "landing + one lesson through both doorways" | `/deliver` | ⚪ no recent verdict | 2 consecutive |
| D6 | `just status habits` shows 0 `not-measured` for green habits | `habits-status.py` | 🔴 8 habits not measured | 0 |
| D7 | **delivery**: a person opens elohim.host and alpha.elohim.host, sees the same landing and completes one lesson; the resilience card tells the same truth on both | `/deliver` + saga ch.10 | 🟡 saga 10 green household; fleet skipped | delivered ×2 |

Open: 6 of 7.

## Lane S — SDK surface (persistence · sync · REA · projection; the ambient promise)

| # | Rung — check | Pawl | Reading | Target |
|---|---|---|---|---|
| S1 | typed wire client fresh — `schema:codegen:ts` / `export_bindings` freshness | pre-push | 🟢 446 views, gate-enforced | hold |
| S2 | manifest grammar validates every domain — `schema:validate` over `domains/*/manifest.json` | pre-push | 🟢 9 manifests | hold |
| S3 | `crates/elohim-sdk` reads AND writes against a peer it does not run — edges 1/2/8 closed; `flush()` errors surface | T2 (`just test sdk` vs the mesh doorway) → T3 (vs doorway-alpha) | 🔴 "neither works against a public endpoint today" | round-trip green on mesh, then alpha |
| S4 | two apps cannot collide — a `contentType` is accepted only if a registered manifest declares it | T1 storage test | 🔴 unpoliced; `h_app_id` hardcoded on the sync plane | write refused with a named reason; `h_app_id` derived from the manifest |
| S5 | an app declares a REA coupling and the substrate emits the event unasked — signal-harness conformance test per manifest | T2 | 🟡 harness enforced app-side (Angular); `action` free-form | one conformance scenario per domain manifest, run by `just test mesh` |
| S6 | CLAIM / BECOME / RENEW runtime-typed (the Jenkins/GitHub primitives) | T1 | 🔴 manifest-only (`sdk/README.md:98-119`) | typed + one scenario each |
| S7 | an app declares a view/projection without a Rust PR | T1 codegen | 🔴 fixed `INTERFACE_FILES` | manifest-declared view compiled through the existing ts-rs seam |
| S8 | governance + economics ambient on every object — `reach-enforced-everywhere` ∧ `identity-cross-signed` ∧ `custodial-authority-answerable` green | habits | 🔴 red · red · unwired | all three green — this is the "barely think about it" rung |
| S9 | `create-elohim-app --template=<shape>` scaffolds from a manifest (platform spec step 3; `app-port --scaffold`) | T2 (scaffold builds + `just test mesh` on it) | ⚪ unlanded | one template builds and passes its own scenario |
| S10 | **delivery**: a second app, composed outside the `elohim-app` shell, served as an EPR app through both doorways with its objects on the quilt | T3 → T4 → `/deliver` | 🔴 none (avodah is internal) | delivered ×2 |

Open: 8 of 10.

## SDK-readiness overview — how much is left, per plane

The SDK's promise decomposes into four planes; a developer can stop thinking about a plane only when its top rungs are locked.

| Plane | Ambient promise | Proof today | Left before "you barely think about it" |
|---|---|---|---|
| **Persistence** (content-addressed, custody-backed) | write once, your peer holds it, the quilt keeps it | S1/S2 locked; `blob-durability` green; RS(4,7) + `ShardRole` landed | S3 (public-endpoint round-trip), S4 (tenancy), R9 (durability curve measured), `StorableBytes::Ciphertext` (private path) unbuilt |
| **Sync** (converges, no server arbitrates) | authored anywhere, readable everywhere within a round | F1 locked; P1 locked; saga 11/11 household | R4–R8 (plateau named and survived; fleet measurable), P3/P4 (iroh pull path + self-awareness), P6 (DHT-authored content projected), F7 (reach on the CRDT plane) |
| **REA** (every value-touching act is a bounded, witnessed event) | declare a coupling, the event appears | S5 partial (app-side harness); `bounded_by` commitments live; `bridges/valueflows` live | S5 substrate-side conformance, S6 (CLAIM/BECOME/RENEW), P7/P8 (attribution rides a cross-signed binding), F6 (adopt-before-author fed) |
| **Projection** (doorways make truth legible; never own it) | serve through any doorway, identical answer | F2/F3 locked; SSR trust-scoped cache | F4/F5 (uniform deploy without the per-host crutch; seed ≠ admin), F8–F10 (reach on every egress; fleet measures it), S7 (app-declared views), D3–D5 (a lane that sees a browser) |

**Counts (open rungs):** R 9/11 · P 9/10 · F 7/10 · D 6/7 · S 8/10 → **39 open of 48**, 9 locked. The three rungs that unlock the most downstream: **R5** (names the plateau — gates R4, R6, R7, F6), **S3** (first honest developer round-trip — gates S9, S10), **F9/D3** (a fleet lane that measures anything — gates F10, D5, D7).

## The moves (what advances which rungs)

Each move = one `/shift` Objective; local-mesh-first; ~10 commits per batch; ONE Jenkins pass; `[edge:validate-only]` for measures; quiesce four-leg preflight before any fleet read. Design-gated moves invoke `p2p-design-gate` first.

**M0 — pawls (Leg 0)** → D2, P5, R3. `just mesh storage-restart|conductors-restart` arms (`justfile:203-214` → `hc-mesh.sh:1822-1827`); `just seed apply mesh content` profile from `mesh_seed_env`; T2 receipt banner in `.husky/pre-push.bash` after project detection (~`:452`, warn-only precedent `:607`); recovery timeline written under `genesis/a2o/reports/` (gitignored, survives `/tmp`); un-`@wip` `sovereign-peer-join` sc1+5. Measure: R3 durable, P5 passing, banner fires.

**M1 — name the plateau, survive saturation (Leg 2a)** → R5, R4, R6, F6. `syncVerdicts` + metric; N=3 baseline with `syncVerdicts` read at the plateau; verify injected admission pressure produces the never-answered streak (AdmissionShed is excluded from it, `projection_reconcile.rs:748-755, 944-948`); content-leg attempt timeout adapts to admission wait (same signal as `AdaptiveBatchBudget`, `reconcile_rails.rs:404-412`; circuit unchanged); phantom HOLD with cooldown (chapter-11 pin-retirement pattern). One validate-only confirm.

**M2 — fleet measurability + legibility (Legs 1, 2b)** → R8, R10, F9 (first cut), D3. `QUIESCE_MODE=label` in `run-dataplane-validation.sh:80-90`, `churn_state` stamped at `:139-162` (gate predicate untouched; PASS-line scoping already landed `fleet-quiesce-gate.sh:373`); SIGSEGV handler in storage `main.rs`; doorway `routes/storage_proxy.rs` per-hop timers + doorway-B PodMonitor label; `warm_stream` re-replay suppression; `@act:ii` siblings for the read-only saga chapters and notary/served-head scenarios; replace the App pipeline's Cypress ghost with the a2o `@act:ii`-safe browser set (or delete and route to genesis). Surfaces D4 (credential) and the P1 cure cost model for the operator.

**M3 — trust-priced sync (Leg 3, design-gated)** → P7, P8, F7, S8 (two of three). Read the fleet binding counters, decide joins-vs-bindings, decide the dual/iroh transport half; flow-note the stale `first_move`. Handshake carries the steward agent key + membership CIDs; receiver verifies via `hc_registry.imagodei_client()` (`hc_client_registry.rs:151`; `P2PNode.hc_registry` `p2p/mod.rs:767`) using `get_membership_by_action` (`qahal_coordinator.rs:596`); relationships need a new coordinator-only `get_relationship_by_action` (hot-swap). `peer_class` keys the per-peer fetch window (`sync_fetch_windows` `p2p/mod.rs:788`, opener untouched), backoff, race-fetch provider order, admission priority. CRDT-plane enforcement is a wire-shape decision (positional msgpack `sync_protocol.rs:358,382`: versioned `DocumentInfoV2` vs reach-scoped topics per `p2p/topics.rs`) — land the F7 falsifier scenario now, the enforcement after the decision. Relationship graph derived on adoption.

**M4 — dual/iroh harvest (Leg 4; parallel with M3, disjoint write set)** → P3, P2, P4, R9. Route `drain_acquisition_queue` through `race_fetch_dual`/`iroh_fetch_leg`; re-run before deciding whether spec row 14 exists; baseline matrix N=3 incl. fanout rows; `PathObservation` + `select_path` inside `peer_map::select_transport` (`peer_map.rs:462`) with C4 tests; re-run; fleet confirm only outside noise.

**M5 — federation uniform deploy** → F4, F5, F10, D4 (operator), D5. blobHash propagation to the federation peer (`dataplane-peer-fallback-and-blob-replication` D5) so sc2 goes green and `stageSpaBlob` retires; `require_seed_authority` fix deployed (sc3 un-`@wip`); `/deliver` run on the landing + lesson promise.

**M6 — SDK first honest round-trip** → S3, S4, S5. Close `elohim-sdk` edges 1/2/8 with a `just test sdk` family that runs the crate's quickstart against the mesh doorway, then doorway-alpha (T3); storage refuses an undeclared `contentType` with a named reason and derives `h_app_id` from the manifest (design-gated; the entity is A — Content — and the guard is a projection-time verdict, no new entry type); one signal-harness conformance scenario per domain manifest.

**M7 — the second app (the SDK's delivery rung)** → S6, S7, S9, S10. Recommended shape: **WordPress-like publishing** (Site/Post/Page/Comment/Subscriber → Content EPR + FeedbackSignal + Commitment; the horizon doc already has the entity map; needs no CLAIM runtime) composed as `domains/<app>/manifest.json` + a minimal EPR-app bundle served through both doorways, scaffolded by the first `create-elohim-app --template`. A Jenkins-like (build claims) or GitHub-like (PR adjudication) shape waits on S6. Khan Academy is lamad. *Which shape is the operator's call; the rung is the same either way.*

## Sequencing

M0 → M1 → M2 → {M3 ∥ M4} → M5 → M6 → M7. Lanes advance concurrently; the lowest open rung per lane is the only in-flight work in that lane. Every shift close re-counts the five lane tables and writes the delta in the habit atom(s) it moved, then `habits-project.py`. WIP fence at kickoff: `dataplane-convergence` → `active: true`, `doorway-failover` (green, run-identified) → `active: false`.

**Operator decisions surfaced, not made:** D4 (doorway-B `API_KEY_SEED`), the P1 DHT-discovery cure (index walk vs read-miss fallback), shem CPU, the alpha iroh flip, the CRDT reach wire shape (F7), the second app's shape (M7).

## Verification (per rung, the command is the rung)

- T0/T1: `just gate elohim-storage` / `just gate doorway`; new tests red-on-old-code via `cargo test` (`EXIT=$?` on its own line).
- T2: `just mesh start` (dual) → `just seed apply mesh content` → `just test mesh '<scope>'` → `just mesh quiesce` → `just mesh recovery-matrix` → `just status saga` / `just status habits`.
- T3: `just dev conductor profile=alpha` → `just test mesh '@concern:sovereign-peer-join'`; `just test sdk --target alpha`.
- T4: one `[build:edge] [edge:validate-only]` per move after the quiesce preflight; read via Jenkins MCP, never trigger via MCP.
- Lane close: `/deliver` for D7 and S10.

## Decomposition

This spec decomposes into the moves M0–M7 above; each move is authored as a `/shift` Objective whose measure is the rung set it names. The lane tables are re-counted at every shift close and the delta is written in the habit atom(s) the move touched. No plan document is minted for the campaign as a whole — the moves are the plan, one per shift.
