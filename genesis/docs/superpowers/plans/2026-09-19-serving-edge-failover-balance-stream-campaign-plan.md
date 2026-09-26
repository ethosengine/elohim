---
title: "Serving edge campaign — failover, balance and streaming proven at both layers, then the ingress retires"
id: serving-edge-failover-balance-stream-campaign-plan
status: Draft
domain: D8
sprint: "unranked drain rung — not named in vision-readiness-sprint-roadmap; composes plans A/B/C below; S1 also lands in D5 (elohim-storage)"
cites:
  - doorway-federation-failover-sprint-plan | Doorway Federation & Failover Sprint | sha256:c66fd04c3b4f16e2 | path: genesis/docs/superpowers/plans/2026-07-31-doorway-federation-failover-sprint-plan.md
  - federation-edge-plan | Federation Edge | sha256:3c47bfddaae214ca | path: genesis/docs/superpowers/plans/2026-06-14-federation-edge-plan.md
  - doorway-federation-three-reds-to-green-plan | Doorway federation | sha256:d2b8f066817b690f | path: genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md
  - doorway-membrane-prosocial-routing-design | Doorway Membrane & Pro-Social Routing | sha256:50dd8febb5447fbb | path: genesis/docs/superpowers/specs/2026-06-20-doorway-membrane-prosocial-routing-design.md
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md
  - genesis/data/timeline/backlog/head-authority-carried-with-content-sync-unit.md
  - genesis/data/timeline/backlog/conductor-admission-saturated-for-hours-after-restart.md
  - admission-receiver-granted-lanes-design | story 3.3's brainstorm starting frame: receiver-granted reserved lanes, shortest-expected-hold first, AIMD background after restart (TCP/Homa lessons from the 2026-09-23 publish storm) | sha256:54611915e41292b9 | path: genesis/docs/superpowers/specs/2026-09-23-admission-receiver-granted-lanes-design.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/data/timeline/backlog/projection-reconcile-actionable-sawtooth.md
  - genesis/data/timeline/backlog/ci-orchestrator-baseline-advance-despite-failure.md
  - genesis/data/timeline/backlog/ci-apex-doorway-cannot-reach-adam-storage-blob-forward.md
  - genesis/data/timeline/backlog/ci-substrate-projection-pull-stream-dark.md
  - genesis/a2o/features/dataplane/doorway-apex-transition.feature
  - genesis/a2o/features/dataplane/doorway-failover.feature
  - genesis/a2o/features/federation/name-routing.feature
---

# Serving edge campaign

> **For agentic workers:** this is a SEQUENCING plan. It orders stories and fixes what each must prove. A story
> is detailed to task level (superpowers:writing-plans shape, one file per story under this directory) only when
> it is next — detail written earlier goes stale against a fleet that moves nightly. Execute a detailed story
> plan with superpowers:subagent-driven-development.

**Goal:** failover, load balancing and reactive streaming each proven between peers and between doorways — on the
household mesh first, then on the alpha fleet — so that the k8s ingress can be replaced by doorways standing
anycast-style behind a name's DNS records, with a CDN in front of what is immutable.

**Architecture:** the doorway is a thin, plural projection over the mesh; it may answer by hostname, relay to the
holder, and withdraw itself from a name. It never chooses byte targets and never replicates its shed state. Every
story moves a named habit and is specified by an a2o scenario. Nothing here designs on the ingress.

**Tech stack:** Rust (`elohim/elohim-storage`, `doorway/doorway-service`, `relay-addr-beacon`), a2o Cucumber
stories under `genesis/a2o/features/`, the household mesh (`just mesh …`), Jenkins edge/app/genesis pipelines.

**Specs this argues from:** plans A `2026-07-31-doorway-federation-failover-sprint-plan`, B
`2026-06-14-federation-edge-plan`, C `2026-09-10-doorway-federation-three-reds-to-green-plan`; spec D
`2026-06-20-doorway-membrane-prosocial-routing-design`. This plan composes them; it does not replace them.

