---
title: One derivation, earned reach — the elohim-native development plane
id: native-dev-plane-one-derivation-earned-reach-design
status: Draft
class: process-meta
process_subdomain: agents
serves: [push-delivers-within-budget, push-gate-pays-once, sprint-lands-each-closure-once, workspace-device-arrival]
date: 2026-11-04
context-tier: disclosed
steward: human:matthew
author: "Claude (orchestrator, Opus 5.5), at the operator's direction; design run wf_1f7dc64d-cd4 (Sonnet, Opus, Codex gpt-5.6-sol and gpt-6-astra readers)"
refines: closure-keyed-landing-plane-design
tags: [derivation, reach, compute-ladder, brit, rakia, eprfs, ark, local-first, gradient, honest-evolution, rea, mishpat]
graduation-trigger: >-
  On two of one steward's workspaces, with GitHub and Jenkins unreachable: a gate closure computed on
  one is reused by the other as a DerivationRecord read through its own TrustPolicy, and an expensive
  artifact (the elohim-storage mesh binary) is fetched from the other's eprfs tier instead of rebuilt.
  Both draws are recorded as epr-rea Use events against the producing Produce event, so the saving is
  read as the difference between recorded events. A landing moves the steward-space dev head while the
  forge mirror is down, and the mirror catches up afterwards without overwriting. Receipts are attached
  to push-gate-pays-once and push-delivers-within-budget.
cites:
  - "native-dev-tooling-over-peers-plan | the 16-row concern table this design re-rules (rows 1-3, 5-11, 13-16) and whose row 5 store it sets requirements for | sha256:b4bf54d75dbf84b8 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
  - "closure-keyed-landing-plane-design | refined: closure keys become DerivationRecord keys; S1b carries D3's landing CAS; S2b stays with that plan | sha256:9156f36238a93744 | path: genesis/docs/superpowers/specs/2026-10-29-closure-keyed-landing-plane-design.md"
  - "steward-space-landing-plane-design | lease fence and expected-prior CAS that D3 lands on; its 'Rakia serves' wording (§5.1) is superseded | sha256:53718b5898db3b15 | path: genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md"
  - "workspaces-cooperate-as-devices-design | the governed peer channel between two workspaces that R1 and the carry ride | sha256:28296e002122f501 | path: genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md"
  - "placement-contract-interfaces-design | §C derivation types and slice 4 (walk_spine, project_view, derive_or_reuse) that track A lands | sha256:de0208f9b6e426db | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "household-follows-latest-design | adoption by declared heads that earned reach (§5) gates for tool releases | sha256:f92e9a35d7410afc | path: genesis/docs/superpowers/specs/2026-10-23-household-follows-latest-design.md"
---

# One derivation, earned reach — the elohim-native development plane

This is a design pass over how developers, CI and CD work natively on Elohim. It starts at the
primitives (epr, eprfs, `.epr-meta`) and meets the brit and rakia developer epics. It settles three
things before work returns to the distributed rakia artifact store (plan row 5):

- the shape of the elohim-native developer experience;
- what makes its evolution honest;
- what the compute ladder looks like over the liquidity and the diversity of our peer runtime.

The pass used six layer readers, three extra readers (sparse checkout and caches, social reach, REA
and mishpat), three lens readings (local-first, p2p-design-gate, gradient-reading) and one synthesis.
Their working files are private (`genesis/local-dev/native-dev-design-pass/`, gitignored). This spec
restates every load-bearing claim with its source path, so it stands on its own.

## Terms

**Components** (all in this monorepo):

- **epr** (`elohim/epr`): the Rust library that defines the protocol's content-addressed atom. It
  covers the dag-cbor CIDv1 codec, Ed25519 signatures, the envelope and the reach enum.
