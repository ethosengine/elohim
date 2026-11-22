---
title: "Native developer CI/CD over peers — the plan that subsumes GitHub, Nexus, Harbor and Jenkins"
id: native-dev-tooling-over-peers-plan
status: active
class: process-meta
date: 2026-10-29
steward: human:matthew
author: "Claude (fork of the overnight integration-capacity shift), at the operator's direction"
serves: [push-delivers-within-budget, sprint-lands-each-closure-once, push-gate-pays-once, workspace-device-arrival, devices-share-one-berth, build-earns-reach-by-attestation]
refines: closure-keyed-landing-plane-design
tags: [rakia, brit, lvi, epr-peer, berth, ark, onboarding, native-ci, scaffold]
cites:
  - "closure-keyed-landing-plane-design | the design whose slices this plan sequences into the concern table (build, attest, reuse, carry, landing) | sha256:9156f36238a93744 | path: genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md"
  - "steward-space-landing-plane-design | the steward space's Arm A/B rungs that the onboarding story (fork an lvi workspace, join, carry jobs) graduates through | sha256:53718b5898db3b15 | path: genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md"
  - "restart-to-connected-sprint-plan-20261026 | the prior sprint on arriving as a device and reconnecting peers — the reliability floor this plan assumes | sha256:d2c664288eedeaed | path: genesis/docs/superpowers/sprints/2026-10-26-restart-to-connected-sprint-plan.md"
  - genesis/research/integration-capacity-grounding-2026-10-29.md
  - elohim/rakia/docs/plans/README.md
  - elohim/rakia/docs/plans/stage-2-canopy.md
  - elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md
  - elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md
  - "native-dev-plane-one-derivation-earned-reach-design | the 2026-11-04 design pass whose rulings §6 applies to the concern table | sha256:024a89c3dba2699f | path: genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md"
---

# Native developer CI/CD over peers — scaffold

> **Operator ruling (2026-10-29).** "Whatever we can do to improve the elohim native developer CI/CD
> design over peers to subsume github, nexus, harbor, jenkins.. is worth doing now regardless of when
> I can supply the compute. We should master this rather than throwing hardware at a problem that
> would just benefit from more elegant peer utilization of compute."
>
> **The finish line, in the operator's words.** "Onboarding a new dev with one workstation natively
> (brit, lvi, peer): they could just fork a lvi workspace, and they join the cluster, and are
> available to run integration jobs if they leave their peer on in the network."

This is a scaffold: every row below states what is wired today with the file that proves it, the
target shape, the slice that moves it (the design's S-ids, the steward-space spec's A-ids, or rakia's
own stage sprints), the habit it serves and the gate that proves it. Rows marked `pending` have no
slice text yet and take shape as the sprints settle. Nothing here is a new register: the habits,
`@concern:` tags and gates are the four artifacts CLAUDE.md names.

## 1. The concern table