## Global constraints (binding, quoted from the sources)

- D:213-215 — "capability→peer selection logic is substrate-native (D1) … doorway carries capability routing only
  on the identity-hosting axis (D8). Conflating the two — a doorway-resident byte target-chooser — is the one move
  that breaks the constitution." Doorway selection is over **doorway holders of a name**, never over byte holders.
- A:107 — doorway shed/admission state must NOT be replicated between siblings. A sibling's state is **observed**
  (probe, response header), never synced.
- A:53 / A:114 — federation-seam doorway→doorway HTTP is allowed; doorway → other peers' storage fan-out is not.
- B:313 — "CDN … scoped to `/blob/<hash>` ONLY … Do NOT CDN any EPR-head/view route (mutable)." B:197 — mutable
  routes get a short TTL.
- B:312 — "Recommend NO automatic failover" for the apex was written for the ingress era; A:86-90 grades the
  options: multi-A + client retry in scope, a managed load balancer is an operator-owned borrowed dependency
  needing an exit-doctrine ledger row, BGP anycast is "filed vision, not scheduled".
- D:117 — TLS "AT doorway (self-terminated, cert-manager/ACME — under sovereign control)". A:256 — the doorway's
  Ed25519 node key is generated fresh at every boot; a persisted key "must land first".
- A:72 — the cross-edge coherence probe is warn-only; `routes::coherence::CoherenceView` is THE detector (B:41).
- MAP:92 — "Doorway is OPTIONAL, not architectural — the D5 mesh is the hosting layer." MAP:263 — where a
  document and a probe disagree, the probe is the authority.
- Operator, 2026-09-19 — doorways are hostname-aware AND stand anycast-style behind a name's DNS records, serving
  failover for each other; they subsume every ingress concern because the ingress goes away.
- Repo rules — story first; household mesh before fleet; a flip needs a run id or a build number; one push per
  batch; cluster operations are the operator's.

## What is true today (2026-09-19, measured)

| | between peers | between doorways |
|---|---|---|
| failover | household-proven (churn 3/3, peer-loss 4/4); `blob-durability` green | household 10/10; fleet 3 passed / 0 failed (edge/dev 1465, 1467); **red** — the pair serves two landing heads |
| balance | admission gate fleet-proven but saturates: matthew + adam hold every permit for the 60 s timeout; replica scoring has no data sources | liveness order only; reach / nearest / weight are `TERM_NOT_YET_WIRED`; the ingress balances |
| streaming | libp2p announce-on-change household-proven; iroh announce code-only (dual mode announces on libp2p alone) | conductor→doorway push exists; doorway↔doorway is HTTP pull + manual refresh; pull stream dark on alpha |

Ingress concerns with no doorway home: TLS + certificates (not built), body-size / WebSocket timeouts / sticky
sessions. Public DNS holds one A record per name, so the beacon's withdrawal (household-proven, poll-driven) has
nothing to withdraw from on the fleet.

Statements in A–D that today's facts contradict are listed at the end; they are corrected here, not there.

## Sequence

Each story names: **habit** it moves · **story** that specifies it · **proof** required · **gate**.

### Sprint 1 — one head, wherever you ask (habits: doorway-failover, dataplane-convergence)

The fleet cannot author a head through either public doorway, and two bounded loops are the prime suspects. Clear
them, then close the zero-lag head oracle, then make the pair comparable.

