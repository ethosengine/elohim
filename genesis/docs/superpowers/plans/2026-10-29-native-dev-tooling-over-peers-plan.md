---
title: "Native developer CI/CD over peers — the plan that subsumes GitHub, Nexus, Harbor and Jenkins"
id: native-dev-tooling-over-peers-plan
status: active
class: process-meta
date: 2026-10-29
steward: human:matthew
author: "Claude (fork of the overnight integration-capacity shift), at the operator's direction"
serves: [push-delivers-within-budget, sprint-lands-each-closure-once, push-gate-pays-once, workspace-device-arrival, devices-share-one-berth]
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
| 7 | **Evidence reuse across devices** | A8 sibling verdict (`gate-runner.mjs:463-486`), closure keys equal on both devices (D4); live two-device receipt owed; `GATE_EVIDENCE=reuse` 5,757 s → 56 s locally | First `independence: sibling` row on a real push; eprfs/orchestrator keys hit after the corpus split | design S2a, S2b | push-gate-pays-once | S2a next |
| 8 | **Carried gates** (peer runs the job) | `just measure` rung H on jessica (12–15 s guest work); rakia executor source on the pin, unbuilt; `ark task run` never run here | A gate neither device paid is claimed by the device whose own ram-guard/io-guard lines admit it, run under `ark task run` in its envelope, verdict folded back; requester waits 0 s when no carrier is present | design S3a; measure-runs-on-a-peer rung A | sprint-lands-each-closure-once, measure-runs-on-a-peer | pending · §5 (requests and offers) |
| 9 | **Landing** (GitHub's role) | pre-push claims `dev-integration` before gates and holds it through them (p90 3,321 s); GitHub ref is the CAS | Phase V gates holding nothing; Phase L holds the lease for a 600 s CAS window; intents are atoms; a silent sibling is overridden by rule; GitHub keeps only the final compare-and-swap | design S1b (A9) | sprint-lands-each-closure-once | morning handoff · landing-queue sprint in flight: S1 (`epr flow intend`, `epr peer intents`) and S2 (decision modules) landing 2026-11-01 |
| 10 | **Jenkins's role** | the only dispatcher, builder, deployer and (nominal) attester | Witness and one builder among many: dispatches, builds what no device carried, never judges policy, never reads a lease | design §2; steward-space A10 (deferred until `runtime-death-witnessed` is green) | push-delivers-within-budget | in motion |
| 11 | **Tool distribution** (brit, epr, launchers) | device bin store: `brit` on PATH is `bin-launch` → content-addressed release (head `bafyreiai7…`); `epr device arrive` reinstalls launchers from `current` (R1) | Releases published to and admitted from the dataplane, not GitHub; auto-update exercised on both devices | restart-to-connected R1/R2; backlog "brit handoff" | workspace-device-arrival | distribution native, publication pending |
| 12 | **Device arrival and the daemon** | `just status device` nine lines; `epr peer serve` not yet supervised; a Che restart cost six manual steps (2026-10-25) | `epr device arrive` is one step to latest and connected; the daemon is an ark `processes[]` child, death-witnessed, relay-independent | steward-space A4; restart-to-connected R3/R4 | workspace-device-arrival, devices-share-one-berth | in motion |
| 13 | **Onboarding a second principal** | roster rows travel by git; `epr actor device enroll \| authorize \| bind`; only `human:matthew` is real, every other human a fixture | Fork an lvi workspace → enrol → the device's head folds on the steward space → its verdicts count as an independent lineage at `trusted` | lvi `DevspaceSeed`; steward-space S2 (collective space, `binds-signer`) | workspace-device-arrival | pending |
| 14 | **Dispatch at community reach** | eprfs-peer over iroh inside one steward's grant | claim / dispatch / result over storage's libp2p swarm for closures already attested at `trusted`; `rakia-peer` crate | rakia Stage 2 (Canopy) | — | pending |
| 15 | **Compute as contribution** | berth leases as ephemeral observations; `delegates-compute` mishpat commitment exists for a second principal | A carried gate is a `Commitment{Produce}` with the carrier's envelope as its bound; recognized through the same REA events as content; stewarded, never free-riding in either direction | design §3 minted 5; rakia Stage 3 (shefa) | — | pending · §5 (requests and offers) |
| 16 | **Capacity read natively** | tonight: cost vector on every cycle row, ledger union, berth fold and reap, the stage-wallclock sensor alive | Each device's carrying-capacity vector declared once in its ark RuntimeManifest and read by dispatch; `Envelope::demand` beside `bound` | design S1c, S4 | push-gate-pays-once, conductor-capacity-represented | S1c landing |

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
- **E4 — one byte store per device**: `BlobStoreTier` composed under the tier on a device that runs
  elohim-storage; the directory store elsewhere; a test that the same CID is never held twice on one
  device by the two paths. No new table, entry type or route: the bytes are Ephemeral (C), rebuildable
  by re-running the method; the `DerivationRecord` is Attested-Private (B2) in brit notes and the tier
  manifest; the custody pledge is a local `custody-blob` commitment (B, born `self`) that graduates to
  the existing elohim-DNA `CustodianCommitment` / `BlobToReplicas` links only at release reach (D6).
- **F2 — the sibling fetch**: `just mesh build` outputs captured into the tier as declared method
  outputs; the sibling resolves the `DerivationRecord` by key, fetches the chunk closure over the tier
  (multi-source where a second holder exists), verifies each chunk and the whole against
  `record.output`, and records `Use` (fetch) against the builder's `Produce`. Receipt: fetch versus
  rebuild as `Use` versus `Produce`, the 454 MB case, in each transport mode the tier supports, with a
  mode that cannot deliver reported as a typed gap.
- **Stories first.** The lesson-1 and lesson-11 a2o scenarios are written `@wip` under
  `@concern:artifact-custody` beside the F2 scenarios, so the release-plane gate is specified now.

**Hardening lane (elohim-storage write-set; runs beside, lands independently).**
- H1 holder-identity binding: payload `peer_id` must equal gossipsub's signed `message.source` (not
  `propagation_source`); on iroh the payload carries the holder's signature; custody counts read only
  bound, unexpired claims. Tests: a spoofed-holder snapshot is refused and changes no custody count; a
  replayed old snapshot does not re-base the cursor; relayed snapshots still apply.
