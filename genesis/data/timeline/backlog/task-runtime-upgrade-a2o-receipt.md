---
id: "backlog-task-runtime-upgrade-a2o-receipt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: the rung-5 a2o story + mesh receipt — publish → elect → adopt → attest → promote → converge → revert-by-re-election, with one peer riding an experiment channel throughout"
slug: "task-runtime-upgrade-a2o-receipt"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
priority: "high"
claimedBy: "claude-sonnet-t6-story"
jobs: [elohim-genesis, elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-channel-ceremony-driver"
  - "backlog-task-release-apply-vehicles"
tags: [upgrade-propagation, rung5, a2o, receipt, story-first, delegable]
---

**Claimable by any implementation agent. The STORY half can (and should)
start immediately — story-first is the repo default; the RECEIPT half is
green only when T1-T4 land (T5 enriches it). This is the spec's
graduation-trigger artifact (§10).**

## Why

The receipt is the whole point of the arc: the moment this passes on the
mesh, the CI roll is no longer the delivery path for the coordinator class,
and the spec's constitutional posture has its first lived demonstration —
the protocol stewarding itself, and the diversity that teaches it.

## P2P design-gate decision

Ephemeral (C) throughout — story, steps, and receipt script stage fixtures
and read receipts; nothing new is persisted or notarized. C4: every
convergence assertion reads from EVERY peer, and unreachable ≠ absent.

## Scope

1. **Story** (`genesis/a2o/features/delivery/runtime-upgrade-propagation.feature`,
   `@concern:runtime-upgrade-propagation @requires:household-nodes`, act
   placement per `project_tests_layered_as_acts_of_one_story`): a household
   mesh receives a coordinator release through the ceremony — staged, soaked
   by a canary, promoted on evidence, converged, and REVERTED by re-election
   when the household finds it wanting — while one peer rides a compatible
   experiment channel and is heard, not outvoted. Written in learner/household
   language, NOT ops language. **MUST enter the context-isolated blind-reader
   revision loop required by `genesis/a2o/.epr-meta` before merge.**
2. **Steps + receipt script**
   (`genesis/a2o/scripts/runtime-upgrade-receipt.ts` + step definitions
   composing it): drives T2's ceremony verbs and reads T3/T4's
   `/admin/adoption`, `/version` coordinator hashes, and T2 `status` JSON.
   Receipt = the §10 chain with timings per station; exit 2 names the
   stalled station. Negative controls: an envelope-broken release is REFUSED
   by every peer's verify arm (typed reason observed); an unauthorized
   declare is refused (T2's control).
3. **Close the loop**: cycle-time delta row appended to the arc doc's table
   (`upgrade-propagation-p2p-design-arc.md`) + a one-line DELTA to the habit
   ledger the a2o `@concern` joins (no status flip without fleet evidence —
   flip authority stays with observation).

## Disjointness contract

- MAY create the feature file, steps, receipt script; append the arc-doc
  table row + habit delta; edit this atom.
- MUST NOT edit Rust source, zomes, `hc-mesh.sh`, or sibling task scripts —
  a missing station is reported as a story-graph node (chain / between /
  assertion + probe / current state), never patched around from here.

## DoD + verification

- Story passes the blind-reader loop; `just test mesh
  genesis/a2o/features/delivery/runtime-upgrade-propagation.feature` green
  twice consecutively on a fresh mesh (fresh channel ids each run).
- The receipt transcript shows: staging convergence on 3/3, canary
  adopt+attest, earned promotion, 3/3 apply convergence with conductor PIDs
  unchanged, revert convergence, and the experiment-channel peer diverging
  compatibly throughout.
- Arc-doc cycle-time row + habit delta appended; `habits-project.py --check`
  clean.

## Implementation notes (2026-09-01)

Story half landed:
`genesis/a2o/features/delivery/runtime-upgrade-propagation.feature`
(`@concern:runtime-upgrade-propagation @requires:household-nodes @wip @act:i`,
household mesh = matthew/jessica/james per `household-mesh.ts`). Ten
scenarios: eight map to §10's receipt chain in order (publish → staging
converges on all three peers → canary (james) adopts + attests with
context → earned promotion on james's evidence, never matthew's own
say-so → fleet convergence with conductor PIDs unchanged and jessica shown
no prompt → revert-by-re-election → james's personal `canary-james` channel
diverging compatibly throughout → the observed version matrix reading every
transition back honestly), plus two scenarios giving §4's constitutional
posture its own assertions: jessica's "protection is not a veto" framing,
and a negative control refusing an envelope-breaking release. Design read:
grandma's proof case (spec §4) is staged through jessica rather than a new
fixture persona, to stay inside the canonical household triad and the
3-peer-mesh framing the receipt's own DoD names; "canary" is read as
in-household (james), not a literal neighboring peer, to keep the story at
Act I / `@requires:household-nodes` rather than Act II's `@requires:shem`.

**Revision pass 1 (2026-09-01, from the first context-isolated blind-reader
verdict REVISE):**

- BLOCKER fixed — canary-a had no follower, so the publish→promote chain had
  no mechanism. Resolved the channel model explicitly: canary-a is now the
  household's shared SOAK channel (Background wires all three peers onto
  it), distinct from james's personal `canary-james` experiment channel
  (his alone); james's narrative role is "serves as the canary" on the
  shared channel, not sole-follower.
