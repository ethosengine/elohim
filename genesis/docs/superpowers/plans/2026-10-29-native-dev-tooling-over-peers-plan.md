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
  - genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md
  - genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md
  - genesis/docs/superpowers/sprints/2026-10-26-restart-to-connected-sprint-plan.md
  - genesis/research/integration-capacity-grounding-2026-10-29.md
  - elohim/rakia/docs/plans/README.md
  - elohim/rakia/docs/plans/stage-2-canopy.md
  - elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md
  - elohim/lvi/docs/specs/2026-10-10-workspace-arrival-as-declared-device.md
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
| 5 | **Artifact custody** (Harbor, Nexus) | Harbor by tag with `imagePullPolicy: Always`; cargo `elohim` registry on Nexus (`elohim-epr`, `epr-rea`, eprfs family 0.3.1; pre-push version-collision precheck) | Harbor and Nexus behind rakia `Registry::has_artifact` by digest; the bundle carrier (≤1 MiB CIDv1 chunks named by a manifest entry) as dataplane custody for images, crates and packages — source of truth per the design §3 minted #3: class B2, identity = the artifact digest, closure tag a secondary index, `node_registry` `CustodianAssignment` the existing entry type for custody once it leaves Harbor | design A7b/S3a; rakia executor spec §4.2 | sprint-lands-each-closure-once | pending |
| 6 | **Attestation** | brit local `recordEvidence` (private notes, never pushed); Jenkins `brit-helper.sh attest` wired but inert | One writer per device at harvest time: brit `ci:<pipeline>` notes from Jenkins's archived graph, `inputs_hash` = closure, `output_cid` = digest; the inert Jenkins arm deleted | design S2d' | push-delivers-within-budget | pending |
| 7 | **Evidence reuse across devices** | A8 sibling verdict (`gate-runner.mjs:463-486`), closure keys equal on both devices (D4); live two-device receipt owed; `GATE_EVIDENCE=reuse` 5,757 s → 56 s locally | First `independence: sibling` row on a real push; eprfs/orchestrator keys hit after the corpus split | design S2a, S2b | push-gate-pays-once | S2a next |
| 8 | **Carried gates** (peer runs the job) | `just measure` rung H on jessica (12–15 s guest work); rakia executor source on the pin, unbuilt; `ark task run` never run here | A gate neither device paid is claimed by the device whose own ram-guard/io-guard lines admit it, run under `ark task run` in its envelope, verdict folded back; requester waits 0 s when no carrier is present | design S3a; measure-runs-on-a-peer rung A | sprint-lands-each-closure-once, measure-runs-on-a-peer | pending |
| 9 | **Landing** (GitHub's role) | pre-push claims `dev-integration` before gates and holds it through them (p90 3,321 s); GitHub ref is the CAS | Phase V gates holding nothing; Phase L holds the lease for a 600 s CAS window; intents are atoms; a silent sibling is overridden by rule; GitHub keeps only the final compare-and-swap | design S1b (A9) | sprint-lands-each-closure-once | morning handoff |
| 10 | **Jenkins's role** | the only dispatcher, builder, deployer and (nominal) attester | Witness and one builder among many: dispatches, builds what no device carried, never judges policy, never reads a lease | design §2; steward-space A10 (deferred until `runtime-death-witnessed` is green) | push-delivers-within-budget | in motion |
| 11 | **Tool distribution** (brit, epr, launchers) | device bin store: `brit` on PATH is `bin-launch` → content-addressed release (head `bafyreiai7…`); `epr device arrive` reinstalls launchers from `current` (R1) | Releases published to and admitted from the dataplane, not GitHub; auto-update exercised on both devices | restart-to-connected R1/R2; backlog "brit handoff" | workspace-device-arrival | distribution native, publication pending |
| 12 | **Device arrival and the daemon** | `just status device` nine lines; `epr peer serve` not yet supervised; a Che restart cost six manual steps (2026-10-25) | `epr device arrive` is one step to latest and connected; the daemon is an ark `processes[]` child, death-witnessed, relay-independent | steward-space A4; restart-to-connected R3/R4 | workspace-device-arrival, devices-share-one-berth | in motion |
| 13 | **Onboarding a second principal** | roster rows travel by git; `epr actor device enroll \| authorize \| bind`; only `human:matthew` is real, every other human a fixture | Fork an lvi workspace → enrol → the device's head folds on the steward space → its verdicts count as an independent lineage at `trusted` | lvi `DevspaceSeed`; steward-space S2 (collective space, `binds-signer`) | workspace-device-arrival | pending |
| 14 | **Dispatch at community reach** | eprfs-peer over iroh inside one steward's grant | claim / dispatch / result over storage's libp2p swarm for closures already attested at `trusted`; `rakia-peer` crate | rakia Stage 2 (Canopy) | — | pending |
| 15 | **Compute as contribution** | berth leases as ephemeral observations; `delegates-compute` mishpat commitment exists for a second principal | A carried gate is a `Commitment{Produce}` with the carrier's envelope as its bound; recognized through the same REA events as content; stewarded, never free-riding in either direction | design §3 minted 5; rakia Stage 3 (shefa) | — | pending |
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
2. Sprint 2: rows 7 (S2a receipt), 4 (S2c DNA), 6 (S2d' attestation on the device), 12 (A4 supervised daemon).
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