- **1.0 The import handler reaches its conductor, or stops asking.** Measured 2026-09-19 (Loki, 14 h flat, all
  seven alpha peers): every 5 s each storage opens the conductor admin API, lists apps, mints an app auth token,
  then is refused on the app socket — ~700 cycles an hour per peer, and no import path works anywhere on the
  fleet. Cause: the conductor binds its app interface to `127.0.0.1:4445` in its own pod and a bridge re-exposes
  it on 8445 (`manifests/humans/_edgenode-conductor.template.yaml`); `import_handler.rs` pairs the admin URL's host
  with the conductor-LISTED port instead of the configured `HOLOCHAIN_APP_URL` its sibling `HcClient` already
  uses. Fix: carry the configured app URL into `ImportHandlerConfig`, back the reconnect off (5 s doubling to a
  5 min cap), demote the token-shape dumps from ERROR. Habit: dataplane-convergence. Proof: unit red→green;
  fleet — `"Import handler connection failed"` falls from ~700/h to 0 on every peer. Gate: `just gate elohim-storage`.
- **1.1 Held reanchor candidates back off.** The sweep's pre-flight probe is hardcoded `AdmissionClass::Interactive`
  (`conductor_writes.rs:948`) — a background sweep borrowing the lane a person's read stands in; it moves to
  `Background` in this story. Task-level plan drafted 2026-09-19 (rust-architect): new
  `services/reanchor_backoff.rs` keyed on `(id, advertised head)`, runtime-config
  `REANCHOR_HELD_BACKOFF_SECONDS` (default 900), metric `elohim_content_reanchor_skipped_total{reason}`. Backlog head-authority item 7: `reanchor_backfill` re-probes 29–43
  held candidates every ~70 s forever, each a failing `declare_canonical_head` call. Habit: dataplane-convergence.
  Proof: household — failing-declare rate for held candidates falls to the backoff schedule; `/p2p/status`
  `provideLoop` shows it. Gate: `just gate elohim-storage`.
- **1.2 A contested declaration that already stands is not re-minted.** Item 6: the same 28 ids re-declared every
  ~20 min. Habit: dataplane-convergence. Proof: household — declares per content id per hour ≤ 1; the ~20-min
  `divergent_actionable` sawtooth amplitude is re-measured and recorded either way.
- **1.3 Fleet read of 1.1 + 1.2.** One edge deploy. Proof: `elohim_conductor_admission_in_flight` on matthew and
  adam below capacity with mean hold under 1 s, sustained 30 min after the roll; sawtooth re-measured. If the
  ceiling does not move, the finding returns to the operator (pool-conductor capacity / hosted-cast size) with
  the two loops excluded as causes. Gate: operator-visible; no further fleet story starts until this reads.