- **eprfs** (`elohim/eprfs`): the content-addressed tree and filesystem built on epr atoms, with views
  and the derivation plane. It includes **epr-cli** (the `epr` command every participant uses) and
  **eprfs-peer** (the iroh channel between a steward's devices).
- **`.epr-meta`**: a directory-local governance package. Its rules and habit atoms gate edits in that
  directory and its children.
- **brit** (`elohim/brit`): a fork of gitoxide, the Rust git implementation. It is a CLI and a set of
  crates that hold the repository, bridge git trees to eprfs trees, and host the build-planning verbs.
- **rakia** (`elohim/rakia`, a git submodule): the build-manifest schema plus planning libraries
  (affected, plan, baselines), linked into brit.
- **ark** (`elohim/ark`): the device's execution envelope and supervisor. `ark task run` runs a job
  inside the device's own resource limits.
- **placement** (`elohim/placement`): pure functions that match a job to the devices that can carry it.
- **lvi** (`elohim/lvi`): the design for a developer workspace that joins the network as a device.
  There is no code yet.
- **epr-rea**: the ValueFlows / REA (resource, event, agent) vocabulary of intents, commitments and
  economic events.
- **mishpat**: the protocol's justice and commitment layer, both a DNA and native records. Commitments
  are made, fulfilled and ruled on here.

**Reach** is the protocol's disclosure scale, ordered from narrowest to widest: `private`, `self`,
`intimate`, `trusted`, `familiar`, `community`, `public`, `commons` (`elohim/epr/src/reach.rs:18`,
`elohim/sdk/schemas/v1/enums/reach.schema.json:5`). Here it is also the scale a build's trust climbs.

**Standing** is a principal's recorded authority in a space. **It has no settled design yet** (§13).
Rulings D4 and D5 depend on it, and that is why join F6 waits on its redraft.

**Other references:**

- "Plan row N" means row N of the concern table in the native-dev-tooling plan (first cite).
- "I1–I7" are that plan's §5 store invariants.
- "S1b", "Phase V" and "Phase L" come from the closure-keyed landing plan. Under S1b's split pre-push,
  Phase V runs gates and holds no lease, and Phase L holds `dev-integration` only for the
  compare-and-swap (CAS).
- "A3", "A7b" and "A11" are slices of the steward-space spec: the lease fence, a sibling's git objects
  absent from the local object store, and the tolerant wire reader.
- "RS 4+3" is Reed-Solomon erasure coding with 4 data and 3 parity shards.
- An **alone-path receipt** is evidence that a flow works with the external role removed: the
  section 0 test, run and recorded.
- jessica and adam are household fixture humans whose devices run on the steward's hardware.

**How to read the normative words.** This is a Draft design. "Never", "only" and "must" in a ruling
state a requirement the design places on code that is not yet written. Each one becomes true when its
track's test enforces it, and §11 names that test. A sentence describing *current* behaviour cites
`path:line` instead. Where current code violates a rule (for example `gate-attest.mjs:108-110` versus
§7 rule 2), the gap is named beside the rule.

## 0. Premises (operator, 2026-11-04)

1. **Native means the queue lives between peers.** Two workspaces already talk over governed peer
   channels. The integration queue and the build queue are features of those channels, owned by no
   central service.
2. **Jenkins is a participant.** It is an enrolled peer that carries work and witnesses under a grant.
   External runners are mirrors or peers that support the network, and none of them is a bottleneck.
3. **Peers agree on four things:** artifact stores and caches, sparse checkouts, peer-native runtimes
   that run and compile builds, and how workloads are distributed. *Agree* means a protocol between
   peers, never a scheduler's decree.
4. **Social reach is where builds graduate.** As independent peers reproduce and attest a build, the
   build earns wider reach through the same reach primitives content uses. Adoption is gated by that
   earned reach.
5. **The trust-as-compute gradient emerges by nature** once builds and releases carry reach. It is not
   a separate scheduler or score.
6. **Spend is revealed by the epr-rea primitives.** What is delivered (release notes, changelogs,
   trailers, stewards, attestations, valueflow contributions) is derived from mishpat over the EPRs,
   never hand-authored.
7. **Priority goes to cycle time, aimed at the prize.** Wall-clock removed from push → landed breaks
   ties. A simple interim step on the trajectory beats an optimization that would have to be torn out.

The test for every move: **if every external runner and the forge disappear, do two workspaces still
integrate and build between themselves? When one rejoins, does it only widen capacity, reach or
independence?**

## 1. Thesis

The elohim-native developer experience is **one participant verb surface (`epr`) over one derivation
plane, on a device that is complete alone.**

- **The unit of CI is the derivation.** A gate verdict, a build output, a carried job and a cache hit
  are all one `DerivationRecord`, keyed by (method CID, input root), with a CID for its output.
- **Peers add; they never gate.** A sibling, carrier, Jenkins or GitHub runner only adds a skip,
  capacity or a mirror. A missing peer means "run here, and say why".
- **Reach is the build's only trust scale.** A record is born `self`. It widens only through a governed
  graduation act over reproductions by independent *principals with standing*.
- **Spend and deliverables are reads.** Carrying is an epr-rea `Produce` or `Work` event. Drawing is
  `Use`. The cost a trusted rung saves is the difference between those events. Changelogs, trailers
  and contributions are projections over fulfilled commitments.

Liquidity (who is online and admits work) buys **speed**. Diversity (independent principals with
standing) buys **reach**. The two ascend separately, and conflating them is the error this design
exists to prevent.

## 1a. What this enables: the liquid repository

> "imagine a liquid repository.. where while you're walking through the tree of code.. every
> epr-meta root module COULD come with a changing social-context as to who cares for that
> component... a button to launch a lvi workspace, where the repo just reveals and knows exactly what
> the compute requirements are to click the lvi button and just start deving on that space at your
> own private reach.. where you can be invited almost anywhere in source code to participate in the
> evolution of code. Stuff doesn't collapse to a repository address anymore." — operator, 2026-11-04

This is the experience the rulings below are for. None of it needs a new primitive. Each part is a
read over something this design already makes real:

| What the person sees | What it is | Where it comes from |
|---|---|---|
| A component in the tree | An `.epr-meta` root: a governed subtree with its own identity, its CID or `id:`, not a repository URL | `.epr-meta` already scopes rules, habits and `covers:` |
| Who cares for it, changing over time | The stewards and contributors of record: a projection over the mishpat commitments fulfilled on that subtree's EPRs | §6. Derived, never a hand-kept OWNERS file. Its authority waits on standing (§13) |
| "What it takes to work here" | The union of the component's method descriptors (pinned toolchain, env) and their envelope demand, plus the projected view: exactly the files those methods read | D2 and §4. The view is the sparse checkout. Demand is plan row 16's `Envelope::demand` |
| The lvi button | Materialize that projected view and toolchain on a device whose own guard lines admit the demand, at `self` reach | lvi's `DevspaceSeed`, with `ark` as the envelope and track E as the source of bytes. No lvi code exists yet |
| Working at private reach | Everything the person derives is born `self`. It widens only by their act and the graduation of §5 | D4, D5 |
| An invitation anywhere in the source | A reach grant scoped to one subtree, offered by its stewards | Reach plus standing, scoped by `.epr-meta` root |
| Contributions flowing back | A landing intent on that subtree, its derivations and REA events, and its earned reach | §§6, 8 |

**One design consequence for track A.** The natural owner of "what this component needs" is the
`.epr-meta` root, not only the build-manifest step. A component's view is the union of its methods'
views. `build-manifest.json` `gate.projects` already sits per directory, so the two line up. Track A
should key view declarations so that a component's view can be asked for by its `.epr-meta` root, not
only by gate step. That is the query behind the button.

**What stands between here and the button:** the projected view (track A), bytes served by peers
(track E), declared envelope demand (plan row 16), standing (§13), and lvi's seed as code.

## 2. The one unit: `DerivationRecord`

The repo already holds the native record. Nobody needs to invent one.

- **`elohim/eprfs/eprfs-core/src/derive.rs`.** The pieces:
  - `DerivationKey { method, input }`, where `input` is `KeyInput::Projected(ProjectedRoot)` or
    `KeyInput::Raw(BlobCid)` (`:87-90`).
  - `DerivationBasis` covers computed, carried under a grant, and recomputed, plus an `Unknown` that no
    trust policy accepts (`:119`).
  - Typed misses arrive as `DerivationUnanswered`.
  - The `DerivationIndex` port records side by side, never overwriting.
  - `TrustPolicy` is the trait by which a reader judges records against its own roster. Not accepting
    `Unknown` is a contract of that trait, which no implementation enforces yet.
  - `to_attestation` (`:178`) constructs a notarized *draft* from one record and a caller-supplied
    `AdoptedHead`. It authorizes nothing: it takes no policy, standing proof, record set or target
    reach (D5).
  - The module's rule: "the record is evidence, never authority … an accepted record whose output
    bytes cannot be fetched is a miss" (`:5-7`).
- **`elohim/ark/cli/src/brit_index.rs`** already reads brit build attestations as `DerivationRecord`s.

**Ruling D1 [integrity] — `DerivationRecord` is the single record.**

- brit's signer signs the record's canonical bytes. brit's `BuildAttestationContentNode` already
  signs its own canonical bytes, which bind the step and the closure hash but no commit
  (`elohim/brit/brit-build-ref/src/build_cmd.rs:88-103`, `elohim/brit/brit-epr/src/elohim/attestation/build.rs:5-29`).
  The node fails in other ways. It is `deny_unknown_fields`, so a newer writer's record is unreadable
  to an older reader (D7). It keeps one note per step and commit, which is an overwrite shape. It has
  no `basis` and no typed miss. And it adds a third key and address format beside `DerivationKey` and
  `gate-verdict@1`.
- brit notes become one *carrier* of records (the git transport). They stop being an authority.
- **Old evidence is converted, never re-signed.** ark's `brit_index.rs` builds a new key and
  `deterministic` value from local context and copies the old brit signature onto them
  (`elohim/ark/cli/src/brit_index.rs:428`). That signature authenticates the original note, not the
  rebuilt bytes. A converted record therefore carries **no signature**. Instead it holds a witness
  pointer to the original note's CID, which a reader may verify over the original bytes. It is admitted
  at most at `self` on the original signer's device. Track C owns this adapter and a test that a
  substituted method or input never verifies.
- `gate-verdict@1` carries the record's bytes.
- The inert Jenkins arm (`genesis/orchestrator/scripts/brit-helper.sh build-ref`) is deleted, not adapted.
- An `Epr{kind: Attestation}` envelope is applied only at graduation (§5). The local path gets no
  extra signing ceremony.

**Ruling D2 — the input root is a view projection, and landing `project_view` is foundational.**

- The key is `KeyInput::Projected`: an eprfs `TreeView` built from the manifest's `inputs.sources`, the
  audited `// gate-reads:` paths and gitlinks. That one object is both the cache key and **the sparse
  checkout a carrier materializes**, so peers agree on sparse checkouts by agreeing on a CID.
- `project_view` has not landed, and `ProjectedRoot` has no public constructor until it does
  (`derive.rs:9-12`). The missing work is already specified as slice 4 of
  placement-contract-interfaces §C: `walk_spine`, `SpineWalk`, `project_view`, `derive_or_reuse`, the
  `P` vectors in `tree-vectors.json` and `eprfs-core/tests/derive.rs`
  (`genesis/docs/superpowers/plans/task-placement-c4-report.md:155-162`).
- ark works around the gap with an identity projection: it decodes a CID string into a `ProjectedRoot`
  (`elohim/ark/cli/src/material.rs:101-113`). That workaround retires when slice 4 lands.
- **No interim key.** Slice 4 is track A of §11. It runs in parallel with the tracks that don't need
  it, and the record (D1) is keyed on a projected root from its first signed byte, so nothing has to be
  re-keyed later. Today's `closureMaterial` v3 (`genesis/orchestrator/gate-evidence.mjs:297-324`)
  becomes the producer of the view *declaration*. Its sha256 survives only as brit's dedup index.
- Toolchain and env fold into the method descriptor CID. A method change is therefore a named
  `NotRecorded` miss, not a silent cold one.
- rakia spec §3.3's Merkle `inputsHash` is not implemented, and would be a fourth key. Upstream output
  CIDs enter a downstream method as `KeyInput`, which gives the same propagation with one key.

## 3. Who owns what

| Layer | Owns | Does not own |
|---|---|---|
| **epr** (`elohim/epr`) | Atom codec (dag-cbor CIDv1, Ed25519), envelope, `Reach`, measures and unknown reasons | Any dev-specific shape |
| **epr-rea** | Intent, Commitment, FlowEvent; `AgentRef` as the canonical identity transport ids resolve to (`model.rs:22-25`) | Meters. Spend is events, not a ledger |
| **eprfs-core** | Trees, views, reach, `AskPolicy`, **the derivation plane** | Git semantics |
| **eprfs-peer** | The iroh device channel (`/elohim/eprfs-tier/1.0.0`): intents, verdicts and leases between admitted devices; from row 5, the tier serves blobs | A second protocol for blobs or jobs |
| **`.epr-meta` / eprfs-meta** | Directory-local governance and habit atoms | — |
| **epr-cli** | The participant verb surface, standing, covenant, acceptance | Execution |
| **brit** | The repository, git ↔ eprfs bridge, the build-planning CLI host (`brit build graph\|affected\|plan\|fingerprint\|baseline`), signed notes as a record carrier, push with expected prior | Governance, intent, review authority, execution |
| **rakia** | Manifest and plan schemas, the constellation, change detection and baselines (as libraries brit hosts), and **custody naming**: a `DerivationIndex` adapter over the dataplane | A store, a server, an executor. "rakia names, the tier serves" |
| **ark + placement** | **The only executor** (`ark task run`, envelope, Passport); pure offer, eligibility, rank and decide | Trust judgments |
| **elohim-storage + DNAs** | Community-reach blobs (RS 4+3), release channel heads, notarized attestations, custody links for release objects | Cache bytes at self or intimate reach |
| **lvi** | The "fork a workspace and join" onboarding story (no crates yet) | — |

**Ruling D8 — `ark task run` is the only executor.**

- Each manifest `run` kind compiles to a method descriptor. A Jenkinsfile stage becomes
  `ark task run <method>`.
- rakia Stage 1's executor sprint retires.
- `rakia-executor` is the measure peer-stage worker, not a build executor. It folds into the carry
  (D9).

**Duplicates to retire, each by replacement with no legacy arm.** Owners and dated `retire-when` lines
go in the backlog:

1. Three attestation shapes for one job → `DerivationRecord` (D1).
2. Three closure keys → one `DerivationKey` (D2).
3. Two offload paths: `just measure` rides the DHT `feedback_signal` envelope, beside the eprfs-peer
   carry → the carry (D9).
4. Two lease ledgers (berth Python, eprfs-peer lease fold), and two berths (`ark::Berth`,
   `genesis/agentic/bin/berth`) → the lease fold and `ark::Berth`.
5. Two `.epr-meta` evaluators and two habit implementations → Rust, once parity holds.
6. Two release planes: bin_store plus GitHub prerelease, and `release-manifest` channel heads →
   channel heads (§7).
7. Two custody types (`node_registry::CustodianAssignment`, `content_store::CustodianCommitment`) →
   ruling D6 (§10, item 10).
8. Four ladder vocabularies (rung H/A, S0–S4, A-ids, "spaces") → the R0–R4 rungs of §4.
9. Steward-space §5.1's "Rakia serves a session the root manifest"
   (`2026-10-27-steward-space-landing-plane-design.md:578`) is superseded: the tier serves, and rakia
   names.

## 4. The compute ladder

| Rung | Admits | Makes cheap | Still pays | State |
|---|---|---|---|---|
| **R0 own device** | Enrolled device key; its own guard lines (ram-guard, io-guard, `ark task run`) | Re-deriving an unchanged closure (reuse took one gate from 5,757 s to 56 s) | The full closure on any change; every mechanical failure | Wired |
| **R1 sibling workspace, same steward** | A roster endpoint of the same lineage, an admitted iroh NodeId, a fresh head | Repeated gates (sibling verdicts); later, carried work and fetched artifacts | Admission, liveness, receiver-side checks, source materialization. **Adds zero independence** | Channel and reuse code wired (`gate-runner.mjs:463-486`); the live two-device receipt is owed; carry and blobs unbuilt |
| **R2 enrolled carriers of the same principal** | Enrolled device plus a `delegates-compute` grant. Jenkins and external runners enter **here**, as enrolled devices | Requester CPU and RAM; work placed near the bytes; Jenkins builds what no device carried | Consent, isolation, interruption. Their records stay `self` | One carrier works: a `just measure` stage on jessica's device. Jenkins is still the dispatcher (plan row 10) |
| **R3 steward space with a second principal** | A second human with an enrolled device, a cross-signed signer → `agent_cid` binding, standing in the space | Reuse under independent standing: the first `trusted` reach | First-contact ceremony; recomputation or confirmation by a different lineage | Pending |
| **R4 community and commons** | A collective as commitment party; reach gates on every serve path | Wide amortization; resilient custody | Reach, revocation, diverse membership, accountable carrying | Pending, doc-only |

**Jenkins and external runners.** They are R2 carriers and witnesses. They build what no device
carried and fold their records back like any other carrier. They never dispatch, judge policy or hold
a lease. A GitHub runner is a mirror that reproduces, and its record lands at the reach its principal
has earned. Its absence costs speed only.

**How peers agree on the four shared things:**

- **Runtimes.** Two peers agree when they hold the same method CID: recipe, pinned toolchain and env
  are folded into it. An `ark` envelope runs it. A peer whose toolchain differs computes a different
  key, so nothing is ever falsely shared.
- **Sparse checkouts.** The input root (D2) names the exact path set. A carrier materializes that set
  from its own objects plus the sibling's (A7b), not the monorepo.
- **Caches.** A hit is admissible on another peer only as a `DerivationRecord` the reader's
  `TrustPolicy` accepts **and** whose output bytes it can fetch and verify by CID. Today's pool caches
  (cargo target pool, sccache, pnpm store) stay device-private, below artifact grain. No wrapper sits on
  the compile path (store invariant I5).
- **Workloads (D9).** One offload path. A `TaskSpec` (method, input root, view, envelope) is offered
  over eprfs-peer and claimed by a carrier whose own guard lines admit it. Placement-core `decide`
  matches over the devices' postures. The result is a `DerivationRecord{basis: UnderGrant}`. When no
  carrier is present, the requester runs it and waits 0 s; the reason is printed as `NotCarried`.
  `just measure` becomes one method on this path, and the `feedback_signal` reuse is retired.
  Requests-and-offers matching between principals arrives with R3.

**Liquidity is measured, never assumed.** Today placement sees `Candidate.live`, a capacity vector and
a freshness bound supplied by the charter (`elohim/placement/core/src/decide.rs:60`; the steward
channel's policy sets 90 s in `genesis/agentic/pool-policy.json`). Three facts get added as folded measures, not as a new register: refusal rate
by guard axis, lease-hold p90, and availability history. These drive admission to carry. Nothing about
liquidity feeds reach.

## 5. Reach is the graduation path

**Ruling D4 [integrity] — independence counts principals with standing, never keys or devices.**

- brit's hex signer, the iroh NodeId and `computed_by` must each resolve to one `agent_cid` through a
  cross-signed binding before a signature counts.
- Device, hardware, toolchain lineage and infrastructure are declared dimensions. They explain a
  `Mismatch` and inform liquidity. They are never counted.
- Today k = 1, and `trusted` is honestly empty. Sibling, carrier and Jenkins records are labelled
  `self` or `sibling` and never counted.
- Plan row 13's "the device's verdicts count as an independent lineage" is struck.
- Why not placement's `FailureDomain`: with no household it falls back to `agent:<key>`
  (`elohim/placement/core/src/offer.rs:210-224`), so one human's devices and agents would read as k = 3.

**Ruling D5 [integrity] — graduation is a separate governed act.**

- Graduation is its own **enforced verb**, not a call to `to_attestation`. It takes the cited policy,
  the set of accepted records, one standing proof per counted principal, the acting steward and the
  target reach. Only when all of them check does it mint **one** notarized attestation per converged
  (derivation, output CID) at that reach. `to_attestation` stays a draft constructor the verb uses.
- Cross-signing proves that keys are bound to one controller. It does not prove that two controllers
  are two humans. Today's enrollment binds devices to a handle and chain root and supplies no principal
  resolver (`elohim/eprfs/epr-cli/src/actor.rs:2358`). Distinct principals come from standing (§13)
  and from nothing else.
- **Graduation is gated on three integrity fixes** being enforced, not merely on the new kind:
  - DNA issuer authorization, which today accepts all
    (`content_store_integrity/src/attestation_validator.rs:67`);
  - release promotion counting principals rather than distinct agent strings
    (`elohim-storage/src/services/release_attestation.rs:859-861`);
  - reanchor backfill no longer admitting attestations
    (`elohim-storage/src/services/reanchor_backfill.rs:51`).
- Each fix ships with an adversarial test: one human with several devices and agents yields k = 1.
- Reach on the record itself only ever narrows, and builders cannot promote their own work.
- The attestation kind is a new `derivation` provenance kind, minted in the next batched
  constitutional DNA move. The DNA's 24 attestation kinds include no build or derivation kind.
  Riding `attestation:device-health` (as releases do today) is refused: its subject kinds are not
  enforced, which makes it a laundering surface. Until the move, nothing graduates past `intimate`.
  That matches reality, since no second principal exists yet.
- One DHT attestation per *reproduction* is refused: lens-p2p puts it at 6–26 thousand heads a year.

**The gradient emerges.** No scheduler or score is involved:

- A peer inside a build's earned reach reuses its verdict or fetches its bytes and follows its
  release. That costs a `Use` event.
- A peer outside that reach reproduces the build or witnesses it. That costs a `Produce` event, and
  that reproduction is what earns the next rung.
- The design obligation is honesty. Every rung's saving shows as a cost that was not paid, measured
  from recorded events. A rung with no measured saving reads **unmeasured**, never "working".

## 6. Spend and deliverables are reads

**Spend.**

- Every draw (a reused verdict, a fetched artifact) is a `Use` FlowEvent with quantity (cpu-s, wall-s,
  bytes).
- Every build or carry is a `Produce`, fulfilling the claim's Commitment.
- Carrying for another device is `Work` under the `delegates-compute` grant.
- Same-principal carrying uses provider = receiver = `human:matthew`. The device and envelope are
  provenance, never a fake party.
- Draw ledgers stay on the drawer's device.

The gaps are concrete, and join F5 owns all three. `ReaVerb` lacks `work` and `deliver-service`
(`elohim/epr/src/witness.rs:315`). Gate cost today rides as `--env` slots on a `Cite` note. The
`.claude/data/*.jsonl` and berth ledgers are ad-hoc meters that retire into events.

**Deliverables.** Release notes, changelogs, commit trailers, steward-of-record lines, habit DELTAs and
valueflow contributions are **projections** over the mishpat commitments a change fulfilled and the
EPRs it touched. That means the derivation records, their `Produce` and `Use` events, the covenant in
force and the intent atom. The minimal read takes:

- **in:** the intent CID and the range of landed heads;
- **out:** fulfilled commitments with their touched CIDs, records and spend;
- **signer:** the projection is unsigned and recomputable by anyone. Only the release act that cites
  it is signed.

The existing contribution projection under `epr flow memory` is its seed. Any hand-written CHANGELOG
entry or typed trailer is a parallel record. Each gets an owner and a `retire-when`, and is removed
once its projection reproduces it. This is the same rule as "the harness witnesses; agents never run
their own attribution".

## 7. The law of honest evolution

**Ruling D7 [integrity] — every atom that crosses the wire is additive-open, with exact-byte
signatures.**

- **Framing.** A signed atom is `{body: bytes, sig}`. The signature covers the body bytes exactly as
  received and is never embedded in them. The body is never re-encoded before verifying.
- **Epochs carry meaning; extensions carry none.** Within one epoch, the only fields a writer may add
  live under an `ext` namespace. By declaration they cannot restrict, scope or authorize anything. Any
  field with meaning, including a *restriction*, moves the epoch.
- **A reader's three cases:**
  - known epoch, known fields: decide;
  - known epoch, unknown field outside `ext`: refuse (a producer-lint violation);
  - unknown epoch: verify, retain the bytes, name the record once, and treat it as absent for every
    decision.

  This closes the case where an older reader drops a newer restriction and over-admits.
- **Retention.** Decoding keeps unknown `ext` content as opaque bytes instead of discarding it.
  `DerivationRecord` decoding discards today, and the nested `DerivationKey` rejects unknown fields
  (`derive.rs:103`, `:148`). Track A fixes both.
- Producers lint strictly. This is the `release-manifest` pattern ("OPEN BY DESIGN").
- An ALPN advertises supported epochs and peers choose the highest common one. There is no further
  negotiation protocol.
- `EprKind` gains an opaque `Unknown` arm on the decode path only.
- brit verification moves from "strict decode" to the three cases above. That keeps the property
  brit's strictness protects today (no field smuggling) while letting mixed versions talk.
- A11's tolerant reader ships before any new byte path on the tier.

The remaining rules each hold through an existing mechanism, and each gap below is a backlog item:

1. **Claims bind to evidence.** Habits require `checks:` and `retire-when`. `habits-status.py` already
   reads NOT MEASURED when no scenario passed. *Gap:* a concern with some checks passed and others
   skipped still reads green. It must read `partly-measured`.
2. **Unmeasured is a type, never a zero.** *Gap:* `gate-attest.mjs` returns `outcome: 'pass',
   tier: 'claimed'` on an unreachable read (`:108-110`), which every exit-code-only consumer reads as
   green. `claimed` must exit with its own code. Attested projects have no local recipe
   (`gate-runner.mjs:309`), so the honest consequence is a ladder:
   1. a `DerivationRecord` for the pinned commit's check from this device or a sibling admits it;
   2. failing that, `claimed` is admitted **for landing only**, as a recorded debt shown in the watch
      view and settled by the next reachable read, which reopens the landing if it reads red;
   3. `claimed` is never admitted for release adoption or reach graduation.

   A forge outage then costs neither landing nor honesty.

   **Debt is inherited through closures.** Unsettled debt is keyed to the pinned commit (the gitlink
   in the projected view). Every derivation whose input root includes that gitlink inherits it, and so
   does every release closure built on such a derivation. Release adoption and graduation check the
   whole closure and refuse while any inherited debt is unsettled. If the settling read comes back red,
   every dependent derivation gets a contest record, a `Recomputed{matched: false}` against the pin.
   Adoption of every release whose closure includes the pin then halts and reverts the way an upgrade
   propagates (rule 5).

   This restores landing availability, not a "run here" fallback. An attested dependency still cannot
   be *verified* locally, because it has no local recipe. The native cure is for the pinned
   repositories (brit, rakia, sophia) to produce their own `DerivationRecord`s on a peer, so that the
   forge's check stops being the only witness. That is a frontier item (§13).
3. **Integrity-class governance fails closed.** The Rust evaluator already routes an unresolvable
   validator on a deny or ask rule to `ask` (`elohim/eprfs/eprfs-meta/src/evaluation.rs` ~:767-805).
   *Gap:* a rule guarding signing, attestation or evidence admission can be declared `inject`, and
   then stays advisory. Integrity-class rules need a marker that lints any class below `ask`.
4. **Versions are declared dependencies, never recency.** A method change is a new CID, a new key and
   a named miss.
5. **A revert propagates like an upgrade.** Tool releases (brit, epr, rakia, ark) publish as
   `release-manifest` channel heads on the dataplane, so a revert reaches peers. The bin_store
   GitHub-download arm is removed once the alone-path receipt is green.
6. **The verifier's own release obeys the verifier's rule.** One admission rule covers every tool,
   including `epr`, which today may bootstrap by local compile without the attestation brit demands.
7. **Retiring a design role requires its alone-path receipt first.** The roles are Jenkins dispatch,
   Harbor, Nexus and the forge CAS.
8. **No laundering.** *Gaps:* reanchor backfill still admits attestations; the release threshold
   counts deduplicated agent ids without enforcing diversity; DNA issuer authorization accepts all.
   Each is an integrity-class backlog item ruled by a Claude model.

## 8. Landing

**Ruling D3 [integrity] — the steward-space signed `dev` head is the landing record now. The forge is
a mirror, with four conditions.**

- **One head move, one expected prior.** Landing is a CAS on the space head, fenced by the existing
  lease fold's holder-side fence (steward-space D6: self-demote to `unconfirmed` when partitioned). The
  head-election threshold is a parameter, set to 1 today and raised to 2 at S2 without a new mechanism.
- **A silent sibling cannot block landing.** The rule-based override in plan row 9 is a recorded act.
  After the override delay, the lease view exempts the overridden device from acknowledgement and
  freshness checks (`elohim/eprfs/eprfs-peer/src/lease_view.rs:781`). So two partitioned devices can
  each accept a landing from the same prior H.
- **A fork is preserved, not lost.** The space head is a *set* of tips, like a DAG's heads. When two
  landings share a prior, both tips are retained as named heads, durably across restarts and
  reconciliation. Neither is dropped or replaced. The fork resolves only by a merge whose candidate
  contains both tips, and the merge lands as an ordinary CAS. Below the commons head divergence is
  normal and convergence is earned.
- **The mirror only ever fast-forwards.** The forge `dev` is pushed after the space head moves. The
  push is accepted only if the candidate descends from the forge's current tip. While the space head
  holds more than one tip, mirroring halts until the merge lands. A retry may refresh the expected
  prior only to a tip the candidate contains. Today an explicit lease skips brit's fast-forward check
  (`elohim/brit/gitoxide-core/src/repository/push.rs:707`), and that is a defect for this path. A
  forge-side change refuses the push and surfaces as a fork. Mirror lag is shown in the watch view
  (§9) and never waited on. Jenkins, still triggered by forge webhooks, sees landings with that lag;
  as a participant, it loses only speed.
- **The receipt includes the worst interleaving.** Two devices partition at H, both record an
  override, and both land. Then: both tips survive a restart; mirroring refuses until a merge contains
  both; and the merge lands and mirrors as a fast-forward.

Where this differs from the synthesis: D3 is **not ranked as a wall-clock move**. The forge round trip
is seconds; the 3,321 s lease p90 is gate time inside the lease, which S1b (Phase V gates holding
nothing) removes. D3 is ranked for the trajectory (premises 1–2), riding with S1b because both touch
the same hook.

## 9. The watch surface

**Ruling D10.** "One view of an intent" is a read-time fold keyed by the intent CID, hosted on
`epr flow context <intent-cid>`, which already works per subject and offline. It joins:

- landing-intent state;
- for each gate method, a hit, a miss or the typed `DerivationUnanswered` reason;
- carriers and their `basis`;
- the landing CAS head;
- mirror lag;
- the REA spend of every draw.

Its first lines are the typed unknowns: `claimed`, `unmeasured`, `held at self`, `not carried`. With
zero peers it prints "no peers: local record only". It stores nothing. It is a pure function of CIDs
that any peer can compute, so it never becomes a second identity for the intent. This is the first
thing an onboarding developer touches, so it teaches the honest states.

## 10. What the distributed store (plan row 5) must satisfy

[I-n] marks an existing acceptance invariant sharpened here; **[new]** marks an addition.

1. **[I3]** The record is `DerivationRecord`. Records sit side by side and never overwrite. A bad
   record is countered by a `Recomputed{matched: false}` record, never deleted.
2. **[I1]** A hit needs CID-verified bytes. Accepted but unfetchable bytes are `OutputNotFetchable`, a
   miss.
3. **[I2, I4]** Every non-hit is a typed reason. A store that is absent, slow, uncredentialed or empty
   changes speed and reach, never a verdict.
4. **[new]** **The first act is a local tier put:** write, address, record, flow event. Publishing and
   replicating are asynchronous widenings whose state is shown per copy. Row 5's "slice 1 PUTs to a
   server" is struck.
5. **[I5]** Artifact grain stays above rustc.
6. **[I6]** The signer resolves to `agent_cid`, or the hit is held at `self`. Agreement is computed by
   the reader (D4).
7. **[I7]** Every draw is a `Use` event and every build or carry a `Produce` event, with quantities
   (§6).
8. **[new]** Additive-open wire with an epoch. `basis: Unknown` is never accepted (D7).
9. **[new]** Lookup is reach-gated on the *request*. `AskPolicy::may_ask` runs before an input root,
   which is a hash of possibly private content, reaches any index.
10. **[new] Custody, ruling D6 [integrity].**
    - Cache and derivation bytes are a local `custody-blob` pledge (class B, born `self`) in the
      device's eprfs tier, with zero DHT heads.
    - Release and recovery-critical objects carry holder sets on the elohim DNA using existing types.
      `CustodianCommitment` and `ShardManifest` are entries and `BlobToReplicas` is a link
      (`content_store_integrity/src/lib.rs:631,3722`). Their blob and shard identities are still
      `sha256-…` strings (`content_store/src/lib.rs:16268`). Moving them to CIDs is an item in the
      batched DNA move, not a consequence of striking `CustodianAssignment`.
    - `node_registry::CustodianAssignment` is struck from plan row 5. Its authority is positional ("regional
      coordinator or quorum") and its address is a bare sha256.
    - Head budget: zero DHT heads at `self` or `intimate`. At wider reach, at most one attestation per
      converged derivation, **plus** the custody entries and anchors of each released object, counted
      separately. Their reconciliation cost is unmeasured.
11. **[new]** Large objects are chunked in the tier: chunks of at most 1 MiB, each named by CID in a
    manifest, and RS 4+3 sharding at community reach. The 454 MB storage binary is the acceptance case.
12. **[new]** The tier daemon mounts a blob store beside its channel store, with a `serve-blob` `Use`
    writer. No second protocol is added. **Serving enforces scope:** a carrier's grant covers exactly
    its task's projected view and declared outputs, and every served object is checked against its
    own reach. Today's tier gate ignores root and object CIDs, so any admitted device can read every
    mounted root (`elohim/eprfs/eprfs-peer/src/gate.rs:14,64`). That holds only because the channel's
    allow-list limits what is published, and a general blob mount removes that implicit boundary. A
    requester's `AskPolicy` cannot constrain a hostile requester. The test: an admitted carrier asks for
    an unrelated CID and is refused.
12a. **[new]** The local put works with the daemon down: a direct library write to the tier, or an
    explicit `queued, unrecorded` state, never a silent drop. Receipt: put and read with no daemon
    running.
13. **[new]** The I1–I7 failure tests (wipe, corrupt, kill a fetch, revoke a credential) run first on
    **one device with zero peers**, then between two of the steward's workspaces.
14. **[new]** No query plane. Lookup is by key or CID only.
15. **[new]** Source materialization (the sibling's objects, A7b), keyed by the input root, is a byte
    class on the same tier. **The carry needs it before it needs output artifacts.**

## 11. Sequence: the prize, parallel tracks, then the joins

**The prize:** two workspaces build once between them. A closure computed on one peer is reused, or
its bytes fetched, by the other. That record carries its spend and sits at honest reach. Every later
rung (Jenkins carrying, a second principal, community reach) widens that same record and path. Nothing
replaces it.

The work splits into **tracks with disjoint write-sets**, so separate agents can author them in
parallel and each reaches its own gate. Gates still serialize on one device: `elohim-storage` and the
eprfs workspace each peak at several GB, and the workspace RAM guard sheds concurrent compiles. Each
track names the wall-clock it is expected to remove and the receipt that shows it. Expected figures are
historical; each track takes fresh before-and-after gate-cycle rows.

### Parallel tracks (start together)

| Track | Write-set | Work | Why it is on the trajectory | Expected wall-clock | Receipt |
|---|---|---|---|---|---|
| **A — foundational** | `elohim/eprfs/eprfs-core`, `elohim/ark/cli/src/material.rs` | Slice 4 of placement-contract-interfaces §C: `walk_spine`, `project_view`, `derive_or_reuse`, the `P` vectors, `tests/derive.rs`. Retire ark's identity-projection workaround (`material.rs:101-113`). Key view declarations so a component's view can be asked for by its `.epr-meta` root (§1a). Retain unknown `ext` bytes and make `DerivationKey` decode per D7 | The one object that is both the cache key and the sparse checkout (D2); every later join keys on it | No standalone saving; enables every join | `P` vectors green; ark keys a task on a real projected root |
| **B — F0** | `genesis/orchestrator/gate-oracle.mjs`, `gate-runner.mjs` | Selection resolves through `brit build affected`, not the deprecated `rakia` PATH binary (`gate-oracle.mjs:24-37`) | Removes a retired tool from the reuse path | Restores dark reuse: up to a full gate (e.g. 5,757 s → 56 s) on a device without the shim | Reuse hit with no `rakia` on PATH |
| **C — record [integrity]** | `elohim/brit/brit-epr`, `brit-build-ref`, `elohim/brit/gitoxide-core/src/repository/push.rs`, `elohim/ark/cli/src/brit_index.rs` | brit signs `DerivationRecord` bodies (D7 framing); verification follows D7's three cases; notes carry records; old notes convert unsigned with a witness pointer; brit push keeps the fast-forward check under an explicit lease | The one unit every rung moves (D1) | No standalone saving; enables F1r | Signed-record vectors: tampered bodies and unknown non-`ext` fields refused, unknown epochs held; substituted method/input in a converted note never verifies; brit push refuses a non-fast-forward under an explicit lease; a Claude-family adversarial review before it lands |
| **D — honesty** | `genesis/orchestrator/gate-attest.mjs`, `.claude/scripts/habits-status.py`, `elohim/eprfs/eprfs-meta` | `claimed` gets its own exit code; a mix of passed and skipped reads `partly-measured`; an integrity-class marker lints rules declared below `ask` | Stops green-by-default before more peers read these verdicts | No saving; prevents false green | Unit tests on each |
| **E — tier blob store** | `elohim/eprfs/eprfs-peer`, `eprfs-storage` | Plan row 5 slice 1: the local tier blob put (chunked, CID-addressed, daemon-down path per §10.12a); then A11's tolerant reader; then a `serve-blob` mount with scope enforcement (§10.12) | A peer runtime holding a closure-keyed artifact, which the operator named | None until the join F2 | One-device I1–I7 failure tests with zero peers and with no daemon; the unauthorized-CID refusal |

**One soft dependency.** Track C reads track A's `DerivationRecord` and `ProjectedRoot` types, which
already exist in `derive.rs`. It authors and tests against them in parallel, using fixture roots. Its
*integration* waits on track A, because a real projected root needs `project_view` and D7's retained
decoding. A and C both touch ark, in different files (`material.rs` for A, `brit_index.rs` for C).
Every other pair of tracks shares neither files nor types.

### Joins (each waits on the tracks named)

| # | Join | Needs | Expected wall-clock | Receipt |
|---|---|---|---|---|
| F1r | **Live two-device sibling reuse** (plan row 7) on a real push. Includes the **producer**: `gate-task.mjs` hands ark a projected view of real source trees, not today's closure-metadata JSON (`gate-task.mjs:105,150`, `ark/cli/src/task.rs:107`). Also emits minimal `Use`/`Produce` events with quantities. Sibling reuse must also run when an ark task handles the gate; today it is bypassed (`gate-runner.mjs:461`, `!task`) | A, B, C, D | One full gate per closure on the second device | A `basis: Computed` sibling row, `independence: sibling`, with its `Use` event |
| F2 | The sibling fetches the mesh storage and doorway binaries from the peer's tier instead of rebuilding. Includes capturing `just mesh build` outputs into the tier as declared method outputs | A, C, E, F1r | One storage build (tens of minutes at `CARGO_BUILD_JOBS=1`) per closure per extra device | Fetch vs rebuild as `Use` vs `Produce`; the 454 MB chunked case |
| F3 | S1b (Phase V gates hold nothing; Phase L holds for the CAS window) with D3 riding along | Owned by the closure-keyed landing plan; D3's fast-forward fix comes from C; D first, so `claimed` never lands silently | Lease p90 3,321 s → the CAS window (S1b's own target p90 < 120 s) | Landing with the forge unreachable; the partition interleaving of §8; mirror catch-up as a fast-forward |
| F4 | D9 carry as the single offload path; source materialization by projected view; `just measure` re-plumbed | A, E, F1r | Requester wall-clock while a peer carries | `basis: UnderGrant` record; `NotCarried` when absent |
| F5 | One release plane for tools; `Work` in `ReaVerb`; deliverables as projections | C, D, E, F1r | Small | A changelog reproduced from fulfilled commitments |
| F6 | R3: a second principal, the signer → `agent_cid` binding, the graduation verb, the `derivation` attestation kind and the three integrity fixes of D5, in the batched DNA move | Standing redraft (§13), C, D | Reuse across principals | The one-human adversarial test yields k = 1; then the first graduation act |

The closure-keyed landing plan's S2b corpus split (eprfs and orchestrator re-gate 547–1,518 s per
disjoint landing) is owned by that plan and runs alongside track A–E authoring.

Doc drift is fixed in passing by whichever track touches the file: rakia `CLAUDE.md` still calls
`rakia-executor` absent, although it is tracked at the current pin, the README verb lists are stale, and epr's comment says "nine kinds" while the
enum has 12.

## 12. Ruling record

| Decision | Ruled | Integrity | Note |
|---|---|---|---|
| D1 record | `DerivationRecord`; old notes convert unsigned with a witness pointer | yes | All three lenses agreed. The adversarial review found ark's adapter re-using a signature; fixed |
| D2 key | View projection; slice 4 lands as foundational track A, no interim key | — | Operator steer 2026-11-04: knock out the foundational piece rather than key on a stopgap; it runs in parallel, so it costs no serial time |
| D3 landing | Space head now, forge as mirror | yes | **Differs:** four conditions (fence, fork-preserving head set, fast-forward-only mirror, the partition receipt). The review's worst interleaving drove the last three; ranked for trajectory, not wall-clock |
| D4 independence | Principals with standing | yes | Struck plan row 13's wording |
| D5 graduation | An enforced verb with per-principal standing proofs; new `derivation` kind; one per convergence; gated on three integrity fixes | yes | Rejected riding `device-health`. The review showed `to_attestation` authorizes nothing |
| D6 custody | Local B pledge for caches; DNA links for releases | yes | Struck `CustodianAssignment`. Two readers proposed reusing it; checking its shape in code (positional authority, bare sha256) decided against it |
| D7 wire law | Body/sig framing; `ext`-only additions within an epoch; unknown epoch held | yes | The review's dropped-restriction case drove the epoch rule |
| D8 executor | `ark task run` only | — | Retires rakia Stage 1 executor sprint |
| D9 offload | eprfs-peer carry only | — | Retires the `feedback_signal` path |
| D10 watch | Fold on `epr flow context` | — | Stores nothing |
| Sequence | Prize-first: five parallel tracks A–E, then joins F1r–F6 | — | **Differs:** parallel tracks on disjoint write-sets (operator steer); F2 (peer-held artifact) ahead of D3; S2b left with its owning plan |

Every integrity ruling was made by the session model (Claude). Codex readers contributed evidence, a
lens and the adversarial review (gpt-6-astra). Its two blockers (D3, D4/D5) and six majors were all
accepted on the evidence, and the session model wrote each cure. No integrity split went to a Codex
side.

## 13. Frontier

- **Settled by review:** gate-runner hands ark a JSON material file, which `ark task spec --material`
  turns into a tree of closure-metadata fields (`gate-task.mjs:150`, `ark/cli/src/task.rs:107`). F1r's
  producer replaces it.
- **Whether one human's devices share one `AgentRef`.** Settles whether F1r's sibling label is
  automatic. Read enrollment (`actor.rs`).
- **Mixed-peer compatibility.** `daemonDiffers` compares this CLI against the locally running daemon's
  CID (`epr-cli/src/peer.rs:846`), not against a peer's version. Whether two differently versioned
  peers interoperate is unverified. A11's two-version fold test settles it.
- **Custody reconciliation cost** for released objects (§10.10). Unmeasured.
- **Attested pins verified natively.** brit, rakia and sophia gate on a peer and write their own
  `DerivationRecord`s, so a forge outage no longer leaves pins as `claimed` debt (§7 rule 2).
- **Materialization cost for non-Rust closures** (the pnpm workspace, DNA in-tree `./target`, unlocked
  flake inputs). A projected-view size census once `project_view` lands settles it.
- **The standing record and council roster.** D4 and D5 assume a standing read, and the commons-pool
  and contextual-authority designs that would supply it failed review. F6 cannot start until standing
  is redrafted. That redraft is a Claude-ruled integrity design.
- **The sibling device has been silent about 30 h** (`epr peer status`). F1r needs both workspaces up.