- H2 `BlobStore` atomicity: temp + fsync + rename; "already exists" means "exists and verifies";
  verify-on-read for inline blobs; a corrupt object is quarantined and the node keeps serving the
  rest. Tests kept distinct: crash mid-write, `ENOSPC`, cleanup failure.
- H3 fetch budgets: drop-equals-cancel racing; a per-artifact budget with stall and total deadlines;
  a manifest size cap before allocation; wasted bytes metered.

**Acceptance.** I1–I7 and §10 items 1–15 as written, plus: the swap test (`manifest_cid` for
`record.output` and back is refused); the interrupted fetch resumes only missing chunks from the
remaining holder; the trickling holder is abandoned on the total deadline and the chunk re-raced within
budget; the oversized manifest is refused before any chunk moves; the unauthorized-CID refusal; the
stale-replay and wrong-channel refusals; the epoch refusal; a schema that rejects bare hex.

**Out of scope, named.** Reach-restricted or private artifacts (the confidentiality plane; ngit L5);
the read-only OCI gateway (ngit H8, held); gate jobs as requests and offers (row 77); the release-head
custody gate's implementation (F5); Radicle 2.0 on iroh (survey W1).

### 7.4 Rulings the sprint needs from the steward (integrity-class; Claude-ruled)

- **R1 — the BLAKE3 `IrohBlobStore`.** Either it becomes a transport-side cache that never holds a
  CID the device's one store does not, or pure-iroh delivery of artifacts waits until it is retired.
  Recommendation: the former, so the per-mode F2 receipt can run.
- **R2 — the custody gate's shape per reach rung.** The invariant (k non-publisher custodians, d
  failure domains, freshness TTL, no conscription) is fixed; the numbers are policy per rung (`self`:
  the local pledge; `intimate`: a second device of the same person; `community`: k ≥ 3, d ≥ 2 as a
  starting proposal). Lands at F5; the numbers go in the release manifest's `adoptionDiscipline`.
- **R3 — ordering of the hardening lane.** Recommendation: H1 and H2 land during this sprint on their
  own write-set and do not gate Track E; they gate any artifact riding `BlobStore` custody.
- **R4 — the yank / supersede / revoke matrix.** The cells (advertise, serve, retain, purge, per
  operation; revocation distinct from validity) are the channel's standing body's call. Today that
  body has one member, which is a fact about the network, not an authority claim.

### 7.5 Mint pass

Rows minted from the survey on 2026-11-05: [arch-dataplane-borrows](epr:arch-dataplane-borrows-backlog)
rows 23–24 (holder-identity binding with staged Stage-2 floors; fetch budgets and drop-cancel racing);
[arch-native-dev-plane](epr:arch-native-dev-plane-backlog) rows 9–11 (custody-before-head at the release
channel; `BlobStore` atomicity and verify-on-read; one byte store per device with the F2 transfer
tests); [brit-daily-driver-handoff](epr:brit-daily-driver-handoff-2026-10-27) row 6 (output binding and
transactional attestation); [commons-holonic-stewardship](epr:commons-holonic-stewardship-backlog)
row 53 (verify against the standing recorded at publication; a named election with channel-scoped
admission; the revoke matrix as the standing body's call).