- **1.4 The head arrives with the content.** Item 1: sync unit `{content, signed head record}`, verified once,
  adopted atomically. Story: `doorway-apex-transition.feature` "A current governed version crosses withdrawal and
  recovery", step `doorway "elohim.host" serves authority B's exact head…` — a single read, no polling, by design.
  Habit: doorway-failover. Proof: household apex-transition 3/3. **Gate: p2p-design-gate** (a signed head record
  crossing the sync plane is a data-entity decision) before any code.
  *Design gate run 2026-09-20* (genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4-gate.md): no new DHT entry
  type and no DNA-hash move — the head `Record` is an existing notarized artifact carried as evidence, verified by the
  receiver's own conductor (`validate_carried_record`), and it rides as one new key (`headRecord`) inside the Automerge
  content doc because the sync wire is positional msgpack and cannot take a field without a two-phase rollout. The
  adopt arm already stamps head + anchor + content patch in one transaction; 1.4 makes that reachable on the fast path.
  **The live red is a torn row, read from both public doorways the same day:** both declare head `uhCkkEBj4…lGBP0K`;
  doorway-alpha's `dhtAnchorHash` IS that head (blob `9a0bae…`, 2026-09-14), while elohim.host's `dhtAnchorHash` is a
  later Update `uhCkk6StXD9…LYjNb` and it serves blob `3bf228…`. So elohim.host serves bytes its declared head does not
  vouch for.
  **Correction, same day — the mechanism first written here was wrong, and the implementer was right to stop**
  (genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4a-blocked-finding.md). The own-commit projection does NOT
  tear a row: since 2026-09-08 `upsert_with_anchor_transaction` withholds `blob_cid` / size / metadata from a declared
  row when the committed action is not its head (`content_diesel.rs:1138`, pinned by
  `late_preserved_snapshot_cannot_replace_canonical_browser_or_server`), and the deploy PATCH declares and writes content
  in ONE transaction — there is no separate declare step to be shed. Loki agrees: adam's 18:01:37 projection of
  `uhCkk6StXD9…` carried a THIRD blob, `a39c96…` (that day's staged landing); the guard withheld it and only the anchor
  moved. adam's row was already serving `3bf228…` at 02:59 that morning — the tear predates the day's deploys, and the
  day's staged landing never became the declared head on either doorway. What CAN tear a declared row is a head MOVE
  that carries no blob pointer; the code has two: the `ContentHeadDeclared` signal arm
  (`rea_projection.rs:1032`, `patch: None`, `StampMode::LegacySignal` — permitted exactly when the deploy PATCH has just
  NULLed the declaration ordering) and `adopt_local` on a move (`head_adoption.rs:1811`, metadata-only patch). Every
  other head-move site carries a full patch. The existing cure for an already-torn row is `pointer_heal_patch` (T7),
  which never reads the anchor and fires once the local conductor answers `canonical: true` — so the fleet row is
  waiting on conductor health, not on new code.
  **1.4a, re-aimed (needs its own design pass before code — three documented regressions live in this function's
  history, `content_diesel.rs:1531-1594`):** `stamp_declared_head_mode` declines to MOVE a declared row without a
  pointer-bearing patch, which turns the first producer into a no-op and makes the second fetch the pointer it defers.
  **1.4b:** the carried record in the doc. Honest-view and probe work rides with 1.4a: the head view calls a torn row
  `notarized` because `trust` is derived from anchor presence alone, and the authority for "which doorway is wrong" is
  the decoded `blob_cid` of `/head-record`, not the anchor.
  *1.4a design pass 2026-09-20* (genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4a-design.md): **proceed
  reshaped — the rule above is wrong as written.** A guard keyed on the patch's blob pointer would freeze head
  convergence for the corpus: 3 770 sampled content items carry zero blob pointers (only app bundles have one), and
  `ContentProjectionPatch.blob_cid: None` means "preserve the column", not "no blob" (`content_diesel.rs:922-938`).
  **T-1 (the cure):** `adopt_local` carries `blob_cid` + `content_size_bytes` from the conductor answer it already
  holds, as its three sibling call sites do — no fetch is needed. **T-2 (the backstop, its own commit, after a
  household reading of T-1 alone):** `LegacySignal` / `HealCanonical` decline a move that carries NO PATCH AT ALL,
  counted under a new `pointer_absent` reason; this overturns the pinned backward-compat tail at
  `content_diesel.rs:5198-5225`. **T-3:** the head view gains an additive `pointerFromDeclaredHead` (the `trust`
  vocabulary, shared with `ContentView`, is left alone) and seam 6 reads the head record's `blob_cid`.
  **1.4d (new, unscheduled):** 1.4a prevents new tears and heals none — no candidate query selects a row because
  it is torn; a torn row classifies `InSync` (`projection_reconcile.rs:4454-4476`).
  *Household reading 2026-09-20* (fresh recast, storage `3ec3614dd` build, reports `sprint-report-household-20260920T22{3158,3619,3958}Z-3ec3614d`):
  T-1 landed (`77cf2ba48`) and the pair converges — by the final read all five endpoints served one head and one
  blob, and seam 6's head-record probe read `OK` on both doorways. But "A current governed version crosses
  withdrawal and recovery" is 0/3, as it has been since 2026-09-18 (habit DELTA 09-18a): its load-bearing step is a
  single read taken immediately after publish, which a ~1 s adoption cannot satisfy. **That scenario is 1.4b's
  acceptance, not 1.4a's** — the design pass named it for T-1 in error, and T-1 was never going to flip it. T-1's
  proof is its red-first unit test and the absence of a torn row on the converged pair; T-2 proceeds as its own
  commit on that basis.
  *Scheduled 2026-09-22:* a bounded pointer-audit sweep (`services::pointer_audit`, new module) heals the class
  1.4d names — a declared row whose blob pointer no longer names the same blob as its own declared head's own
  conductor record. It walks declared+pointer-bearing rows on a round-robin keyset cursor
  (`content_diesel::list_declared_blob_pointer_candidates`), probes this node's own conductor
  (`AdmissionClass::Background`, never Interactive), and runs the SAME guarded T7 heal
  (`pointer_heal_patch` + `stamp_declared_head_mode(.., StampMode::HealCanonical, ..)`) `adopt_local` already
  applies to a row some other path selected — never authoring, declaring, contesting, or moving a head. Metric
  `elohim_content_pointer_audit_total{outcome}`; spawned on the same late-connect guard as the re-anchor backfill,
  gated by `ELOHIM_POINTER_AUDIT` (default on). The fleet's own torn row (edge #1474 seam-smoke,
  `ADVISORY-SAME-HEAD-DIFFERENT-BYTES`) cannot be healed by this sweep until it heals independently of code: both
  doorways answered `UNREADABLE-HEAD-RECORD reason=cell-disabled` in #1474, so this node's own conductor cannot
  yet produce the record the sweep reads FROM — it is waiting on cell health, not on this landing.
  *Sprint 1.4 landed on `dev` 2026-09-26 (fleet reading owed after the push):* **1.4b** redesigned after a second
  review (genesis/a2o/reports/recovery/serving-edge-20260926/story-1.4b-r-design.md) — no doc key; operator ruling:
  the author's publish mints its own election, and a sibling verifies courier-carried evidence read-only, admitted
  only on authoring standing (root author or its delegate) over a root it holds (`f2ffa5df4`, `06e9587f8`). Bytes
  come before the move on every adoption path (courier, AdoptLocal, sweep obey arm — `0fd34e4ed`). **1.4c** (the
  serve path's doc-blob preference) is removed in `06e9587f8`. The SQL ordering carries the election's link-hash
  tiebreak (`18354ca53`); an iroh-only peer raises the adoption trigger (`34336f041`); **T-3**'s honesty field
  shipped as `anchorMatchesHead` — named for the anchor check it is, not the byte check `pointerFromDeclaredHead`
  would claim (`31438f8d4`). Household: the station passes 6/6 at 701–1873 ms (was 47–59 s, or never). Still
  frontier: steward/affiliation roles as standing (they live in eprfs, not verifiable in wasm); composite peers do
  not fall through on `Timeout`.
- **1.5 The pair is compared.** *Landed locally 2026-09-19:* the existing seam-smoke `dht-fetch` seam compared only
  `headActionHash` and printed CONVERGED on edge/dev 1465 while the pair served two blobs. It now also compares the
  served `blobHash`; live it reads `ADVISORY-SAME-HEAD-DIFFERENT-BYTES` — **one notarized head, two blobs**. That is
  the sharpest form of the red: the row's `blobHash` moves by a per-doorway PATCH that the head action does not
  carry, which is exactly the "per-host imperative write" dataplane-convergence forbids and what 1.4 retires.
  Original scope, kept for the record: `verify-projected-head.sh` passes per host because its expected hash is `auto`.
  Add a pair leg that reads both doorways' `GET /api/v1/federation/coherence` and the landing row and prints one
  `pair head: SAME | DIFFERENT (a=… b=…)` line, warn-only per A:72. Habit: doorway-failover. Proof: the line in
  an edge build; the habit's same-head clause becomes a probe reading instead of a manual curl.
- **1.6 App delivers behind a roll.** The app pipeline's head-authoring stage waits on write-readiness (admission
  below capacity) instead of a fixed 360 s, and the orchestrator consults per-pipeline success so an app retry
  does not re-roll edge (backlog ci-orchestrator-baseline, piece 2; piece 1 is the staged 09-14 work awaiting its
  owner). Proof: app + genesis green from one push with edge untouched.