| # | Concern | Wired today (proof) | Target | Slice | Habit | Status |
|---|---|---|---|---|---|---|
| 1 | **Build declaration** (the pom) | `build-manifest.json` × 21, validated by `elohim/rakia/schemas/v1/build-manifest.schema.json`; typed `run` kinds (cargo, root-just, attested), declared `inputs.sources`, `cargo.env` caps; read by `gate-runner.mjs` and `graph-walker.mjs` | Every step declares the inputs its closure key hashes, and the derivation script reads the manifest, never a Jenkinsfile | design S1a (closure-input-digest reads declared sources + COPY set); rakia Stage 1 sprint 2 | push-gate-pays-once | partly wired |
| 2 | **Build execution** | Jenkinsfiles, imperative (edge 195 KB at the CPS cliff); local gates via `run-local-gate.sh` | `rakia build` executes a manifest's steps; a Jenkinsfile stage is `sh 'rakia build <step>'` | rakia Stage 1 sprint 5 | push-delivers-within-budget | pending |
| 3 | **Change detection and selection** | `rakia affected` / `plan` / `ci` / `baseline` (binary installed); brit/gix change detection; baselines as git notes-refs | Unchanged: this is already native; the orchestrator's already-built filter keys on closures (S3b) | design S3b | push-delivers-within-budget | wired |
| 4 | **Closure-keyed artifacts** | tonight: `closure-<key16>` tags + config labels on storage/doorway images, `harbor-oracle.sh` hit/miss | DNA pack keyed and re-tagged (S2c); app bundle keyed; one derivation per step | design S1a, S2c | push-delivers-within-budget | S1a landing |
| 5 | **Artifact custody** (Harbor, Nexus) | Harbor by tag with `imagePullPolicy: Always`; cargo `elohim` registry on Nexus (`elohim-epr`, `epr-rea`, eprfs family 0.3.1; pre-push version-collision precheck) | Harbor and Nexus behind rakia `Registry::has_artifact` by digest; the bundle carrier (≤1 MiB CIDv1 chunks named by a manifest entry) as dataplane custody for images, crates and packages — source of truth per the design §3 minted #3: class B2, identity = the artifact digest, closure tag a secondary index, `node_registry` `CustodianAssignment` the existing entry type for custody once it leaves Harbor | design A7b/S3a; rakia executor spec §4.2 | sprint-lands-each-closure-once | pending · next sprint: the elohim-native store, acceptance I1–I7 (§5) |
| 6 | **Attestation** | brit local `recordEvidence` (private notes, never pushed); Jenkins `brit-helper.sh attest` wired but inert | One writer per device at harvest time: brit `ci:<pipeline>` notes from Jenkins's archived graph, `inputs_hash` = closure, `output_cid` = digest; the inert Jenkins arm deleted | design S2d' | push-delivers-within-budget | pending |
| 7 | **Evidence reuse across devices** | A8 sibling verdict (`gate-runner.mjs:463-486`), closure keys equal on both devices (D4); live two-device receipt owed; `GATE_EVIDENCE=reuse` 5,757 s → 56 s locally | First `independence: sibling` row on a real push; eprfs/orchestrator keys hit after the corpus split | design S2a, S2b | push-gate-pays-once | live between one steward's devices: a sibling's green stands in (host-compatibility plan G1, landing 39; 16 of 21 gates reused on a re-push, 2026-11-21) |
| 8 | **Carried gates** (peer runs the job) | `just measure` rung H on jessica (12–15 s guest work); rakia executor source on the pin, unbuilt; `ark task run` never run here | A gate neither device paid is claimed by the device whose own ram-guard/io-guard lines admit it, run under `ark task run` in its envelope, verdict folded back; requester waits 0 s when no carrier is present | design S3a; measure-runs-on-a-peer rung A | sprint-lands-each-closure-once, measure-runs-on-a-peer | Shadow by default; landing 13 on 2026-11-22 carried elohim-storage on the second seat in 1,384 s, completing 3,653 s of gates in 2,311 s. |
| 9 | **Landing** (GitHub's role) | pre-push claims `dev-integration` before gates and holds it through them (p90 3,321 s); GitHub ref is the CAS | Phase V gates holding nothing; Phase L holds the lease for a 600 s CAS window; intents are atoms; a silent sibling is overridden by rule; GitHub keeps only the final compare-and-swap | design S1b (A9) | sprint-lands-each-closure-once | morning handoff · landing-queue sprint in flight: S1 (`epr flow intend`, `epr peer intents`) and S2 (decision modules) landing 2026-11-01 |
| 10 | **Jenkins's role** | the only dispatcher, builder, deployer and (nominal) attester | Witness and one builder among many: dispatches, builds what no device carried, never judges policy, never reads a lease | design §2; steward-space A10 (deferred until `runtime-death-witnessed` is green) | push-delivers-within-budget | in motion |
| 11 | **Tool distribution** (brit, epr, launchers) | device bin store: `brit` on PATH is `bin-launch` → content-addressed release (head `bafyreiai7…`); `epr device arrive` reinstalls launchers from `current` (R1) | Releases published to and admitted from the dataplane, not GitHub; auto-update exercised on both devices | restart-to-connected R1/R2; backlog "brit handoff" | workspace-device-arrival | distribution native, publication pending |
| 12 | **Device arrival and the daemon** | `just status device` nine lines; `epr peer serve` not yet supervised; a Che restart cost six manual steps (2026-10-25) | `epr device arrive` is one step to latest and connected; the daemon is an ark `processes[]` child, death-witnessed, relay-independent | steward-space A4; restart-to-connected R3/R4 | workspace-device-arrival, devices-share-one-berth | in motion |
| 13 | **Onboarding a second principal** | roster rows travel by git; `epr actor device enroll \| authorize \| bind`; only `human:matthew` is real, every other human a fixture | Fork an lvi workspace → enrol → the device's head folds on the steward space → its verdicts count as an independent lineage at `trusted` | lvi `DevspaceSeed`; steward-space S2 (collective space, `binds-signer`) | workspace-device-arrival | pending |
| 14 | **Dispatch at community reach** | eprfs-peer over iroh inside one steward's grant | claim / dispatch / result over storage's libp2p swarm for closures already attested at `trusted`; `rakia-peer` crate | rakia Stage 2 (Canopy) | — | pending |
| 15 | **Compute as contribution** | berth leases as ephemeral observations; `delegates-compute` mishpat commitment exists for a second principal | A carried gate is a `Commitment{Produce}` with the carrier's envelope as its bound; recognized through the same REA events as content; stewarded, never free-riding in either direction | design §3 minted 5; rakia Stage 3 (shefa) | — | pending · §5 (requests and offers) |
| 16 | **Capacity read natively** | tonight: cost vector on every cycle row, ledger union, berth fold and reap, the stage-wallclock sensor alive | Each device's carrying-capacity vector declared once in its ark RuntimeManifest and read by dispatch; `Envelope::demand` beside `bound` | design S1c, S4 | push-gate-pays-once, conductor-capacity-represented | one answer on a device, 2026-11-21: `berth room cargo`; nothing about capacity crosses between devices |

## 2. Ease of operation — the moments, and what each should cost

| Moment | Today (measured) | Target | Proof |
|---|---|---|---|
| Arrive after a restart | six manual steps (2026-10-25); launchers wiped with `/opt/rust/cargo/bin` | `epr device arrive`, one command, no cargo | `just status device` all ok after `kill -9` + restart |
| Run the gate for what I touched | `just gate` (serial; storage 642–2,613 s; holds `dev-integration` throughout) | same verb; pays once per closure; holds nothing; a sibling or carrier may have paid already | gate-cycle rows with `independence: sibling` / `carried` |
| Land on dev | push, lease held through gates, ref-lock rejections after long windows | push → intent → CAS window ≤ 120 s p90 | berth fold in `berth status` |
| Validate a feature branch | PR + `[edge:gate-only]` tag (until tonight) | any branch off dev/main: gates + local builds, publishes nothing, by rule | `non-dev-publishes-nothing.test.mjs`; `oras repo tags` unchanged |
| Know what it cost | seconds only; p90 from a fragmented ledger | cpu / rss / throttle / io on every row; one union | the five `--env` slots on a cycle row |
| Add a device | enrol + bind + populate the bin store by hand | fork a workspace, enrol, serve | the second device's head folds within one `fold` |

## 3. Sequence

1. Sprint 1 (in flight): rows 4, 16 (α, γ landing tonight); row 9 (β) handed to the morning.
2. Sprint 2 (re-ranked 2026-11-01, §5): first the hit-rate preconditions (reuse that holds), then row 5's store design pass with I1–I7 as its acceptance tests and its first slice between two of this person's workspaces; rows 7 (S2a receipt), 4 (S2c DNA), 6 (S2d' attestation on the device) and 12 (A4 supervised daemon) ride beside it.
3. Sprint 3: rows 7 (S2b corpus split), 3 (S3b), 8 (S3a the carry), 5 (A7b bundle custody as the carry's carrier).
4. Sprint 4: rows 16 (S4 demand), 11 (brit off GitHub), 13 (second principal).
5. Rakia Stage 1 sprint 5–6 (rows 2, 6) run beside sprints 2–3: `rakia build` is what lets a Jenkinsfile stage become one line.
6. Rakia Stage 2 (rows 14, 15) after a second principal exists.

## 4. Open questions (fill as they settle)

- Row 2: does `rakia build` execute the manifest's `run` kinds directly, or shell to `run-local-gate.sh` first (reuse the recipe, replace the walker)?
- Row 5: which artifact class leaves Harbor first — images (already closure-tagged) or crates (already on a native registry)?
- Row 11: what signs a brit release on the dataplane, and what admits it on a device that has never seen GitHub?
- Row 13: the acceptor set for a second principal's first verdict (D5's three gaps: `binds-signer`, signer admission, signature verification).
- Row 15: the REA event shape for a carried gate when the carrier and requester are the same principal (today) vs two (later).
- Hardware: the media PC (Ryzen 5, 16 GB, no TLC drive yet) can carry one storage gate at a time once the pool has a drive; until then it is a sibling-verdict peer. The laptop likewise. Neither is a Jenkins agent.

## 5. Reconciliation 2026-11-01 — the store's gaps and sccache's traps

> **Operator direction (2026-11-01).** The next sprint fills the artifact store's gaps and is
> positioned so the failures that made sccache a blocker cannot recur: "we want to avoid the bugs that
> caused sccache to become a blocker to our development when we implement our store." The ranked rows
> live in [the workspace-discipline cluster](epr:arch-workspace-discipline-backlog), rows 71–77.

**What this sprint makes shareable.** The landing-queue sprint (row 9) makes *verdicts* shareable and
the landing order safe. It does not move *bytes*. Moving bytes between workspaces is row 5: the
elohim-native store (whole derivation keyed by gate closure, a brit `build put --output <bafkrei>`
record, bytes in the eprfs tier store, reach under a `custody-blob` pledge born `self`, REA
Produce/Use/Consume). It is not sccache with another backend.

**sccache's failures as acceptance criteria for row 5** (sources: [sccache-garage-harden](epr:sccache-garage-harden), the CI anti-patterns museum row 5):

| # | Invariant | The sccache failure it closes |
|---|---|---|
| I1 | A cache fault is never build output: every draw ends `hit(verified bytes)`, `miss` or `unavailable(why)`; bytes are CID-checked on read and only a hit reaches the consumer | an S3 404 fed into rustc's stdin probe as null bytes ("unclosed delimiter") |
| I2 | Empty is cold, never broken: no sentinel or init object exists | a bucket wipe removed `.sccache_check` and *caused* the symptom |
| I3 | Poison cannot persist: one key class (closure → output CID through a signed record); a bad record is superseded | probe entries survived `SCCACHE_RECACHE`; heals were a key DELETE or a full repave |
| I4 | The store is always optional: absent, slow, uncredentialed or emptied means build locally and print why | `RUSTC_WRAPPER` set on binary presence; a dead Garage key and the 2026-08-23 metadata loss failed builds |
| I5 | No wrapper on the compile path: artifact grain sits above rustc | ~1.7% of compiles failed to spawn the wrapper (ENOENT) under parallel load |
| I6 | A hit names its signer and closure; reach widens only on converging independent derivations | a CID proves the bytes, not that they are a correct build — sccache had no answer |
| I7 | Every draw is a flow event; hit rate and fetch-vs-rebuild time are measured | failures were invisible (optimistic success) and re-diagnosed each time |

Tests owed before slice 1: wipe the store, corrupt one blob, kill a fetch midway, revoke the
credential. Each must give byte-identical output and a printed reason.

**Gaps to close first** (the arc's own critics, plan `pasted-content-id-3a40-are-some-virtual-nova`
§ Staged): the custody pledge does not reach the tier store; the Mishpat `Commitment` payload is
confused with the elohim-DNA REA `Commitment`; slice 1's first act PUTs to a server the device may
not run (a local-first refusal); brit signer, `computed_by` and `agent_cid` are unjoined namespaces;
the 454 MB storage binary exceeds the 64 MiB frame and the tier door has no `serve-blob` Use writer.
Also A7b: a sibling's commit objects are absent from the local odb.

**Hit-rate preconditions** (a fast store with a near-zero hit rate is still slow): no gate writes a
tracked file (the eprfs gate appends to `.claude/data/architecture-findings.jsonl` today); one
`// gate-reads:` must not mask a walk in the same function; undeclared inputs refuse in seconds; eprfs
leaves tree scope (12 unresolved reads after S1). Cluster rows 73, 76 and 74.
**Met 2026-11-03/04.** Rows 73, 74, 76 and 80 are closed. eprfs keys on paths: 222 reads, its
governance walk named as `**/.epr-meta`. A clean-worktree reuse was measured at 35 min 32 s, then
22 s. Thirteen more cargo gates are enrolled for reuse. The oracle again plans dependents with
sibling worktrees present (row 81; rakia#3, brit#4/#5). Baseline detail: spec
native-dev-plane-one-derivation-earned-reach §11.

**Requests and offers (rows 8 and 15).** A landing intent is already a ValueFlows `Intent` its
pusher fulfils; `just measure` sends a stage to a named provider under a grant. A market adds a job
published as a provider-less Intent, capacity offered as Intents, a match as a `Commitment`, and
completion as Produce plus Use, with the result's reach widening as independent derivations
converge (I6). Trust prices it: own devices reuse cheaply, a stranger's output pays a re-derivation
premium. Shefa's requests and offers are the home. Cluster row 77, after row 5 holds.

**Throughput.** Above 64 MB, elohim-storage already erasure-codes blobs (RS 4+3) and `blob_swarm`
spreads shard fetches across holders, so committed holders add their upload for large derivations
and any four of seven shards rebuild (the slow peer is skipped). Compile-unit objects gain nothing
from sharding; between two workspaces on one cluster, locality and hashing dominate.

## 6. Reconciliation 2026-11-04 — one derivation, earned reach

> **Operator direction (2026-11-04).** A design pass from the primitives (epr, eprfs, `.epr-meta`) to
> brit and rakia, before returning to row 5. "Native" means the integration and build queues live
> between peers. Jenkins is a participant, and external runners are mirrors or peers, never
> bottlenecks. Builds graduate through the social-reach primitives. Spend is revealed by epr-rea, and
> deliverables are derived from mishpat. Wall-clock removed from push → landed breaks ties, aimed at
> the prize.

The rulings and their evidence are in
[the native dev plane design](epr:native-dev-plane-one-derivation-earned-reach-design). Where a row
above disagrees with this section, this section governs.

- **Row 1.** Each manifest step declares a `TreeView` (sources, audited gate-reads, gitlinks) and a
  method descriptor (recipe, toolchain, env). The projected view is both the key and the sparse
  checkout (D2).
- **Row 2.** `ark task run` is the only executor. A Jenkinsfile stage becomes `ark task run <method>`.
  rakia Stage 1's executor sprint retires (D8).
- **Row 3.** "Wired" was overstated: reuse goes dark without the deprecated `rakia` binary. Track B
  moves selection to `brit build affected`.
- **Row 4.** Closure-keyed artifacts become `DerivationRecord`s keyed (method CID, projected root).
- **Row 5.** The record is `DerivationRecord`. Cache bytes are a local `custody-blob` pledge (B, born
  `self`). Release holder sets use the elohim DNA's `CustodianCommitment`/`BlobToReplicas`/`ShardManifest`
  links. `node_registry::CustodianAssignment` is struck. Slice 1 starts with a **local** tier put
  (track E). The spec's §10 adds eight requirements to I1–I7 (D6).
- **Row 6.** One `DerivationRecord` per device, signed by the brit signer. brit notes carry records.
  The Jenkins attest arm is deleted, not adapted (D1, track C).
- **Row 7.** `gate-verdict@1` carries record bytes. The two-device receipt is join F1r, labelled
  `independence: sibling` and never counted toward reach (D4).
- **Row 8.** The eprfs-peer carry is the only offload path, and its result is `basis: UnderGrant`.
  `just measure` is re-plumbed onto it and the `feedback_signal` path retires. The carry needs source
  materialization by projected view, not output custody (D9, join F4).
- **Row 9.** The steward-space signed `dev` head is the landing record. A silent sibling cannot block a
  landing. The head is a set of tips, so a partitioned double landing keeps both tips durably until a
  merge containing both lands. The forge mirror only fast-forwards and halts while a fork stands. The
  receipt includes that interleaving (D3, riding S1b).
- **Row 10.** Jenkins is an R2 carrier and witness under a `delegates-compute` grant. Its records stay
  `self`.
- **Row 11.** brit, epr, rakia and ark publish as `release-manifest` channel heads. One admission rule
  covers every tool, including the verifier itself.
- **Row 13.** Struck: "the device's verdicts count as an independent lineage". Instead, after a
  cross-signed signer → `agent_cid` binding and standing in the space, the *principal's* records count
  at `trusted`.
- **Row 14.** The record is `DerivationRecord`. "rakia names, the tier serves", which supersedes
  steward-space §5.1's "Rakia serves".
- **Row 15.** Same-principal carrying is provider = receiver = `human:matthew`. `Work` is added to
  `ReaVerb`. Gate cost moves from `--env` slots to `Use` quantities. Changelogs, trailers and DELTAs
  become projections over fulfilled commitments.
- **Row 16.** Add the missing liquidity facts as folded measures: refusal rate by guard axis, lease-hold
  p90, and availability history. Liquidity feeds carry admission, never reach.

**Sequence.** Five parallel tracks on disjoint write-sets:

- A: eprfs-core slice 4 (`project_view`). This is foundational.
- B: the gate oracle.
- C: the brit record (integrity).
- D: honesty gaps.
- E: the tier blob store.

Then come the joins: F1r (live sibling reuse), F2 (a sibling fetches the storage binary instead of
rebuilding), F3 (S1b with the landing CAS), F4 (carry), F5 (one release plane and deliverables as
reads), and F6 (a second principal). The prize is that two workspaces build once between them. Every
later rung widens that record and path; none replaces it.

## 7. Reconciliation 2026-11-05 — Radicle/Heartwood into the roadmap, and the artifact-store sprint handoff

> **Operator direction (2026-11-05).** Before the artifact-store sprint, read Radicle and the
> Heartwood protocol for prior-art blockers and hard-earned lessons; then ground in the epr, brit and
> rakia primitives and synthesize those lessons into this roadmap, ending with the handoff for the
> sprint's implementation plan. "The artifact store on our p2p dataplane touches the very foundations
> of the Elohim Protocol."

The survey is [Radicle / Heartwood](epr:radicle-heartwood-cross-pollination-2026-11-05) (36 claims
adversarially verified; Codex cross-family, Claude integrity and completeness reviews). The grounding
reports behind this section are kept under `genesis/local-dev/radicle-survey/` (epr/brit, rakia and
landing-plane lineage, custody lineage). §6 governs where this section is silent; where this section
and the survey's own "What this changes" list differ, this section governs, because it is adjudicated
against §10 of the design and D6, which the survey read only partly.

### 7.1 The guidestars this sprint is built from (in the authors' words)

- "A CID tells you what the bytes are. An Elohim EPR tells you what they mean, who cares for them,
  and who said they could be here" (`protocol-specification.md:29`). Bytes by CID, meaning by record.
- Three planes, one truth: control (commitments, attestations, elected heads), data (blobs, shards),
  projection (SQLite, doorway, any forge-like view), and the projection is "never the source of
  truth" (seam map §7).
- "rakia names, the tier serves" (design §3; §6 row 14). Rakia is a manifest over the dataplane,
  never a store or a registry service.
- "Which release applies is a DECLARED head, never recency" (runtime-artifacts spec); "Peers add;
  they never gate"; "Reach is the build's only trust scale" (design §1).
- "The record is evidence, never authority … an accepted record whose output bytes cannot be fetched
  is a miss" (`eprfs-core/src/derive.rs:5-7`).
- "A peer is never silently conscripted" (custody phase 3); "Custody may exist … but confers no
  authority whatsoever" (inalienable red-team); "Custody commitments are the only notarized objects"
  (swarm curve and blind custody).
- Value flows back to every steward: every put is a `Produce`, every draw a `Use`, accounted to the
  true principal (`human:matthew` on the real network) with persona, session and device as provenance
  (steward-space A5; `eprfs-peer/src/local_blob.rs`).
- The test for every move (design §0): if every external runner and the forge disappear, do two
  workspaces still integrate and build between themselves?

### 7.2 Where each Radicle lesson lands

The survey's fourteen lessons sort into four bins against §10 and D6. **The most important
correction to the survey:** its headline lesson (a push-time replica count is not custody) is true at
every reach, but D6 splits the mechanism by reach. At `self` and `intimate`, custody is a local
`custody-blob` pledge with zero DHT heads, and the only gate is I1: a hit needs CID-verified bytes. The
non-publisher, failure-domain, freshness-counted custody gate the survey asks for belongs to the
**release channel head** (the runtime-artifacts election; join F5), not to Track E's local put. Track E
must not grow a replica count.

| Bin | Lessons | Where it lands | What changes |
|---|---|---|---|
| **A. Already satisfied by the design or the code** | L4 lineage and parent binding (D7 body/sig framing and epochs; the release manifest's `channelId` and `lineageParentCid` hint verified against the channel chain); L10 versioning (`TIER_BLOB_MANIFEST_EPOCH`, `UnsupportedEpoch`); L3 **on the tier path**: `eprfs-storage`'s directory store already writes temp + fsync + rename and the two-writer race is tested (`eprfs-storage/src/lib.rs:307-343`, `durability_tests.rs`); L5's "record before success": `put_local_blob` returns `LocalBlobUnrecorded` rather than reporting success without its event; L11's "never delete": §10.1 counters a bad record with `Recomputed{matched: false}` | Track C, Track E (as tests) | Add the survey's tests to the receipts: a replayed older envelope refused as stale; an envelope signed for channel A presented on B refused as grafted; a manifest at an unsupported epoch answered `unsupported-manifest-version`, never "not found" |
| **B. This sprint (Track E + F2)** | L5 output binding (`record.output` is the artifact's raw CID; the manifest CID is a different value; a swap must be refused); L7 chunked, resumable, multi-source transfer (§10.11; the 454 MB storage binary); L8 budgets (per-artifact stall and total deadlines, a manifest size cap checked before allocation, losing racers cancelled, wasted bytes metered); L9 reachability as a per-path fact; L14 full CIDs only in manifests (schema rejects bare hex) | Track E slices 2–3, join F2 | §7.3 lists them as acceptance tests |
| **C. elohim-storage hardening, a separable lane, before any artifact rides the community-reach dataplane** | L2 holder-identity binding: `handle_inventory` applies a sender-declared `peer_id` never checked against the gossipsub-signed author (`p2p/gossip_dispatch.rs:410-445`; dispatch carries only the relaying `propagation_source`, so the fix threads `message.source`), and `reconcile/custody.rs:507-510` counts honored custodians from those rows; L3 on the `BlobStore` path: non-atomic inline writes, "already exists" by metadata probe, unverified inline reads (`blob_store.rs:160-172, 286-299, 606-624`); L6 Stage-2 inventory signatures as a staged floor (declared level, dual-read, adoption metric, per-peer monotonic floor); L8's detached losing race tasks (`blob_fetch.rs:231-240`) and the 64 MiB frame | New rows in [arch-dataplane-borrows](epr:arch-dataplane-borrows-backlog) (holder binding, budgets) and [arch-native-dev-plane](epr:arch-native-dev-plane-backlog) (`BlobStore` atomicity); habit `blob-durability` | Not a Track E blocker: the tier ALPN admits only the person's own devices, authenticated by the handshake, and the tier store already writes atomically. It **is** a precondition for L1's gate at community reach and a live defect in custody counts regardless |
| **D. The release plane and standing (F5, F6, the standing redraft)** | L1 custody-before-head as a measured contract (≥ k non-publisher custodians over ≥ d failure domains, counted from authenticated, unexpired evidence; typed `under-custodied`; bounded repair; no conscription); L11 yank / supersede / revoke as distinct operations decided by the channel's standing body; L12 authority lineage retrievable after membership changes; L13 divergence as a typed conflict for a named election, admission by the target channel's rule | `release_adoption` and the custody controller at F5; the standing redraft (design §13) and F6; rows in [arch-native-dev-plane](epr:arch-native-dev-plane-backlog) and [commons-holonic-stewardship](epr:commons-holonic-stewardship-backlog) | This sprint writes the a2o scenarios (`@wip`) so the gate is specified before the release plane is built; it does not implement the gate |

Two survey verdicts are **narrowed** here. Survey D5 ("one store, the tier as a view through
`BlobStoreTier`") becomes: **one byte store per device**, served by the tier daemon (§10.12), where a
device that runs elohim-storage backs the tier with `BlobStore` through the existing `BlobStoreTier`
adapter and a workspace without it uses the eprfs directory store. The difference between a `self`
cache and a released object is control-plane (a local pledge versus DNA holder sets, D6), never a
second physical copy on the same device. The BLAKE3 `IrohBlobStore` is the one real third store and
needs a ruling (§7.4 R1). Survey D3 ("a generic channel head moves only when D2 holds") is scoped to
wider reach, as above.

### 7.3 Handoff: the artifact-store sprint (plan row 5, Track E slices 2–3, join F2)

**One sentence.** A sibling workspace fetches the mesh storage and doorway binaries (the 454 MB
chunked case) from the steward's other workspace over the tier, verified chunk by chunk and whole,
instead of rebuilding them, with every draw a `Use` and every build a `Produce`, on one byte store per
device, and with nothing served that the requester's grant does not cover.

**Habit.** The sprint's DELTA lands on `sprint-lands-each-closure-once` (its prize is this join) and,
for the hardening lane, on `blob-durability`. No new habit is minted; `materialize-what-is-read`
(eprfs) names the walk the fetch rides.

**Preconditions, refused before launch, each with its cure.**
1. brit's `build find` is dark in this repository since 2026-11-04 (gix 0.81's 32-slot object store
   against 49 pack indexes; `push-gate-pays-once` DELTA 2026-11-05). Cure: pin brit `9776a9ad95`
   (`fix/gix-slots-after-trust`) under the pin-attestation gate. Without it F1r cannot be measured and
   F2's record cannot be found by its key.
2. Track A's `project_view` (slice 4) and Track C's signed `DerivationRecord` are in flight on their
   own write-sets. F2 integrates on them; the Track E slices do not wait for them.
3. The sibling device is up (`epr peer status`; it was silent ~30 h on 2026-11-04).
4. The one-device I1–I7 failure tests from Track E slice 1 are green with zero peers and with no
   daemon (wipe, corrupt, kill a fetch, revoke a credential; the daemon-down put and read).

**Slices, in order (write-sets disjoint from the hardening lane).**
- **E2 — A11 tolerant reader** (`eprfs-storage`, `eprfs-core`): mixed-epoch manifests decode per D7;
  an unknown epoch is held and answered `unsupported-manifest-version`; unknown `ext` bytes retained.
- **E3 — `serve-blob` mount with scope enforcement** (`eprfs-peer`): the tier daemon mounts the blob
  store beside its channel store; a `serve-blob` `Use` writer; the gate checks root and object CIDs
  against the requester's grant (`eprfs-peer/src/gate.rs:14,64` ignores both today). Receipt: an
  admitted carrier asks for an unrelated CID and is refused.
- **E4 — one authoritative payload store per device** (gated by H2, R3.3): `BlobStoreTier` composed
  under the tier on a device that runs elohim-storage; the directory store elsewhere; a test that the
  same CID is never held twice on one device by the two paths; transport indexes and bao outboards may
  reference the payload, never copy it (R1.4). No new table, entry type or route: the bytes are Ephemeral (C), rebuildable
  by re-running the method; the `DerivationRecord` is Attested-Private (B2) in brit notes and the tier
  manifest; the custody pledge is a local `custody-blob` commitment (B, born `self`) that graduates to
  the existing elohim-DNA `CustodianCommitment` / `BlobToReplicas` links only at release reach (D6).
- **F2 — the sibling fetch**: `just mesh build` outputs captured into the tier as declared method
  outputs; the sibling resolves the `DerivationRecord` by key, fetches the chunk closure over the tier
  (multi-source where a second holder exists), verifies each chunk and the whole against
  `record.output`, and records `Use` (fetch) against the builder's `Produce`. Receipt: fetch versus
  rebuild as `Use` versus `Produce`, the 454 MB case, in each transport mode the tier supports, with a
  mode that cannot deliver reported as a typed gap.
- **Stories first.** The R2 and R4 a2o scenarios are written `@wip` under `@concern:artifact-custody`
  beside the F2 scenarios, so the release-plane gate is specified now: custody admission at Trusted
  reach, `pending-election` with its reason, `under-custodied` and repair, quarantine versus refuse,
  consent withdrawal, `no-eligible-head`, and the mixed-version refusal of a policy epoch a reader
  cannot interpret.
- **`adoptionDiscipline` gains a policy epoch** (D7) before any restrictive field — custody admission
  (R2) or lifecycle cells (R4) — is read; an older reader that cannot interpret the epoch cannot admit
  an election under it.

**Hardening lane (elohim-storage write-set; runs beside, with explicit acceptance dependencies —
R3).** E2 and directory-backed E3 do not wait on it; H2 gates E4's `BlobStore` composition and every
F2 receipt that rides it; H1 gates every use of remotely advertised holdings as custody evidence.
- H1 holder-identity binding, every ingress: on libp2p the payload's holder identity must equal
  gossipsub's signed `message.source` (threaded through dispatch, which carries only the relaying
  `propagation_source` today; relays preserved); on iroh the payload carries end-to-end holder
  authentication; snapshots, deltas, custody announcements (`gossip_dispatch.rs:795` projects
  `holder_agent_cid` unbound) and salvage ads all enforce binding, freshness and replay before any
  projection changes; custody counts read only bound, unexpired claims plus R2's commitment and
  verified-fetch evidence. The authenticated envelope lands here under a declared level and D7 epoch;
  universal rejection of legacy advertisements follows measured adoption, and until then legacy rows
  are fetch hints that never satisfy a custody floor. Claude-family adversarial review before landing.
  Tests: a spoofed-holder snapshot is refused and changes no custody count; a replayed old snapshot
  does not re-base the cursor; relayed snapshots still apply; a forged iroh snapshot naming another
  holder is refused.
- H2 `BlobStore` durable, verified publication across representations: inline (temp + fsync +
  rename), chunked (chunks and `.chunks` index prepared and synced before the `CHUNKED` marker;
  directory entries synced), shards (verified against shard hashes, never the composite); "already
  exists" means "exists and verifies"; verify-on-read for inline blobs; a corrupt object is quarantined
  and the node keeps serving the rest; staging and quarantine files are excluded from `list_hashes`,
  tallies and serving; existing identities and layouts stay readable. Tests kept distinct: crash
  mid-write, competing writers, deletion race, `ENOSPC`, cleanup failure, power loss after the marker.
- H3 fetch budgets: drop-equals-cancel racing; a per-artifact budget with stall and total deadlines;
  a manifest size cap before allocation; wasted bytes metered.
- H4 `parse_content_address` refuses a CID whose multihash is not sha2-256 instead of reinterpreting
  its digest (`blob_store.rs:228-232`; R1.6); new surfaces refuse the legacy `sha256-<hex>` spelling.

**Acceptance.** I1–I7 and §10 items 1–15 as written, plus: the swap test (`manifest_cid` for
`record.output` and back is refused); the interrupted fetch resumes only missing chunks from the
remaining holder; the trickling holder is abandoned on the total deadline and the chunk re-raced within
budget; the oversized manifest is refused before any chunk moves; the unauthorized-CID refusal; the
stale-replay and wrong-channel refusals; the epoch refusal; a schema that rejects bare hex.

**Out of scope, named.** Reach-restricted or private artifacts (the confidentiality plane; ngit L5);
the read-only OCI gateway (ngit H8, held); gate jobs as requests and offers (row 77); the release-head
custody gate's implementation (F5); Radicle 2.0 on iroh (survey W1).

### 7.4 Rulings (2026-11-05; integrity-class, ruled by the session model after an Astra challenge)

> **Operator direction (2026-11-05).** "Make the calls on these … with a reminder that Elohim Protocol
> is trying to be generational architecture; we do not mind doing the rework to get the foundations
> right. You can dispatch Astra agents to challenge your conclusions."

Method: a draft of all four was challenged by four independent gpt-6-astra legs, one per ruling, each
briefed to refute (`genesis/local-dev/radicle-survey/astra-challenge-R{1..4}.md`). All four returned
REFUTED. The session model re-read every code claim the rulings rest on and accepted the findings
that held; the full record with the findings accepted and the one claim rejected (a schema regex
that does not exist) is `genesis/local-dev/radicle-survey/rulings-final.md`. Where a ruling below and
an earlier line of §7.3 disagreed, §7.3 was amended.

| Ruling | Decision | Rework accepted | Note |
|---|---|---|---|
| **R1 — address and the BLAKE3 store** | The address is CIDv1, self-describing; each object class declares a **canonical emission profile per epoch** (today raw + sha2-256 for bytes and chunks, dag-cbor + sha2-256 for records). **BLAKE3 is an allowed transport and storage hash, not an identity** for artifacts, chunk manifests or `DerivationRecord.output`; admitting a new identity hash is a governed migration. Output identity is canonicalized at production; a locally verified hash equivalence is a rebuildable index that locates payloads and never rewrites records, merges principals, transfers reach or manufactures agreement. Bao verifies the transfer; acceptance verifies the canonical CID. **One authoritative payload store per device**; the iroh-blobs `FsStore` may be that store only after receipts for canonical answer, atomic recovery, index reconstruction, daemon-independent operation, scoped serving and retention across every live reference (its tag-based `forget` must never collect committed bytes); until then pure-iroh artifact delivery waits. Address resolution is not authorization. | Yes: `parse_content_address` reinterprets any CID digest as SHA-256 (`blob_store.rs:228-232`) — lane H4; the "Phase 11 cutover" comment is intent, not a ruling; DNA custody strings migrate with digests preserved; rakia's `blobCid` + `sha256` pair stays until a coordinated transition. | Draft had admitted BLAKE3 as an interchangeable identity and a "transport cache" that was a second copy; both refuted on D5 convergence, D7 and plan §7.2 |
| **R2 — custody admission** | Private, SelfScope and **Intimate create zero DHT heads and require no quorum**; a second device's copy is shown per copy, never a prerequisite. At **Trusted and above** a channel's election is admitted under a **versioned custody admission policy** in `adoptionDiscipline` (own policy epoch, D7) naming the closure, coding parameters, the **tolerated failures** it promises to survive, evidence ages and the retention window; admission demonstrates reconstructibility under those failures; whole-copy custodians and RS shard holders are counted differently; the 64 MiB shard threshold applies to the artifact, never to tier chunks. Evidence that counts = an authorized commitment **and** authenticated, replay-resistant availability (H1) **and** a bounded-age verified fetch attributable to the serving holder; inventory rows alone never count; the publisher's devices never satisfy external custody. Failure domain = a **declared dimension with provenance** (household first); unknown is never independence. Liveness is honest: missing consent or evidence yields a durable `pending-election` reason, blocks no local work, conscripts nobody; later loss yields `unknown` / `under-custodied`, never a head move by itself. | **Profile 0, experimental, to be measured:** Trusted/Familiar survive one whole-copy loss (k ≥ 2, two households); Community/Public survive two (k ≥ 3, two households) or RS 4+3 with four distinct shard indices over two households; Commons survive four (k ≥ 5, three households); inventory horizon 600 s; **repair within 900 s is a measured objective**, missed deadlines reported with cause. Commitments bind to bounded windows; no per-chunk DHT heads. | Draft contradicted D6 and itself at Intimate, lacked a failure model, treated dwelling hub as proven independence, and promised repair the kick limits cannot deliver |
| **R3 — hardening lane** | H1–H4 are mandatory in this sprint on their own write-set with explicit acceptance dependencies: **E2 and directory-backed E3 do not wait; H2 gates E4's `BlobStore` composition and every F2 receipt riding it; H1 gates every use of remotely advertised holdings as custody evidence at every reach and transport.** H1 covers snapshots, deltas, custody announcements and salvage ads, libp2p by signed author, iroh end-to-end, with freshness and replay before projection; the authenticated envelope lands with H1 under a declared level and epoch, and only universal rejection of legacy advertisements waits on adoption. H2 is representation-aware (inline, chunked, shards; staging excluded). Roster admission never authorizes general blob access (E3 keeps its scope gate). | Yes: the draft's "H1/H2 do not gate Track E" recorded the wrong architecture for E4 | Draft refuted on E4's dependency, iroh coverage, the custody-announce ingress, and H2's representations |
| **R4 — release lifecycle** | Lifecycle, current eligibility, serving authorization and custody retention are **separate decisions composed under the constitutional floor**, each act binding subject, scope, issuer, authority evidence, interval, reason and policy epoch. **Supersede**: an authorized election chose another release; re-eligibility only by current qualification and a new election. **Withdraw (yank)**: channel-scoped under verified delegation or the channel's standing rule; prevents new adoption; elects no replacement; grants no serving authority; pinned recovery only under current reach, grant and safety policy. **Standing revocation**: records whose authority ended, when, for which acts; prior releases stay assessable against publication-time standing; a compromise finding names its affected interval. **Security and harm override the defaults** through the existing pair, **quarantine** (expiring hold; blocks adoption, serving, redistribution) and **refuse** (terminal; mandated local destruction); **subject consent withdrawal** removes replication and serving authority; nothing overrides these floors; global recall is never promised. **Heads move only through the election** among presently qualifying candidates; `no-eligible-head` is explicit, `unreachable` is never absence. Custody follows bounded obligations; audit evidence is kept apart from payload availability; **deprecation** is advisory, **expiry** explicit. | Yes: the draft's "yank keeps serving pinned peers", "head moves to the prior winner" and "revoke retains for audit, never purges" are all withdrawn; cells live under a policy epoch with a mixed-version refusal test; general enforcement waits on the standing design (§13) | Draft refuted on the swarm spec's quarantine/refuse contract, the inalienable spec's consent withdrawal, the election design's ceremony, and D7 |

What the challenge changed in §7.3: E4 gated by H2; H1's scope widened; H4 added; the R2/R4 stories
named; `adoptionDiscipline` gains a policy epoch before any restrictive field is read.

### 7.5 Mint pass

Rows minted from the survey on 2026-11-05: [arch-dataplane-borrows](epr:arch-dataplane-borrows-backlog)
rows 23–24 (holder-identity binding with staged Stage-2 floors; fetch budgets and drop-cancel racing);
[arch-native-dev-plane](epr:arch-native-dev-plane-backlog) rows 9–11 (custody-before-head at the release
channel; `BlobStore` atomicity and verify-on-read; one byte store per device with the F2 transfer
tests); [brit-daily-driver-handoff](epr:brit-daily-driver-handoff-2026-10-27) row 6 (output binding and
transactional attestation); [commons-holonic-stewardship](epr:commons-holonic-stewardship-backlog)
row 53 (verify against the standing recorded at publication; a named election with channel-scoped
admission; the revoke matrix as the standing body's call).

## 8. Delivery stations (2026-11-19)

§6 wrote the tracks and joins as table rows and prose. `epr flow project` mints from `- [ ]`
stations and reads neither, so until this section the epic's own sequence had no intents in the
flow. Each track and join is restated below as one station. The states were first read on
2026-11-19 from a checkout 134 commits behind dev, then checked against dev 01e4de7f1: every file
a State line names is byte-identical there, and F1r-producer, F3 and F5 were corrected for what
the steward-device-arrival and native-throughput sprints landed. The design and the receipts stay
in [the native dev plane design](epr:native-dev-plane-one-derivation-earned-reach-design) §11;
this list only makes them claimable.

The sprints that work this epic keep their definition of done and lanes in a journal and their
open rows in the `steward-device-arrival-open-items` backlog, and the projector reads neither.
The open sprint's lines are stations in [the one-baseline plan](epr:one-baseline-plan), which
also carries the rows the native-throughput sprint left open.

Gap ids are positional. New stations are appended at the end of this section; stations are ticked,
not reordered or removed. A tick is a claim that still awaits verification, not a fulfilment.

Two joins have no station here, because the
[host-compatibility plan](epr:host-compatibility-foundation-plan) already carries them and a second
station would mint the same promise twice. F1r's two-device receipt is that plan's G1 (met on
landing 39). F2's fetch between the two devices is its K6, K7 and K12.

- [ ] **A — eprfs-core slice 4 is what the gate keys on: a component's view can be asked for by its `.epr-meta` root, and ark's closure-metadata projection is retired** (serves push-gate-pays-once)

State: `walk_spine`, `project_view` and `derive_or_reuse` exist in eprfs-core (`walk.rs`,
`derive.rs`). ark's `material.rs` still builds its view from the closure's field names over one
metadata node. Since `c71f584f8` (2026-11-22) `ark task source-root` projects a listed set of real source
files, and `GATE_SOURCE_VIEW=shadow` (default off) prints that candidate beside the present key
for the orchestrator gate. First two-device reading at that commit: the candidate root was equal
on shem and ethosengine (812 files) and the present key was not. The gate still keys on closure
metadata; the shadow decides nothing.

Verify: the `P` vectors are green, and ark keys one task on a projected root of real source trees.

- [x] **B — gate selection resolves through `brit build affected`, with no `rakia` name left on the reuse path** (serves push-gate-pays-once)

State: on dev since 2026-11-21 (5535405d4..b74f60096). `genesis/orchestrator/gate-oracle.mjs`
resolves `BRIT_BIN` or `brit` on PATH and runs `brit build affected`; when that cannot run, the
gate says why and selects by path alone. Receipt, on a device with no `rakia` on PATH:
`[gate] elohim-compute reused: green 4h ago at 5535405d4 (signer this workspace)` (wall-clock
sprint journal). The second half of Verify, the `sophia` selection with sibling worktrees present,
was not re-run in that pass.

Verify: a reuse hit on a device with no `rakia` on PATH; with sibling worktrees present, a `sophia`
change selects `elohim:build-angular`.

- [ ] **C — brit signs `DerivationRecord` bodies and its notes carry them; a brit push keeps the fast-forward check under an explicit lease** (serves sprint-lands-each-closure-once)

State: the brit checkout holds no `DerivationRecord`. The type lives in eprfs-core and in ark's
`brit_index.rs`.

Verify: the design's signed-record vectors (a tampered body and an unknown non-`ext` field are
refused, an unknown epoch is held), and a Claude-family adversarial review before it lands.

- [x] **D — honesty: `claimed` has its own exit code, a mix of passed and skipped reads `partly-measured`, and an integrity-marked rule declared below `ask` fails validation** (serves pin-attestation)

State: all three are in code. `gate-attest.mjs` returns 3 for a claimed pass,
`habits-status.py` reports `partly-measured`, and `eprfs-meta/src/integrity.rs` refuses the
rule (test `integrity_marked_rules_declared_below_ask_fail_validation`). The tests were read, not
re-run, in this pass.

- [x] **E — the tier blob store: a local chunked put, the tolerant reader, and a `serve-blob` mount with scope enforcement** (serves sprint-lands-each-closure-once)

State: landed by the artifact-store sprint, E3 in 3b266e41d and E2 in 1b76c2327 (that sprint's
`RESULT.md`). The one-device failure tests the design names as this track's receipt were not
re-read in this pass.

- [ ] **F1r-producer — on a real push the gate hands ark a projected view of source trees and records `Use` and `Produce` events with quantities** (serves push-gate-pays-once)

State: `gate-task.mjs` writes a `material.json` of closure metadata for `ark task spec --material`.
It waits on A. The sibling half is done: a Task gate asks the sibling channel on a local miss
(c031c1a4). Since 72e84ae8b every eprfs gate also records a crossed `Produce` for the `epr` and
`bin-launch` it built, keyed by a build-input view (`elohim/eprfs/bin-build-view.json`); that is
the release producer, and the gate task itself still keys on closure metadata.

Verify: a `basis: Computed` sibling row with its `Use` event, from a gate that ran as an ark task.

- [ ] **F3-probe — one landing is attempted with the forge unreachable from this device, and stops at the first step that needs the forge** (serves sprint-lands-each-closure-once)

This runs the design's own test (§0) once, before F3 is built, so that the next station is named
by a failure and not by a plan. Whatever step fails is appended below as a station.

Verify: a journal entry carrying the log line of the first forge-dependent step, or a landing
recorded between the two workspaces while the forge was away.

- [ ] **F3 — a landing holds dev-integration only for the compare-and-set window, and the forge is a fast-forward-only mirror (S1b with D3)** (serves sprint-lands-each-closure-once)

State: a landing is `git push origin HEAD:dev` under the temporary exception in the root gospel.
All 21 scenarios of `genesis/a2o/features/devflow/landing-queue.feature` are `@wip`. One step
toward it is on dev: a `self/<name>` push is a hand-over that runs seconds checks and leaves a
record (bd6a23772; 22.9 s measured, landed by the sibling with `independence: sibling`). It still
travels through the forge.

Verify: a landing with the forge unreachable; the partition interleaving of design §8; mirror
catch-up as a fast-forward; hold p90 under 120 s over ten landings.

- [ ] **F4 — carrying is the one offload path: a peer carries a job from a projected view, and `just measure` runs on that path** (serves measure-runs-on-a-peer)

State: not started. `measure.sh --on adam` refuses and names the operator items. It waits on A, E
and F1r-producer, and on a carrier whose key is not exiled.

Verify: a `basis: UnderGrant` record, and `NotCarried` when no carrier is present.

- [ ] **F5 — one release plane for tools: brit, epr, rakia and ark publish as release-manifest channel heads named by derivation, and a changelog is a projection over fulfilled commitments** (serves runtime-upgrade-propagation)

State: brit 0.2.6 was published through the forge's release workflow on 2026-11-17. `epr` now
moves between a person's devices as a native draw (`epr bin follow`, c781db9ab; 5.8 s on
2026-11-17). The release drawn on 2026-11-18 crashed on shem, because the two devices' C compilers
default to different instruction levels; the sprint ruled that C compiles at the Rust target's
baseline (lane E12, not yet landed). A device's first `epr` is ruled to be a forge release asset
checked by a verifier (R5), and is not built: the laptop still compiles from source.

Verify: a changelog reproduced from fulfilled commitments, and a release whose test status and
publish status are reported separately.

- [ ] **F6-standing — the standing record and council roster are redrafted as a Claude-ruled integrity design** (serves build-earns-reach-by-attestation)

State: drafted 2026-11-22 as a narrower design than the one that failed review: a standing read
for counting only (`genesis/docs/superpowers/specs/2026-11-22-standing-read-for-counting-principals-design.md`,
rulings S1 to S8), with how standing is made, changed and recovered left to the general redraft
it names in its §7. Draft status; one adversarial review was run the same day and its findings
are recorded in the design's last section. The contextual-authority design failed review twice
(its §12); the commons-pool design deferred standing to it.

- [ ] **F6 — a second principal: the signer to `agent_cid` binding, the graduation verb, the `derivation` attestation kind and D5's three integrity fixes, in the batched DNA move** (serves build-earns-reach-by-attestation)

State: waits on F6-standing, C and D.

Verify: the one-human adversarial test yields k = 1; then the first graduation act.

F6-standing and F6 were held from 2026-11-19 to 2026-11-22 because nothing in the register
stated reach across principals. The habit `build-earns-reach-by-attestation`
(`genesis/orchestrator/.epr-meta/`, unwired) states it since 2026-11-22 (§10), so both are
claimable. F6 still waits on F6-standing, C and D.

Two stations found while writing this section (2026-11-19):

- [ ] **Flow-root — `epr flow project` mints only from a tree at the device's published head, or says which commit it read** (serves ledgers-fold-once)

State: the projector reads the tree under `--root` and writes the store under the same root.
The device's store belongs to the main checkout, which sprints do not fast-forward: on
2026-11-19 it sat 134 commits behind dev. A projection run there that day minted 97 intents and
29 commitments from that tree. Eight of them,
`plans__2026-11-19-steward-device-arrives-outside-the-cluster-plan#1` to `#8`, came from a plan
withdrawn the same day and name no open work. A minted intent has no withdrawal: the only
outflow from the commitment stock is fulfilment (`epr flow stocks` says so), which stood at
1,500 minted against 57 fulfilled.

Verify: a projection from a checkout whose HEAD is not on `origin/dev` refuses or records the
commit it read; a station removed from its document can be retired by a ruling.

- [ ] **Terminal-env — no session starts with `EPR_BIN` naming a build older than the device's head** (serves workspace-device-arrival)

State: backlog row 17 records one terminal exporting a stale `EPR_BIN`. On 2026-11-19 every
editor terminal on shem opened since 2026-11-10 carried it, pointing at a build of 2026-11-02,
while the editor's terminal host process did not; no shell startup file, repository setting,
workspace file or direnv file sets it. With it set, `epr govern` took 1.1 to 1.6 s where the
head takes about 0.45 s, the cite-seal hook reported unsealed cites the head verifies as sealed,
and commits read `UNMEASURED`.

Verify: `epr device status` names an `EPR_BIN` that resolves to something other than the head,
and a new terminal on shem starts without one.

One station found while reading the open work against the steward's market picture (2026-11-21,
§9):

- [x] **F4-probe — one landing where another seat or another device pays one long gate with no new wire kind, and the lander reuses its verdict** (serves sprint-lands-each-closure-once)

This runs F4's promise once with what already crosses, before the carry is built, so that the
next station is named by a failure and not by a plan, as F3-probe does for F3. It has two rungs.
On the first, the carrier is the lander's own second cargo seat: the push loop starts one long
gate there, runs the others, and collects its verdict at the end. On the second, the carrier is
another device of the same steward: the lander makes its commit reachable with a `self/<name>`
push and says so with a `berth say` note; a device whose own guard lines admit it runs that gate
in a clean checkout at that commit; its green crosses as `gate-verdict@1`. Nobody is assigned:
whether a device takes the gate is its own answer.

State, 2026-11-22: both rungs have run. First rung: landing 13 (`9dbfbac2d`) carried
`elohim-storage` on this device's second seat beside the loop (1,384 s; thirteen gates summing
3,653 s in a gate phase of 2,311 s; guards under their lines). Second rung: landing 15
(`005649870`), where the sibling device ran `just gate eprfs` for the lander's commit (802 s,
green) and the lander's log printed `eprfs reused: green in 802 s on the sibling …
independence: sibling`; that gate cost the lander 32 s. Receipts: the sprint journal and arrival
backlog rows 73 (delta 2) and 76. What the second rung named is the next station below: the ask
waited 7 h 38 min behind a stalled exchange that told neither session.

State before the rungs ran, kept for the record: the push hook runs gates one after another and
asks for a sibling's verdict once, when a gate starts, without waiting
(`genesis/orchestrator/gate-runner.mjs` `decideReuse`). A push that touches a governance path refuses every sibling green. The second rung
travels through the forge, so it does not pass the design's §0 test. The first rung waits on
arrival backlog row 64: the build hook admits one build where the policy offers two seats.

Verify: a landing log that shows two gates overlapping on one device, with ram-guard and io-guard
under their soft lines, and its total against the serial figure; then a landing log that prints
`independence: sibling` for a gate another device ran for that landing's commit. Either way the
journal carries the seconds saved or the first blocker, and the blocker is appended here as a
station.

One station named by the second rung of F4-probe (2026-11-22):

- [ ] **F4-ask — the ask to carry a gate is a record both devices read, raised before the gates, with a bound** (serves sprint-lands-each-closure-once)

The second rung carried its ask as `berth say` notes. A note reaches a session, and only while
the exchange under it moves; when that stopped for more than nine hours, the sender's line said
"note sent" and the waiter's said nothing. Two lines now tell each side
(`genesis/agentic/bin/berth`: a `say` to a silent device, a `--follow` when a device goes silent).
That is a stopgap. The station is the arrival backlog's row 9, step 2: the hand-over is a landing
intent carrying the head, the closure keys and the misses, which a device reads from its own fold
whether or not a session is at its inbox, and which names the time after which its asker stops
waiting and runs the gate itself.

State: not started. Read in the code on 2026-11-22 (a map with its line references is kept in the
sprint's campaign directory, `f4-ask-map.md`):

| What the station needs | What exists | What is missing |
|---|---|---|
| A record raised before the gates | `epr flow intend`'s `raise` writes one, on the device only; the crossing `intent` event is written by `verified`, after the gates | a crossing form of the raise |
| The head, the closure keys, the misses | the intent atom carries tree, base and closure keys; the hook's hand-over step already computes, per selected gate, whether this device holds a verdict for the head | the commit, the ref it is reachable at, and the misses, on the record |
| A device reads it from its own fold | `epr peer intents` | it shows no misses; nothing tells a waiting session that one arrived |
| The verdict comes back | `gate-verdict@1` crosses; the lander asks for a sibling's verdict at each gate's own start | nothing |
| The objects reach the second device | the forge, through a `self/<name>` push | a path that is not the forge (F3's concern, not this station's) |

A first slice that adds one crossing kind and no claim (my reading of what the probe asks, to be
ruled on before it is built):

1. **The hand-over posts the misses.** A push to `self/<name>` already runs only the seconds checks
   and records, per gate, the verdicts this device holds for the head. The same step raises the
   landing intent and writes one crossing event on it, of kind `misses`, in `dev-landing@1`:
   the commit, the ref, and `<project>=<closure key>` for each gate with no reusable verdict. The
   hook stays a thin caller of `epr flow intend`.
2. **A miss settles by fold, never by a clock.** It is open until an admitted device's verdict for
   that project and closure is folded, or the intent is landed, withdrawn, dismissed or superseded.
   No time crosses that a second device must agree with. What a requester expects a gate to cost
   (its own p90) rides along as a display figure, so a device can judge whether it would finish
   in time to matter.
3. **The requester waits for nothing.** A push to dev runs its loop as today, cheapest gate first,
   and asks for a sibling's verdict at each gate's start, as it already does. What a sibling
   finished by then is reused; what it did not, the lander runs. Tests 1 and 2 of §9 hold by
   construction: the ask reserves nothing, and the requester can always run alone.
4. **A device takes a miss by its own answer.** `epr peer intents` lists open misses; a `--follow`
   on the inbox says when one arrives, as it now says when a device goes silent. Whether to run
   the gate is the device's own decision from its own lines (`berth room cargo`). Nobody is
   assigned, and at this slice a session decides, not a daemon.

What this slice does not have, and the next one needs: the `carry` claim. Without it two idle
devices can both run one miss, and the lander cannot skip a gate it has not yet seen a verdict
for. S3a's rule (wait `confirm_wait_s` for a `carry` claim, else run alone) is that slice.

Cost to land the first slice: `epr-cli` and the crossing allow-list change, both on the list of
paths that refuse a sibling's verdict, so that landing reuses none; no crate CI reads as an Edge
input is touched.

Verify: one landing where the hand-over push posts the misses, a second device's log shows it took
one from `epr peer intents` with no note sent, and the lander's log shows the reuse line for it.

Read against an outside pattern (2026-11-22, session 4ce727b3, at the steward's request). The
article "Building always-on agents" (claude.com, 2026-11-13) builds an unattended agent from an
event inbox, timers, memory, permissions and an outbox, and a practitioners' thread under it adds
what broke for them. Read against this station it changes nothing in the slice above and sharpens
what the slice should show. The wider reading is in the arrival backlog (rows 23, 30, 50 and 76,
delta of the same day).

| The pattern says | What this station already holds | What the reading adds |
|---|---|---|
| Every source writes to an inbox that outlives the worker; nothing calls the agent directly | The ask is a record on the intent, read from a device's own fold whether or not a session is at its inbox | Nothing. It is this station's own sentence, and the general form of what rows 23, 50 and 76 each met with a note |
| The inbox dedupes by the source's id, because delivery is at least once | A crossing event has a CID; berth's inbox takes a note from the fold once per CID (`genesis/agentic/bin/berth`, `known`), and the note hook is at least once since row 76's delta | A replayed or re-raised hand-over is the same ask. The `misses` event should be read once per CID by whatever lists it, and the slice's test should include a replay |
| A follow-up is a timer in its own store that fires an event | Step 2: a miss settles by fold and not by a clock, and no time crosses that a second device must agree with. Step 3: the requester waits for nothing | No timer and no store. Where an asker does wait (a question, not a carry), the time it stops waiting is its own: written on the ask so the other side can see it, binding on nobody else. That is the "bound" in this station's title and row 76's last owed line |
| One active worker per inbox | Nobody is assigned; a device takes a miss by its own answer | Nothing here. The pattern's rule is the `carry` claim's, and it stays in the next slice |
| A wake should ask whether the lane can afford to run (the thread) | §9's test 3: capacity is sensed by the carrier, from its own lines | Nothing. It is the same rule in our words |

Two things the pattern has that this plane has already refused, named so that a later reader does
not add them: a scheduler that polls a store and wakes a worker (§9's vocabulary; the landing-plane
design's "No agent wakes another agent" at S1), and a queue that holds what was asked (test 1: a
request reserves nothing).

One missing node, named and not built here. Chain: an ask that waits. Between "the asker's own
stop-waiting time passes" and "the asker does the work alone": the asker's own session hears that
the time has passed. Probe: a session whose only activity is a `--follow` on its inbox prints one
line when an ask it raised passes its time, as it now prints one when a device goes silent.
Current state: nothing reads an ask's time; the bound is the asker's own habit (row 76).

Two stations found while reading the plan against the steward's reach reading (2026-11-22, §10):

- [ ] **F6-k1 — one human with two devices and an agent counts as one voice wherever attestations are counted toward reach** (serves build-earns-reach-by-attestation)

D5 gates graduation on three integrity fixes. Two of them are in elohim-storage and need no DNA
move: release promotion counts distinct agent strings, and reanchor backfill admits attestations.
This station is those two, each with its adversarial test. The third, the DNA issuer floor, stays
in F6 with the batched DNA move.

State: not started. `tally` in `elohim/elohim-storage/src/services/release_attestation.rs` keeps
one voice per agent string, so a peer that attests twice counts once, and one human's two devices
count twice. `is_canonical_content_type` in `reanchor_backfill.rs` admits the `attestation:`
prefix. Counting principals needs a signer-to-principal read, which F6-standing supplies; the
tests can be written first against a fake resolver, the way §9's contracts are proven.

Verify: a storage test in which three agent keys bound to one human yield one qualifying voice,
and one in which the backfill sweep leaves an attestation row unauthored; both run by
`just gate elohim-storage`.

- [ ] **F6-probe — one green gate verdict is offered for graduation with what exists today, and the journal records the first check that refuses it** (serves build-earns-reach-by-attestation)

This runs F6's promise once before the verb is built, as F3-probe and F4-probe do for their
joins, so that the order of F6's parts is set by a refusal and not by this plan. The candidate is
a `gate-verdict@1` this device computed for a landed closure. The probe walks D5's list by hand:
the signer resolves to an `agent_cid`; a standing proof exists for the principal; a second
principal's record for the same (derivation, output) exists; an attestation kind exists to mint.

State: a first read by grep on dev 6934782dc, 2026-11-22; no verdict was offered and no command
of the plane was run, so the station stays open. (1) Signer: `gate-evidence.mjs` `signerIdentity`
reads brit's hex agent id, and `epr-cli/src/peer.rs` `standing` resolves a device's `did:key` to
its steward's handle through the roster; `brit-epr/src` names no `agent_cid`, so the brit signer
is bound to a principal by nothing this read found. This is the first refusal. (2) Standing proof:
the roster row reads "claimed by themselves", which binds keys to a handle and proves no distinct
human. (3) No second principal's record exists. (4) The DNA's 24 attestation kinds include no
derivation or build kind; `attestation:gate-decision` is among them and its subject was not read,
so whether it could be mistaken for a build gate's verdict is open.

Verify: a journal entry naming each of the four checks as held or refused with the command or
file that showed it; the first refusal is appended here as a station or names an existing one.

## 9. Reconciliation 2026-11-21: the market reading

The steward's picture, given on 2026-11-21: the integration pipeline should work like a market on
the REA primitives. A device publishes the steps it needs derived. Devices that sense spare
capacity see a step that fits, admit it, run it and return a signed result. There is no central
controller, queue or executor pool. He bounded the reading twice: it refines what is in flight
and invents nothing, and it steers while the wall-clock harvest stays the priority.

**The reading.** For one steward the picture is ruling D9 and join F4 of
[the native dev plane design](epr:native-dev-plane-one-derivation-earned-reach-design): one
offload path, a `TaskSpec` claimed by a carrier whose own guard lines admit it, and `NotCarried`
with a 0 s wait when none is present. The way there is the hand-over ladder in the
`steward-device-arrival-open-items` backlog, row 9: `self/<name>`, then the hand-over as a landing
intent both devices read, then carriers that write their results back, then offers in place of
branches. Between principals the picture is §5's requests and offers (row 77 of the
`arch-workspace-discipline` backlog), which stay held with F6 until a habit states reach across
principals. Slice S3a of [the landing plane design](epr:closure-keyed-landing-plane-design) is the
same carry at its first rung. Nothing here is a new design.

What exists, by role, as read from dev f1e3540fd:

| Role | Its name here | State |
|---|---|---|
| Request | the landing intent and its misses (`epr flow intend`, `dev-landing@1`) | The `intent` event crosses only after the gates have run (`verified`). It names a tree and lists no misses. The push hook raises none. |
| Claim | `carry`, a `Commitment{Produce, satisfies:[intent], bound}` | Designed in S3a. Not built. |
| Performance | `ark task run` in the carrier's own envelope | Built for the device's own envelope, for the `orchestrator` gate. |
| Settlement | `gate-verdict@1`, later a `DerivationRecord{basis: UnderGrant}` | Crossing. 25 projects are enrolled for reuse. |
| Capacity | the carrier's own guard lines; only a claim or a refusal crosses | Sensed on the device. Not yet one admission answer (below). |

**Five tests.** A change to the gate plane is read against these. Each is already ruled; the last
column is what broke it on 2026-11-21. Row numbers are the arrival backlog's.

| # | Test | Ruled in | Broken by |
|---|---|---|---|
| 1 | A request reserves nothing. Asking costs no seat, pod or lease until a peer admits the work. | berth: a second claimant is refused, never queued; S3a "post the intent's misses" | Sixty Jenkins builds each held a pod request for 21 hours before they could refuse (row 63). |
| 2 | A claim is bounded and expires, and the requester can always run the work alone. | design §1 "Peers add; they never gate"; D9 | A landing with green gates was fenced by a quiet sibling (row 61). |
| 3 | Capacity is sensed by the carrier, and only its effect crosses. A device answers "is there room" once, from its own lines. | D9; `elohim/placement/core/src/offer.rs` | The policy and berth offer two cargo seats and the build hook admits one (row 64). |
| 4 | The unit on offer is a step keyed on exactly what its recipe reads. | D2; habit push-gate-pays-once | Too wide: rows 42, 51 and 59 (a two-Jenkinsfile edit paid a 1,133 s cargo gate). Too narrow: rows 47, 26 and 29, an inherited `EPR_BIN`, the node version. |
| 5 | Settlement is a signed record keyed on that closure. Speed and reach are separate: a sibling's result buys speed and never counts as an independent witness. | D1, D4 | Nothing. A carried result must keep it. |

Jenkins is tests 1 and 2 applied to a participant: it refuses before it reserves, and it builds
what no device carried.

**Vocabulary.** Code, stories and documents use the words the code has: landing intent and its
misses, `carry`, carrier, `NotCarried`, verdict, derivation record. They do not introduce job,
queue, executor, worker pool, scheduler, bid or price. "Market" is the picture and "requests and
offers" is the name. Which device carries is an outcome of who admits, never a policy line.
Admission rests on standing and a grant. Trust prices verification, and nothing in a record is a
price.

**Interfaces.** The steward, the same day: "the right design choices, and the direction of the
implementation plan.. for beautiful APIs, SDKs, Interfaces and composition." Each cut below is
therefore built as one implementor behind a contract. A patch to the push hook is acceptable only
as a thin caller of one.

The landing is one algorithm: for each selected step, reuse a verdict, else offer the step to a
carrier, else run it here; then collect and settle. It depends on the contracts in this table and
never on an implementor.

| Concern | Contract | First implementor | Later implementors | Native home |
|---|---|---|---|---|
| Where a step runs | A carrier: `offer(step)` answers carried, with a handle, or not carried, with a reason and whose line refused; `collect` returns the gate's own verdict; `withdraw` leaves nothing running | `second-seat` in `genesis/orchestrator/gate-carry.mjs` (`GATE_CARRY=off\|shadow\|on`, default shadow): one long single-job cargo gate on this device's second cargo seat | a sibling device over the peer channel; an enrolled carrier such as Jenkins | D9: a `TaskSpec` offer, `ark task run`, placement-core `decide` |
| Whether there is room | One answer per device and resource: admit, or refuse with the axis and the owner of the limit | `berth room cargo --json`: the policy's seats, the build hook's own count of builds, the RAM and I/O guards; it refuses on anything it cannot read as healthy | ark's `BerthOffer` posture with a host-floor refusal | `elohim/placement/core` (`Posture`), `elohim-compute` (`Refusal`, `LimitOwner`) |
| What a landing still needs | The landing intent's misses, raised before the gates run | none yet (row 9, step 2) | | `epr-rea` `Intent`; `epr flow intend` |
| What a gate is keyed on | Its own recipe and what that recipe reads. The boundary is the executor. A gate project's own `inputs` block is the gate's, both classes: there `buildProcess` names the gate's own scripts, and they stay in its selection and key. A step the gate borrows from a pipeline contributes its sources class only, which is its sources minus its own `buildProcess`: a step's `buildProcess` names the files that define the pipeline executor's process (a pipeline file, a CI build script), and a test refuses a gate step that lists anything else there (five entries in two gate-only manifests are parked in it until the next deliberate Edge roll) | the JS gate key, for projects without a native closure | eprfs, whose bin store hashes the whole tree (needs a roll; row 42) | D2: the projected view; the method CID |
| The result | A verdict that names its basis (computed here, carried, reused) and its carrier | `gate-verdict@1` | `DerivationRecord` | D1 |

A contract is proven by a test any implementor must pass, run against at least two implementors,
one of them a fake. The first-implementor column is what exists on 2026-11-21; a row is corrected
here when its implementor lands. The carrier contract's test is `carrierContract` in
`genesis/orchestrator/gate-carry.test.mjs`, run against `second-seat` and an in-memory carrier. A
sibling carrier needs four things from the peer channel to pass it: an offer that answers admit or
refuse with that device's axis and owner, never silence; a step it can transfer (closure key,
commit, recipe); a handle that outlives the requester's process; and a withdraw the peer
acknowledges. Its results carry the basis `under-grant`, and an unreachable peer collects as
`unknown`, so the step runs here.

**Two wordings reconciled.**

- *Cargo seats.* The design's §11 says "Gates still serialize on one device". The policy has
  offered two seats since 2026-09-09, and the build hook admits one because it counts compiler
  processes (row 64). The steward ruled on 2026-11-21: two seats, then measure. The design's
  sentence described a workspace of 31 GiB; ram-guard read 14.6 of 41.0 GB committed on the day of
  the ruling. A device's own second seat is the first carrier: the push loop that can hold one
  gate in flight there is the loop a carry needs.
- *Measure-class work.* S3a says "measure-class is never carried"; D9, which is later, says
  "`just measure` becomes one method on this path". This session's reading, which is not a ruling:
  there is one path, and a carrier's own posture says which class it admits. A dev berth never
  admits measure-class work; a provider that holds the stores does.

**Order.** Harvest first, by seconds per landing. Each line names its test.

1. The eprfs tests run in parallel (rows 53 and 60): 800 to 1,700 s on every landing that selects
   eprfs. No test applies; a carrier would pay the same.
2. One answer for room (test 3, row 64): the hook counts builds, then the push loop holds one gate
   in flight on the second seat. Up to the storage gate's 3,535 s when storage is selected;
   unmeasured.
3. The lane fence (test 2, row 61): 561 s when it bites.
4. The keys narrowed (test 4): row 42, row 51, then `GATE_RECIPE_KEY` on once its shadow lines
   show no disagreement.
5. Soundness before a second device takes a gate (tests 4 and 5): rows 47, 26 and 29, the
   Terminal-env station, the node version, the scope of row 19's governance fence, and Row-20 of
   [the one-baseline plan](epr:one-baseline-plan). These save no seconds. Without them a crossed
   verdict can be wrong.
6. Station F4-probe (§8), then row 9's ladder as far as the probe asks.

Not built under this reading: a request with no named provider on the DHT, `rakia-peer`, any
published capacity vector, any price or reciprocity ledger. The rows named here keep their homes
in the two backlogs and in the sprint journal
(`genesis/docs/superpowers/sprints/2026-11-21-wall-clock/journal.md`). This section orders them
and mints nothing else.

## 10. Reconciliation 2026-11-22: the reach reading

The steward asked whether this plane differs from the ephemeral sandboxes the industry gives its
coding agents, and then stated where the difference lies: "It's the reach ladder that builds the
layers of trust, and the actual attesting network (connected to it's REA/vsm systems) that allows
something to become trusted, sutained in the public/commons, the holochain DNA notary (unique
holons) that affords it the distributed cryptographic integrity and network-based soverigenty over
anything's provenance." (his words, unedited)

**The reading.** Nothing in that sentence is new to
[the native dev plane design](epr:native-dev-plane-one-derivation-earned-reach-design). Each layer
has a ruling:

| The steward's layer | Where it is ruled | State on dev 6934782dc |
|---|---|---|
| The reach ladder builds trust | §0 premise 4; §5 "the gradient emerges": inside a result's earned reach a peer pays a `Use`, outside it a `Produce`, and the reproduction earns the next rung | Reach is derived per read. `self` and `sibling` are in use. `trusted` is empty: k = 1 |
| The attesting network, joined to REA and VSM, makes a thing trusted and sustains it | D4: independence counts principals with standing. D5: graduation is one governed act with a standing proof per counted principal. §9 test 5 here: settlement is a signed record, speed and reach separate | Verdicts cross between one steward's devices. No signer resolves to a principal. The standing design failed review (design §13) |
| The DNA notary gives integrity and network sovereignty over provenance | D5: one notarized attestation per converged (derivation, output), a new `derivation` kind in the batched DNA move. D6: DNA links for releases | No build or derivation kind exists among the DNA's attestation kinds. The issuer floor accepts all. Nothing from the gate plane is notarized |

So the vision was written and was not in flight. §8 held F6-standing and F6 because no habit
stated reach across principals, and the rule that work serving no habit is held kept them
unclaimed. Everything this plan has built since 2026-11-04 is the part a hosted build system also
has. The part it does not have was parked by a missing habit.

**What changes.**

1. A habit states it: `build-earns-reach-by-attestation`, in `genesis/orchestrator/.epr-meta/`,
   `unwired` and inactive. It names no check, because nothing observes it yet. F6-standing and F6
   are claimable under it.
2. Two stations are appended to §8. F6-k1 takes the two of D5's three integrity fixes that need
   no DNA move. F6-probe walks one verdict through D5's list by hand and records the first
   refusal.
3. A sixth test joins §9's five, for any change to the gate plane:

| # | Test | Ruled in | What it sorts |
|---|---|---|---|
| 6 | Would this behave the same on rented sandboxes with a remote cache? If yes, take the known answer and do not design one. If no, it is the part to build natively, and it names the rung of reach it serves. | The steward, 2026-11-21: "this shouldn't be re-inventing the wheel"; design §0 premise 7 | Known answers: what a step is keyed on (test 4), a fresh envelope for each carried step, hermetic inputs. Native: admission by the carrier (tests 1 to 3), the signed record and its lineage (test 5), the grant, graduation |

   Test 4 has the most rows broken against it in §9 and is the one with a known answer. The
   soundness rows of §9's order 5 (an inherited `EPR_BIN`, the node version, rows 47, 26 and 29)
   are what a fresh envelope removes: a device that persists carries state a new sandbox cannot.
   F4's carrier therefore runs each carried step in an envelope that inherits nothing it did not
   declare, which F4's row already begins with `TMPDIR`.

**Order.** §9's order stands and the harvest stays first. Three lines are added beside it. None
takes a cargo seat from the harvest except the storage gate F6-k1 pays once.

- F6-standing was drafted the same day as a read for counting only (its station's State). Every
  other F6 part waits on it. It is Claude-ruled and integrity-class.
- F6-probe runs now, by hand, and is a journal entry.
- F6-k1's tests are written against a fake resolver once F6-standing names the resolver's
  contract, and its storage change lands with the next landing that selects `elohim-storage`.

The first carrier from a second principal is still blocked outside this plan: adam's agent key is
chain-fork-warranted until exile D1 is decided (F4's row in the design).

**What "trustful market" may be claimed.** Until one graduation act has minted an attestation at
`trusted`, the plane is measured at one principal, and a document that describes it says so. A
rung with no measured saving reads unmeasured (design §5).

Not built under this reading: a build attestation riding `attestation:device-health` (D5 refused
it), one DHT attestation per reproduction (D5 refused it), any score, ranking or reputation
number for a device or a signer, any new entry type outside the batched DNA move.