- Vocabulary expanded to close five gaps the reader named: the
  follow → resolve → adopt lifecycle (each verb now defined, not just
  ADOPTION); the conductor-mediated-resolution vs. adopting-from-a-hint
  distinction (Station 2's Then step now says a peer hint may point a
  runtime at a channel, but the runtime's own conductor still does the
  resolving — gossip isn't refused, only *unverified* adoption is); a
  coordinator/integrity-line sentence; a concrete example of what an
  attestation probes; and one line pinning ELECTION to "the deterministic
  arbiter," not a ballot (title's "election" left unchanged, as directed).
- "governance tally" (undefined jargon) replaced with a plain-language
  Then step naming content/files/recorded-agreements directly.
- jessica's veto scenario reframed from an untestable absence claim to an
  inspectable one (no setting/flag/control exposed) plus a named, non-abstract
  governance surface (the release's own explanation + escalation reach —
  both literally named in spec §4, not invented here).
- Station numbers moved from comments into scenario titles ("Station N —
  …", 8 of them, one per §10 receipt link) since runners strip comments;
  short comments retained alongside as rationale, not as the sole carrier.
- "the ceremony above" (a positional reference) replaced in both Station 7
  and Station 8 with a self-contained Given ("a complete ceremony has
  staged, promoted, and reverted a release on the commons channel").
- Minors: safety-property Then steps now lead with the human-legible claim
  and subordinate PIDs/agent-keys/DNA-lineage as parentheticals (Stations 3,
  5, and the negative control). Doorway Background line kept, not cut — the
  a2o `CLAUDE.md` convention states "Background: always include `Given
  doorway "alpha" at ...`" unconditionally; no step here exercises it yet,
  but future admin-surface reads may proxy through it, and the convention
  itself is unconditional, not usage-gated.

Scenario count is unchanged (ten); none were added or removed, only
retitled/reworded.

**Revision pass 2 (2026-09-01, from the second context-isolated blind-reader
verdict REVISE-but-converging — vocabulary block, station chain, and
jessica's thread independently praised; coordinator flagged this the FINAL
revision round, merging on integrator judgment next):**

- Arc framing moved from header comments into the Feature description's
  opening two sentences ("rung 5 of the household's upgrade-velocity
  ladder... rungs 1-4... already landed: the ground this rung stands on");
  header comments trimmed to only the spec-path and backlog-atom pointers.
- Vocabulary gained three definitions the reader named as missing: the
  COMPATIBILITY ENVELOPE (validation-rule identity + additive-only wire
  changes + a matching declared lineage parent — diverging inside it is
  welcome, breaking it is refused); a one-sentence COORDINATOR/INTEGRITY
  grounding (coordinator = hot-swappable behavioral layer; integrity =
  shared validation identity, moving it needs a heavier ceremony); and an
  ELECTION mechanism line (a deterministic rule every peer applies locally
  to the same declared candidates — no consensus round, no judge).
- Station 2's Then step reworded from a universal negative ("never adopted
  from a hint") to an observable claim: each runtime's own adoption record
  names its own conductor as the resolution path, so a hint is visible as a
  verified pointer, never as the thing adopted.
- Station 3's Then step reworded from an unbounded "nothing changed" claim
  to an observable one: james's runtime's own passport reports the same
  agent identity and cells, human claim first, infrastructure detail
  subordinate.
- Station 7 reworked from "at any point during that ceremony" (unsteppable)
  to three named, sequential checkpoints — after staging, after promotion,
  after the revert — as three When/Then pairs inside the SAME scenario
  block (the acquisition-pins.feature multi-When/Then-pair pattern), so the
  scenario count stays ten and no Scenario Outline expansion was needed.
- Background's soak-channel Given split in two: "follows its shared soak
  channel" / "james is designated the canary on that channel" — was one
  compound line.
- Ceremony agency unified on "matthew runs the ceremony" across Stations
  4/6/7/8 (6 and 7/8 previously had the ceremony itself as grammatical
  subject — "a ceremony has staged…" — inconsistent with 4's "matthew runs
  the promotion ceremony"; Station 8 was fixed too for the same reason even
  though the coordinator's list named only 4/6/7, since it shared the exact
  phrase Station 7 had).
- Left unchanged per explicit instruction: the "But" clause in jessica's
  scenario, the doorway Background line, the Feature title (including the
  word "election"), scenario count (ten), and the station structure/order.

**Pending, explicitly not run here:** the coordinator stated this is the
final revision round and will merge on integrator judgment — no further
blind-reader dispatch from this side. Step-definition/receipt-script wiring
stays blocked on sibling tasks T1-T4 and is the integrator's.

**Blind-reader loop record (integrator, 2026-09-01):** two context-isolated
rounds run. Round 1 REVISE (1 blocker — canary-a unwired — + 6 majors), all
applied. Round 2 (fresh reader) REVISE converging — independently praised the
vocabulary block, station chain, and jessica thread; its named path-to-READY
items all applied (envelope + coordinator + election definitions, arc framing
into prose, observable Then steps, named observation points). Merged on
integrator judgment: no structural findings remain; the story is `@wip` and
CANNOT pass until T1-T4 land, so the final fresh-reader pass is deliberately
attached to the step-wiring half of this task, before `@wip` lifts.

## Local verification handoff — the pre-push adoption ceremony (2026-09-01)

**Purpose:** prove the full election-and-upgrade chain E2E on the local mesh
BEFORE the queued ~18 commits push (local-mesh-first: the fleet confirms,
never discovers). Everything below is staged; nothing is pushed until the
transcript exists.

**State at handoff:**
- Fresh debug `elohim-storage` (T3 controller + T4 vehicles + T5 rail) built
  20:58Z at `/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev/debug/elohim-storage`
  — the slot `hc-mesh.sh` auto-selects when the release slot is absent.
- Conductors healthy post-orphan-remediation (135f272ea; PIDs 1374383-5);
  **storage arms DOWN** — they boot INTO the new binary.
- Harness carries the orphan guard + the `.next` slot consumption (both
  fixture-proven GREEN).
- Drivers ready: `epr-release-package.ts` (T1), `release-ceremony.ts` (T2),
  `release-attestation-probe.ts` (T5). All run from `genesis/a2o`.

**Ceremony stations (transcript each; exit codes on their own lines):**

0. **Preflight** — `just mesh status`: conductors live, no
   `orphaned-data-root`, storage down as expected.
1. **Storage up on the new binary** — `just mesh storage-restart matthew
   jessica james` (preserved `.environ` + exe records; verify each peer's
   recorded exe names the debug slot). Then per peer: `GET /version` shows
   the new build commit; `GET /admin/adoption` EXISTS (404 before this
   binary = wrong slot).
2. **Follow the channel (rung-4, no restart)** — write
   `ELOHIM_RELEASE_CHANNELS = "runtime:coordinators:elohim:receipt-<date>=observe"`
   into each peer's watched runtime-config file + `POST
   /admin/runtime-config/reload`. `/admin/adoption` lists the channel,
   `mode: observe`.
3. **Package** — T1 packager on a repacked coordinator bundle
   (`content_store.wasm`), `adoptionDiscipline` small for the receipt
   (`soakSecs≈30, attestationThreshold: 1`). **PUT the artifact blob to ALL
   THREE peers** (controller blob fetch is local-only by design — recorded
   T3 station; per-peer PUT is the interim).
4. **Publish + staging convergence** — `release-ceremony.ts channel create`
   + `publish` (fresh channel id), then `status` until **3/3 conductors**
   answer the same staged head (this settles T2's open ~2-min gossip
   question — allow up to the ~20-min churn norm, record actual).
5. **Observe verdicts** — `/admin/adoption` on all 3: `verdict: ok` (or the
   precise typed refusal — `threshold_unchecked` is expected until an
   attestation exists; that is honest, not a failure).
6. **Canary apply** — flip james's entry to `=apply` (config reload).
   Next sweep: `appliedRelease` on his row, `/version` coordinator wasm
   hashes flip, **conductor PID unchanged**; after `soakSecs`, his soak
   attestation authors (T5 rail — it deliberately waits out the window).
7. **Promote + fleet convergence** — `release-ceremony.ts promote` (earned,
   on james's evidence); flip matthew+jessica to `=apply`; 3/3 converge,
   PIDs unchanged. Record wall-clock (expect ~2-min class).
8. **Revert-by-re-election** — `release-ceremony.ts revert` to the prior
   release; 3/3 converge BACK (wasm hashes restore); no flag, no re-key,
   no reset.
9. **Attestation cross-peer proof** — `release-attestation-probe.ts`
   (expect qualifying count with builder excluded; `mismatched=0` post-
   56fe4b802).
10. **Optional stretch (binary class)** — package the debug binary as a
    `storage-binary` release on a second channel; james applies → slot +
    sidecar staged, `pendingRestart: true` sticky → `just mesh
    storage-restart james` consumes `.next` (archives `.applied-<ts>`), exe
    record names the slot. If refused `binary_stakes_not_simulacra`: the
    mesh is not DECLARING Simulacra stakes — that refusal is the fail-closed
    design working; declare the stage explicitly or record the refusal as
    the receipt.
11. **Record + push** — append the cycle-time row to the arc doc's table
    (`upgrade-propagation-p2p-design-arc.md`), one-line delta + transcript
    refs here, commit path-limited; THEN the operator pushes the batch (one
    push; rakia submodule already pushed).

**Failure routing:** typed refusals on `/admin/adoption` are the diagnosis —
never work around them (`manifest_undecodable`→publish shape;
`dna_lineage_mismatch`→bundle vs installed; `artifact_unavailable`→blob not
on that peer; `already_current`→idempotence working). Cross-peer election
questions → `version-matrix --observed` + `release-ceremony.ts status`
(unreachable ≠ absent). Harness states (`orphaned-data-root`) refuse loudly
by design — remediate stop/start, never bypass.