### Sprint 2 — a name outlives its doorway, on the fleet (habit: doorway-failover)

- **2.1 Beacon legs can serve more than one shared name.** A:205-217: `--shared-record-name` / `--record-owner`
  are single-valued, so one leg cannot contribute to both `doorways.elohim.host` and the apex, and the staged
  beacon manifest silently reverts apex ownership. Story: extend `doorway-apex-transition.feature` membership
  scenario to two shared names on the household `file` sink. Proof: household.
  *Corrected 2026-09-20:* the code half was already done — repeatable `--shared-record NAME=OWNER` landed in
  `906d7b159` (2026-08-23) and per-lane state in `2a10415d5`; A's evidence predates both. What was open, and landed
  in `04abd05ef`: the two-name scenario, the file-sink lane-isolation pin, and the staged manifest's apex posture
  stated with its operator-gated flip. The manifest's exclusive apex claim agrees with the coturn manifests today;
  it bites only if the coturn side moves the apex to a shared lane first. Household run still owed.
- **2.2 A doorway answers for a name it is a member of.** With the ingress out of the path (household), a request
  for the sibling's name is served under this doorway's contract or relayed to the holder with
  `x-elohim-served-by` — and resolves the same head (needs 1.4). Story: `name-routing.feature` + a host-bound
  contract scenario. Habit: served-under-standing.
- **2.3 Fleet: two doorways behind one name; one really sheds; the name keeps serving.** **Operator-gated**
  (A:190-198, C Task 21): DNS multi-A, the beacon manifest, and the ingress host-conflict check are cluster
  operations. This plan delivers the manifests and the runbook; the operator applies. Proof: a beacon log of
  withdraw → re-admit against a real shed, and an outside client that never saw an outage.

### Sprint 3 — balance by something real (habits: served-under-standing, conductor-capacity-represented)

- **3.1 One selector term, observed not replicated.** `services/name_routing.rs` `SELECTOR_TERMS`: wire `Weight`
  from what a holder already says about itself on the response path (its catching-up shed / `Retry-After`,
  `X-Available-Permits`), remembered with decay. No sibling state sync (A:107). Story: a `name-routing.feature`
  scenario where the shedding holder is demoted and promoted back. Proof: household.
  *Designed 2026-09-20* (genesis/a2o/reports/recovery/serving-edge-20260919/story-3.1-design.md): `X-Available-Permits`
  is emitted by elohim-storage only, never by a doorway — the one honest observation is a relay reply that
  declares backpressure with the holder's own `Retry-After`, honoured as the demotion window (clamped 5–300 s,
  binary rank, no probes; demotion reorders, never excludes). `note_shed` already demotes a shedding holder today
  but `replace_all` forgets it at the next 60 s refresh — surviving the refresh is the story's whole content, so the
  scenario (`a07134562`, `@wip`) spans one. The household cannot stage it yet: it needs a third doorway (`gamma`)
  and a holder able to declare a window longer than the cycle (the real sheds declare 2 / 20 / 30 s). The shed
  fixture must NOT be gated on `dev_mode` alone — alpha runs `DEV_MODE=true`, which would make it a public
  denial-of-service lever; it takes a dedicated boot flag set only by the household launcher.
- **3.2 Peer replica scoring gets its first data source.** `services/serve_routing.rs` admits `current_load`,
  `delivery_score`, `attested_rtt_ms` have none; `transport_paths.rs` already keeps an EWMA RTT. Feed it. Story:
  `serve-routing-rtt-ordering.feature` loses `@wip` with a topology the household can actually vary. Habit:
  dataplane-convergence.
- **3.3 Admission degrades as a curve.** The design question from backlog conductor-admission (reserved
  interactive capacity vs background ceiling vs cold-start pacing; heartbeat call sites classed `interactive`).
  **Gate: brainstorm**, then its own plan. Scheduled here, may be pulled forward by 1.3's reading.
  *Design input 2026-09-23* (specs/2026-09-23-admission-receiver-granted-lanes-design.md): the 2026-09-23 alpha
  publish storm, read through TCP and Homa. The recommended frame is a receiver-issued grant over reserved
  lanes: an interactive floor that background work never takes, shortest-expected-hold first, AIMD background
  concurrency after a restart, and a `Retry-After` derived from the measured hold. It protects person-facing
  calls. It does not cure a conductor-internal storm; that is borrows-backlog row 17. The brainstorm starts here.

### Sprint 4 — doorways hear each other (habit: doorway-failover; D:192-194 "designed-for, not built")

- **4.1 The pull stream runs.** Backlog ci-substrate-projection-pull-stream-dark: `projection.streams pull=false`
  on every alpha pod. Find why; light it. Proof: fleet `/health` shows the stream up.
  *Corrected 2026-09-20:* nothing is dark. The CI label `projection.<peer>.streams` reads `/p2p/status` `.pull`,
  which is the per-node content-pin ACQUISITION rollup (`elohim-storage/src/p2p/acquisition.rs`), not a
  doorway↔doorway stream: `caughtUp=false` with `total==0` is the designed idle state of a pod nobody has pinned
  content on, ungated by any env var, and the backlog item resolved it as an assert-ghost on 2026-06-11
  (`substrate-verify.sh` already warns `idle` instead of failing). What remains of 4.1 is the emitter-side
  `state: idle|active|caughtUp` field. The doorway↔doorway content-projection stream this sprint is named for does
  not exist to be lit — building it is 4.2.
- **4.2 Projection-index replication doorway↔doorway, reach-earned, pushed.** Replaces
  `POST /admin/federation/peers/refresh` as the freshness path; the refresh verb stays as the operator's
  actuation twin. Backpressure is bounded and lossy with the poll as backstop, the shape
  `p2p_iroh/announce_change.rs` already uses. **Gate: p2p-design-gate.** Story: new, in `features/federation/`.
- **4.3 iroh peers get the doorbell.** Dual mode announces on libp2p only, so an iroh-only peer waits the 60 s
  round; measure it, then announce on both. Habit: dataplane-convergence.

### Sprint 5 — the doorway terminates its own TLS (habit: doorway-failover retire-path; D:117)

- **5.1 The doorway's node key persists across boots** (A:256 precondition).
  *Landed locally 2026-09-20* (`a9910d11c`): `DOORWAY_NODE_KEY_FILE`, load-or-generate, corrupt file aborts boot,
  first-writer-wins publish; household wired. Sessions already survive a restart (HS256 from `JWT_SECRET`
  everywhere) — this stabilises the JWKS/DID identity only. The fleet has nowhere to keep the file: doorway pods
  mount only an `emptyDir`; a volume or mint-once Secret is an operator manifest, owed before 5.3.
- **5.2 rustls listener beside the plain one**, certificate from a file pair — the household mesh serves
  `https://…elohim.local` with a locally minted CA. Story: `served-shell-boots.feature` over https.
- **5.3 Issuance and rotation.** DNS-01 is the only challenge that works for a name several doorways share, and
  the beacon already holds the DNS writer — so the certificate for a shared name is requested by a member and
  proven through the beacon's owner-stamped TXT record. **Gate: brainstorm + red-team** before any code.
- **5.4 The ingress's remaining settings get doorway homes:** body-size budget
  (`deployment/ingress-body-size-budget.feature`), WebSocket idle timeouts, and client affinity (which anycast
  cannot promise — the doorway must not need it).

### Sprint 6 — a CDN can stand in front (habit: doorway-failover; B Tasks 2–3)

- **6.1 Validators, not caching, on mutable routes:** `ETag` / `If-None-Match` on EPR-head and shell HTML with a
  short TTL (B:197). Nothing mutable becomes CDN-cacheable (B:313).
  *Landed locally 2026-09-20* (`1103207e4`): EPR-head GET carries a strong ETag over the exact body, 304 on
  `If-None-Match`, `no-cache` (`private, no-cache` when not anonymous), `Vary: Authorization, Cookie`. The shell
  half is narrow: a chrome-injected page carries the request's omnibar context and stays `no-store` with no
  validator, and injection fires for nearly every real shell — a revalidatable shell waits on 6.2's render key.
- **6.2 The shell is addressed.** The HTML entry names only `/blob/<hash>` assets (already `immutable`, one
  year); SSR output for an anonymous commons request is keyed by `{serverBlobHash, route, reach}` so it can be
  revalidated instead of re-rendered. Story: `served-shell-boots.feature` asserts every asset URL is addressed.
- **6.3 B Tasks 2–3 drained** (the `/blob/*` cache stanza, `DEPLOY_VERSION`), still gated on X-EDGE-DEF.

### Then — the ingress retires

Preconditions, all as probe readings: 1.5 says SAME; 2.3 has a real withdraw/re-admit on record; 3.1 demotes a
shedding holder on the fleet; 5.3 has rotated a certificate without an operator; 6.2 holds. The retirement itself
is an operator act with its own runbook — this plan ends by handing over that checklist.

## Order and parallelism

1.1 → 1.2 → 1.3 is strictly serial (one fleet read for both fixes). 1.4 needs its design gate and can be designed
while 1.1–1.3 run. 1.5 and 1.6 are pipeline work, independent of the Rust stories. Sprints 2–4 each have a
household-only first story that can start once 1.4 lands; every fleet story waits on 1.3's reading. Sprint 5 and
6 touch only doorway-service and can interleave. Never two fleet stories in one deploy — the reading is lost.

## Corrections to the source plans (probe beats prose)

- A:75, A:53, A:199-200 — multi-A for a shared name is not live: public DNS has one A record per name, and the
  ingress at each premise pins a name to one doorway, so apex client fallback cannot reach the sibling premise.
- A:205-217 — "`--shared-record-name` is single-valued, a second flag silently last-value-wins": evidence dated
  2026-08-18; the repeatable lane landed 2026-08-23 (`906d7b159`). A Task 3.1's `check-ingress-conflicts.sh` is at
  `genesis/orchestrator/scripts/`, not `scripts/ci/`.
- A:155, C:1480 — same-declared-head reads green on the household; the fleet pair serves two heads.
- B:12 — per-host head islands "by construction": every live projection contract is any-host now.
- C:1546, C:1632 — evidence stamped `face02de` / `2c338124a` predates the fleet's `a6ba209e2`.
- C:581 — "the only membership sink is Cloudflare": `relay-addr-beacon/src/sinks/file.rs` exists and the
  household apex-transition scenario runs on it.
- D:116, D:117, D:128-131, D:206 — anycast, GeoDNS, self-terminated TLS and "the ingress is a test-bench" describe
  the destination. Today the ingress is load-bearing production routing.

## Complementary work captured, not planned here

- The conductor ceiling itself on matthew and adam (operator lever; backlog conductor-admission).
- Flow control borrowed from TCP/Homa after the 2026-09-23 publish storm (backlog arch-dataplane-borrows rows
  16–17): receiver-granted doorway→storage blob forwarding with no blind retry, and conductor publish
  backpressure in the holochain/kitsune2 fork (telemetry first).
- `delegated-sweettest.feature` returns `pending` without its fixture and fails strict validate-only runs.
- No gate runs `scripts/ci/*.test.sh`.
- An untracked `resource_limit_raise_test.py` (2026-09-13) fails any push touching `.claude/scripts/_lib`.
