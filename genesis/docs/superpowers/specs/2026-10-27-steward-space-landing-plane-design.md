---
title: The steward space — a landing plane for two-agent sprints now, governed by the protocol it builds later
id: steward-space-landing-plane-design
status: Draft
class: process-meta
process_subdomain: agents
serves: [devices-share-one-berth, sprint-lands-each-closure-once]
date: 2026-10-27
context-tier: disclosed
steward: human:matthew
author: "Claude (orchestrator, Opus 5.5), at the operator's direction; design run wf_715ca915-117"
refines: workspaces-cooperate-as-devices-design
tags: [steward-space, berth, epr-peer, landing-queue, brit, rakia, qahal, constitution, gradient, local-first]
graduation-trigger: >-
  Two agent sessions on two of one steward's devices run a sprint against one Jenkins in which no
  closure is gated twice, no backlog item is claimed twice, every handoff carries bytes, dev-integration
  is held only for the CAS window (p90 < 120 s), and each Jenkins tip verdict reaches both devices' folds;
  proven by genesis/a2o/features/devflow/landing-queue.feature in-container and as a joined
  @requires:shem receipt.
cites:
  - "workspaces-cooperate-as-devices-design | the steward channel and steward-scoped leases this spec refines into a landing plane (slices A0-A13) | sha256:28296e002122f501 | path: genesis/docs/superpowers/specs/2026-10-23-workspaces-cooperate-as-devices-design.md"
  - "workspace-berth-carrying-capacity-design | berth records decisions, not messages; this spec freezes berth's vocabulary and moves conversation to the space | sha256:f7f53bb9e7686d19 | path: genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md"
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
  - "holochain-evolution-epic | §4.1 change classes and k-of-n council roster that the reflexive constitutional path (§5.5) routes protocol changes through | sha256:d821c5f45fd5d2e5 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "upgrade-revert-and-constitutional-consensus | the method (simulation, controlled trial, revertibility) S4 inherits; its 'propagation is consent' is corrected in §8 | sha256:4673f9958d96b617 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-14-upgrade-revert-and-constitutional-consensus.md"
  - "qahal-architecture-vision | the community pillar S2 graduates into: spaces, membership, message/thread as collective-space records | sha256:6a519b464b586832 | path: genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md"
  - "live-session-plane-design | presence is ephemeral and session-scoped; the presence plane of the log follows it | sha256:d1adbf86e46fd487 | path: genesis/docs/superpowers/specs/2026-10-14-live-session-plane-design.md"
  - "human-participant-actor-plane-design | actor grammar (human:, agent:role@model, session as lookup scope) behind A5's durable addressing and honest attribution | sha256:5f51c8e1c60887b8 | path: genesis/docs/superpowers/specs/2026-09-23-human-participant-actor-plane-design.md"
  - "compute-envelope-tevah | ark supervises the epr-peer daemon and the Jenkins witness (A4, A10) | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
---

# The steward space — a landing plane

> **Operator decisions (2026-10-27)**
>
> - **Agent standing:** agents are delegates now. Standing is earned by track record, never by a key or model label.
> - **Deliverable:** two arms, reliability first.
> - **One gradient:** friction is priced by reach × stakes on one machinery—near-frictionless inside the steward's grant and high bars at planet scale. Neither may leak into the other.

## 1. Thesis

### 1.1 What the channel is, in protocol terms
The "steward channel" is a **steward space**: a holon defined by its membership that holds its own activity ledger. At n=1 it is matthew's devices plus the agents he has delegated. At S1 its job is to be a **landing plane**. It turns "two agent sessions, one Jenkins, 40 h" into closures that are gated once, claimed once, handed off as bytes and witnessed once.

Operator-supplied session observations (unverified by a durable repository receipt) say acknowledgements took 5–15 s with no drops, while time was lost around the transport: a duplicate eprfs fix, a prose handoff, an approximately eight-hour push, a lease held through a one-to-three-hour gate run, and four latent edge wedges.

Arm A therefore makes "the link never stays down or wedges" first, then the throughput slices.

### 1.2 Three entities
| Entity | What it is | Headed? |
|---|---|---|
| **Space** | A stable identity. S1: "matthew's devices", from the steward chain root. From A12: `space_id = CID(genesis{chain_root, salt})`. | No |
| **Rulebook** | Governance EPR: crossing vocabulary, resources, settlement rule, relays, retention, amendment rule. S1: the pool-policy berth block as loaded by each daemon. S2: records on a declared head. | **Yes**, and it is the only headed thing |
| **Log** | Typed records, unheaded, carried by signed heads | No |

"Release channel" keeps its canon meaning. Rename "steward channel" to **steward space** (§8).

### 1.3 The log's three planes

The source meant by the earlier shorthand `commons plan:732` is not uniquely resolvable in this repository (unverified); the three-plane decision below stands on its own.
| Plane | Holds | Retention (stated honestly) |
|---|---|---|
| **Commitment** | claims, withdraws, landing intents, verdicts, overrides, amendments, halt/unhalt, adoption | Durable and append-only. Contest and counter-evidence are never pruned. |
| **Conversation** | notes, replies, acks | The author may tombstone a note. Honest holders then hide it and garbage-collect it after the retention window (14 d proposed). **A note that has crossed is assumed to be retained by its worst holder.** The sender-side secret refusal (A1) is the only real protection. |
| **Presence** | moorings, lease liveness | Head state once A12 lands; never stored |

Chat text never authorises anything. A note that mentions an act is not that act.

### 1.4 The gradient: one machinery, friction priced by reach × stakes
Every act, from an S0 lease to an S4 amendment, has the same shape: **intent → evidence → acceptance by (class × reach) → CAS → witness → revert**. Going up a rung changes only the acceptor set, the window and the evidence bar.

- **Down-leak test.** High bars attach only to integrity, constitution and governance-class paths, and to *adoption at reach*. Everything else lands on evidence. Inside the grant there is no council, window or sibling-review ceremony. Governance-class verdicts are batched, with one human verdict per sprint batch.
- **Up-leak test:**
  - Stakes follow **the reach of the effect**, never the proposer's identity or grant.
  - A `sibling` verdict is refused in every k-of-n count.
  - **k counts independent lineages, never species.** An agent's lineage is its delegator's until standing is earned.
  - A collective is a sibling space, never a widened personal roster.
  - Narrowing someone else's power is priced as a widening of the narrower's power.

**Floors that never get cheaper:**
1. Constitutional denies. Transcripts, recall receipts, private memory and keys never cross. This is compiled into eprfs-peer.
2. Counter-evidence and contest.
3. **Halting one's own grant.** Any actor may do it, and it takes effect immediately. Every member also keeps the algedonic *signal* right.
4. **The vocabulary and relays needed to amend, contest, halt and unhalt can never be narrowed.** This is a required floor, not current compiled behavior. The current default-deny crossing allow-list contains only `moor`, `unmoor`, `claim`, `renew`, `release`, `override`, `note`, and `shed` (`elohim/eprfs/eprfs-peer/src/allow.rs:29-38`; `genesis/agentic/pool-policy.json:152-156`).
5. The local relationship: one device alone always works for local work, and shared external resources fall back to evidence-priced override (A3/A5).

**The reflexive rule.** The channel never rides what it upgrades: it uses eprfs-peer over iroh, never kitsune2, the conductor or a DNA. The same holds one layer up: **the gate never judges a change to itself with its own candidate bytes** (the governance class, A9). Every wire or semantic change goes **expand → contract**, and records describe the semantics they were written under, so readers never depend on a global agreement.

---

## 2. Arm A: S0→S1 reliability slices, in dependency order

Each slice ends with a DELTA line in the habit atom it moves (§3), followed by `habits-project.py`.

**Minimum viable sprint path:** A0 → A1 → A2 → A3 → A4 → A5 → A6 → A7 → A9 (shadow, then on) → A10. After that: A8, A11–A13.

**Gate per slice:**
- Rust: `just gate eprfs` (device-runtime is an eprfs workspace member; `just gate elohim/eprfs/device-runtime` resolves the same project).
- Node: `node --test` under `just gate orchestrator`.
- Each a2o station runs `@act:host` (two devices in one container) before the `@requires:shem` receipt.

### A0. Measure and probe first (no code)
**Probes (read-only):**
1. **Closure-key equality.** Run `GATE_EVIDENCE=reuse` in report mode on both devices at one clean commit, for elohim-storage, doorway, steward-node and eprfs. Name any component that differs; the suspects are `rustc -vV`, `GATE_CARGO_PACKAGES` and the native evaluator. **A8 is blocked until this reads 100 %.**
2. **iroh relay compatibility.** Does an iroh 0.92 client (eprfs-peer pin `=0.92`) reach an iroh-relay 1.0.3? Use a throwaway endpoint, never the live daemon.
3. **Direct path.** Can the two Che pods reach each other directly? Is today's path direct or relayed (`epr peer status --json`)?
4. *(Closed by code read.)* A v1 fold lists and diffs only `channel/events` (`fold.rs:112-121`), so additive paths such as `channel/nacks/*` are ignored by v1 readers. The nack leaf is in-epoch.
5. Is origin public?
6. *(Closed by Jenkins `getJobs`, 2026-10-27.)* `elohim-orchestrator` indexes only `dev` and `PR-*`, but downstream multibranch jobs including `elohim-eprfs` discover and build `feat/*`, `fix/*`, `claude/*` and `dispatch/*`. A feature-branch offer therefore spends CI per offer.
7. **Edge selection.** For the last 20 dev pushes, compare the orchestrator's changeset selection (graph-walker) with the tags present. This sizes A9/A10's derived-resource rule.

**Baselines:** landings per 40 h; lease-hold seconds; re-gate seconds; duplicate claims; Jenkins waves per landed commit; link-down minutes; fold failures; daemon restarts; RSS; and `epr peer leases` latency. The earlier 448 MB and 0.22 s values are proposed baselines pending a durable measurement receipt.

**Measures:** the existing flow-bound machinery, `gate-cycle-seconds@1` and `delivery-series.mjs`. **No new register.**

**Rollback / wire:** none.

**Answers:** H1, M6, L4; critic A, D, E. The four formerly unavailable shem commits are now in local history; runtime-only shem observations still require receipts.

### A1. Keep the landed cures; finish sender honesty

The overnight cures are baseline on current dev, not work in this slice:

- `9d5e27da2`: bootstrap filters the current release with `admissible()` (`elohim/eprfs/device-runtime/src/bin_store/lifecycle.rs:121-132`).
- `6c0c4f7e7`: the Python resolver accepts `bafkrei…` and `bafyrei…` release CIDs (`.claude/scripts/_lib/epr_client.py:66-71`), not arbitrary verified codecs.
- `d8f4e4184` (merged by `7cd81fcb2`): a note that does not enter the channel store makes `berth say` exit 1 and say no other device will read it (`genesis/agentic/bin/berth:1738-1749`; `genesis/agentic/berth_test.py:939-965`).
- `f13cbdd33` (merged by `f8622594b`): eprfs CI tests no longer read host state.
- `a230c83a7`: pre-push refuses before gates when brit, rakia, or sophia is uninitialized (`.husky/pre-push.bash:41-52`).

Current `berth` uses exits 0, 1, 2, 3, and 4 (`genesis/agentic/bin/berth:82-87`), not 0/75/69/65/3.

**What remains:** add a device-private outbox; add the 16 KiB `env:text` bound and blob-CID path; add sender-side secret-pattern refusal; make every sender verb name delivery state; audit callers that require exit 0; and correct the berth text in `.claude/skills/hc-dev-orchestrator/SKILL.md:587-589` and `.codex/skills/hc-dev-orchestrator/SKILL.md:590-592` (line 219 is about the sign-in portal).

**Tests:** preserve the landed `bafyrei` and exit-1 regressions; prove outbox recovery within 60 s; refuse a 70 KiB note; and never report success without naming delivery state.

**Rollback:** `git revert`. **Wire:** none.
### A2. The fold never wedges; the link never stays down; refusals are visible
**Goal:** no single record stops a fold; a refused record is always visible to its author; an idle head costs nothing; a dropped neighbour comes back. The nack leaf moves here from the earlier A3, because an advancing fold is only safe if the author can see what was skipped.

**Re-join** (`daemon.rs:453`):
- A `NeighborDown` from an admitted node removes it from `joined` and schedules `join_peers` with 2 s→60 s backoff.
- A 30 s sweep re-joins any admitted node whose last head is older than 90 s.
- **Transport and gossip stream errors back off in-process without limit (capped at 60 s).** The daemon exits non-zero only on local faults: store, key or head table. A ~20 min relay churn during an edge redeploy must never exhaust ark's governor.
- The repository verifies the `iroh-gossip = "=0.92"` pin (`elohim/eprfs/eprfs-peer/Cargo.toml:29-33`). The precise HyParView source-line explanation is unverified because no `hyparview.rs` is tracked here; cite pinned upstream source before relying on that mechanism.

**Per-leaf faults, uniform across kinds.** The kind split is gone. An unread oversized leaf has no bytes and no kind, so no rule can split it.
- **Oversized** (`fold.rs:155-160`): record `quarantined{cid, path, reason: unread-oversize}` with no bytes.
- **Decode or canonical failure** (`:170`, `:244-256`): stop `?`-aborting the fetch. Verify the CID over the received bytes, keep them as received in `<state_dir>/quarantine/<device>.jsonl` (device-private, never crosses), and record `reason`.
- `:147` (over-budget) stays a clean incomplete break. It is a budget, not a fault.
- **Every quarantined CID is also written to this device's nack leaf** `channel/nacks/<device>` = `{refused:[cid], quarantined:[cid], refused_total}`, capped at 64 CIDs plus a total. On overflow the reader reads `unconfirmed`. The leaf changes only when the set changes. The existing signed head commits to its root, so the nack is a cosignature: *"I folded your head ≥ s; I accepted everything except these."*
- **The settlement reader honours nacks** (`lease_view.rs` `own_verdict` ≈`:600-647`). **effective** = every other admitted device has `folded[me] ≥ s` **and** the claim CID is in no peer's nack set (plus A3's fence). A nacked claim reads `refused by <peer>`, never silently effective. The fold also reads the peer's `channel/nacks/<me>`.
- **Advance rule (expand → contract, per author).** The receiver lets `folded[author]` advance past a quarantined leaf **only if that author's latest moor carries `env:reads=nacks/1`**. An older author keeps today's behaviour: fold incomplete, which is safe but wedged. No global agreement is needed. Each device's records state what it reads.
- **Author recovery without A13:** `berth withdraw <cid>` writes a commitment-plane `withdraw` record. The lease view treats the claim as released, so the next claim proceeds.

**No idle work:** when the heard root equals the last folded root, skip the connect and `folded_from`. `fetch_channel` already returns no work in that case (`fold.rs:115`); the remaining cost is the connect plus `folded_from`'s whole-store parse (`:383`). Replace that parse with an in-memory held-CID index per device.

**Small fixes:**
- Bounded work leases lapse by TTL. The `mesh` daemon lease remains the explicit no-TTL exception (`genesis/agentic/bin/berth:64-72`).
- Fix the false log line at `daemon.rs:817-821`.

**Tests (red first):**
- `fold_quarantine_and_nack`: a 100 KiB note, a 100 KiB lease claim and a newer-schema note in the middle of the CID order. With an `env:reads=nacks/1` author, `folded` advances, quarantined = 3, and the author's view reads the lease claim `refused by B`, never effective. With an old author, the fold stays incomplete.
- A dropped QUIC connection gives link-down under 60 s.
- A simulated relay outage of 20 min ends with the daemon still running (0 exits).
- 0 store parses per unchanged head; RSS ≤ 150 MB; `epr peer leases` < 30 ms.

**Rollback:** re-pin the previous release.

**Wire:** an in-epoch env slot plus an additive tree path. Probe 4 confirmed v1 readers ignore the path. Either device may go first.

**Answers:** H4, M3, critic I; codebase-truth and adversary quarantine findings; gradient "fail closed on the lease, not the fold".

### A3. Settlement holds under partition and skew
**Holder-side fence.** A holder treats its own lease as effective only while, from every other admitted device, it has a head received within `TTL − skew_margin` (margin 30 s) whose `folded[me]` covers its latest renew. Otherwise it **self-demotes to `unconfirmed`**, and phase L, dispatch and deploy refuse. Renew cadence is ≤ TTL/3.

**Override timing.** A peer may override a silent holder only after `TTL + max_skew + margin` has elapsed since it last heard the holder's head. Combined with the fence, this gives at most one effective holder *during* a partition, not only after it.

**One head move, one expected prior.** Every shared head move—lease, lane, landing tip, rakia channel head, or bin-store `current`—is one compare-and-swap that names its expected prior head. Content writes remain immutable and separate from the move. A stale writer is refused; it never silently replaces a head. This is the cure class for the 2026-10-27 launcher stranding: two readers had disagreed about one `current`, while [Anthropic Scion](https://www.anthropic.com/engineering/redesigning-source-control) and [code.storage's `expectedHeadSha`](https://code.storage/docs/guides/capture-agent-state.md) make that disagreement explicit at the move.

**Rules are advisory, computed where they are enforced.**
- The daemon computes `rules = sha256(canonical{crosses kinds, steward-scope names})` over **the allow-list it has loaded**, with comments stripped. It advertises this in `epr peer status`, and from A12 in the head.
- `berth` reads it from the daemon, never from a session's worktree. The digest is per device, not per session moor.
- The daemon **reloads pool-policy on change** (as it already does the roster at `daemon.rs:540`) and re-projects. While the file and the loaded list differ, status shows `rules: reloading`.
- Skew renders `rules skewed: <device> <x8>` in `berth status`. **It never gates settlement.**
- Readers are tolerant: a device whose allow-list is a superset of the kinds the sender wrote settles normally.
- A semantic change that both sides must share is carried **in the record itself** (for example A9's `env:order=cid-1`). A reader that does not know the value reads that record `unconfirmed`, and only that record.

**Tests:**
- `skewed_allow_lists_settle_on_nacks_not_digest`.
- A partition longer than TTL: an a2o station asserts **at most one effective holder throughout**.
- A comment-only pool-policy edit causes no flap.
- Two worktrees on different bases on one device produce one digest.
- `devices-share-one-berth` is re-reded honestly, then turned green (§3).

**Rollback:** re-pin. **Wire:** none new (status field; head field in A12).

**Answers:** H5, H8; gradient rules-digest blocker; adversary partition and digest-skew findings.

### A4. Supervised, safely upgradable daemon, and relay independence (before any wire bump)
**Supervision:**
- `genesis/agentic/peer/space-berth.json` is the proposed ark RuntimeManifest. ChildSpec `epr-peer` runs `artifact: {channel: bin:epr}` from the PVC bin store. Runtime use of that release path remains unverified without a captured `epr bin status --json` receipt.
- Env is declared, because `env_scrub` defaults on. `EPR_PEER_RELAY_URL` is renamed to `EPR_PEER_RELAYS`, with a compatibility read of the old name for one release (expand → contract).
- Readiness: `StdoutLine{contains:"epr peer: serving"}`. Restart: Permanent. Default governor (transport errors no longer cause exits; see A2).
- Started idempotently from SessionStart (`device-arrive.sh`; Codex through `epr device arrive`). §9 decision 13 asks whether to also launch it from a postStart entry that only backgrounds the launcher and always exits 0. A dormant device no longer stalls its sibling either way (A3 override timing, A9 intent liveness).

**Exec floor proven first:**
- Add a candidate-path preflight using existing `epr check`, `epr ready [--target REF] [--deep]`, and `epr peer serve`; there is no current `serve --check` (`elohim/eprfs/epr-cli/src/main.rs:394-395`). It proves exec floor, roster, head-table parse, store open, and loaded-rules digest without binding or writing a lease.
- Extend `bin_store/landing.rs:226` `health()` (called from `land` at `:114`) to run that preflight, so `current` never moves to a release that fails it.
- Move bin-store `current` by the A3 compare-and-swap, naming the expected prior release CID. Candidate bytes land immutably before that move; a launcher holding a stale prior gets a refusal it can diagnose rather than a stranded split view ([Anthropic Scion](https://www.anthropic.com/engineering/redesigning-source-control); [code.storage](https://code.storage/docs/guides/capture-agent-state.md)).
- Packaging carries a nonempty `envelope.wireEpochs` (`genesis/a2o/scripts/epr-release-package.ts:1733-1738`; `elohim/elohim-storage/src/services/release_adoption/verify.rs:436-440`). Epoch negotiation and drop-epoch refusal remain proposed and require verification against the initialized rakia pin.

**Fallback (corrected target):**
- Add admissible-predecessor fallback to `resolve_current` (`launch.rs:139-155`) and `resolve_mode` (`:191-292`).
- Change `record_running_daemon` (`launch.rs:323-372`) in the same slice to accept that predecessor; current behavior still refuses it as `daemon-not-current`.
- Rollback lives once, in the bin store. ark `RevertToPreviousClosure` stays unbuilt.

**Relays:**
- `endpoint.rs:50-53` uses `RelayMode::Custom(RelayMap)` from `EPR_PEER_RELAYS`.
- Direct roster addresses come first, through an additive `direct[]` on the PeerEndpoint row. Alpha comes last and is never the only path.
- `status.peers[].path = direct | relay:<url>`. Pre-push warns when it claims `alpha-fleet` while alpha is the only live path.
- `epr peer status` shows `supervised_by`.

**Tests:**
- After `kill -9`, the daemon is back in under 10 s, with a witness.
- Landing a broken candidate leaves `current` unmoved, with 0 s link-down.
- A forced `current` failure: the predecessor runs **and records its daemon lease**.
- Container restart: the daemon is back by the first SessionStart.
- With the first relay dead, the link is up in under 30 s.

**Rollback:** stop the manifest and run `epr peer serve` by hand. **Wire:** none.

### A5. Durable actors, honest attribution, a safe inbox, halt, priced overrides
**Attribution (provider principal, agent provenance):**
- On the real network the only human is `human:matthew`; every other human is a genesis test fixture. Agent output therefore records PRINCIPAL `human:matthew` as provider, with persona `agent:<role>@<model>`, session and device recorded as PROVENANCE. **"Vendor is provenance not payee."**
- `_observation.py:187-196` carries the persona, session and device provenance without minting a separate economic party. `ram-guard.py:379` takes the provider principal from `epr actor current --device`.
- Persona-level economic attribution and multi-human accounting are exercised in Simulacra, the declared network-stakes stage in `elohim/elohim-storage/src/trust/stage.rs`, until a second real human enrolls or an agent persona earns standing.
- A session's effective capability is the intersection of `human:matthew`'s grant and the `agent:<role>@<model>` role capability, never the whole device. This is capability attenuation, not a second principal: attribution remains PRINCIPAL `human:matthew`, with role, session and device as PROVENANCE. [GitLab's composite identity](https://docs.gitlab.com/user/duo_agent_platform/) supplies the intersection rule; this design keeps the A5 principal ruling.
- Rename `CLAIM_SIGNED_SOURCE` to `standing-device`. The change is additive.
- Reconcile `agent-identity-claim-and-acceptance.feature:73`.

> Operator, 2026-10-27: "H7 attributed to matthew (me) is correct.. everyone else here is part of our genesis test-fixtures.. We need to be able to test everything in a simulacra as if it were the mature network, but respect the boundary".

**Hook-stamped human-presence clause.** This remains proposed. Today `participant-standing.py` is a SessionStart hook that instructs the agent to ask and record clauses; no PostToolUse `AskUserQuestion` hook is configured, and it does not stamp human presence (`.claude/settings.json:19,226`; `.claude/hooks/participant-standing.py:229`). The slice adds a response-reading hook, then stops treating agent-recorded clauses as proof for human-only acts.
- **Until this hook is live, every "human-only" act below is cooperative at S1. The design states that plainly and claims nothing stronger.** Signing with a device key after a TTY confirmation is the S2 upgrade.

**Addressing:**
- `moor` carries `env:actor`, `env:steward` and `env:reads`. `actor-claim` records cross.
- `berth say --to lane:<id> | agent:<role>@<model> | human:<h> | device:<did>` is resolved on the receiving side. A note to a lane goes to its current holder. With no holder, it is `deferred` and surfaces to the next session that moors on that lane.
- `moor --succeeds <prior>` carries what was said, never what was granted.

**Inbox:**
- Rebase `feat/berth-inbox` (`ffa6f95bb`) onto the epr-rea verified reader.
- The acked cursor moves only on `ack` or `reply`. A separate shown cursor moves on print. `surfaced` is device-private.
- Messages render as `<untrusted-space-note cid=… from_device=… actor=… kind=…>escaped</untrusted-space-note>`, with control characters stripped and no ready-to-run reply line.
- Add and package an inbox-notice hook (name resolved at A5), capped at 600 bytes per prompt and kept out of `ram-guard.py`. No `space-inbox-notice` package exists today; the related current package is `.epr-meta/elohim/packages/hooks/device-head-notice.json`.
- **Scope of the injection claim:** framing protects the *comprehension* path. The *authority* path is protected only by the hook-stamped clause and the evidence-priced override. The tests below measure both separately.

**Waiting:** `berth wait --for carried|accepted|surfaced|acked|landed|green --timeout` exits 0, 124 or 1.

**Overrides priced by evidence, not by resource name:**
- An actor (agent or human) may override a silent device's claim on any resource once `TTL + max_skew + margin` has passed since that device's last heard head (A3). The claim has then lapsed by the receiver's own clock, so the override cuts nothing live. The override is recorded.
- **Before** that point, overriding `alpha-fleet` or `jenkins-dispatch` needs the hook-stamped human clause. `lane:` and `dev-integration` stay agent-overridable before expiry, recorded and bounded by GitHub's CAS.
- Every bounded work lease has a TTL; the no-TTL `mesh` daemon lease is outside this override path. An idle Che restart therefore waits at most the work lease's TTL + margin.

**Halt (a commitment-plane record, not a lease):**
- `halt{scope: grant:<steward>, reason}` may be taken by **any actor within that grant**, agent or human. Attenuating one's own grant is cheap, so it takes effect on fold. The record names its actor.
- **It is sticky.** Unmoor, mooring TTL and liveness expiry never touch it. Only an `unhalt{cites: <halt cid>}` record lifts it. A device that has not folded the unhalt stays halted.
- `unhalt` requires the hook-stamped human clause; until that exists, the cooperative caveat above applies. This is a widening, so it carries the higher price.
- While a halt is in force: every phase L refuses, new steward claims by agents are refused, prompt hooks surface the halt first, and the A10 witness performs no auto-revert.
- **Scope floor at every rung:** a halt binds only the halter's own grant (their devices and delegates). The S2 form is in §5.1.
- No PreToolUse network blocker.

**Ambient line:** `berth status --household` and the SessionStart headline show:
- the space's devices;
- the advisory rules digest and any skew;
- the queue head;
- claims;
- the last witness verdict;
- quarantine and nack counts;
- halt status.

**Tests (a2o stations):**
- All 13 agent-produced notes carry PRINCIPAL `human:matthew`; persona, session and device remain queryable as PROVENANCE. The same scenarios run in Simulacra with fixture principals to exercise mature-network accounting without promoting fixtures into real-network standing.
- A note to `lane:X` survives `/clear` and reaches the next holder.
- An injected "run berth claim alpha-fleet --offline-ok" renders framed. With the hook live, a fleet override **before expiry** without a stamped clause is refused, even when the agent runs it with `--as human:matthew`.
- An agent's halt reaches every live session on both devices within one fold. An unmoor does not lift it. An unhalt without a stamped clause is refused.
- Kill shem's container while it holds `jenkins-dispatch`: ethosengine overrides with no human within TTL + margin, and the override is recorded.

**Rollback:** revert the hook package; provider-principal attribution remains, while hook-stamped authority checks return to their cooperative S1 state. **Wire:** additive in-epoch kinds (`actor-claim`, `ack`, `withdraw`, `halt`, `unhalt`).

**Answers:** H2, H6, **H7 rejected for the real network by operator ruling 2026-10-27**, M10, L1, L2, L5; critic B; findings on human-only acts, halt self-lifting, offline-ok stalls and halt species-terminal authority.

### A6. Work claims cross devices

Current `berth claim` accepts resource, class, TTL, note, closure, and offline override; it has no `env:satisfies` (`genesis/agentic/bin/berth:30-35,530-536`). Current `epr flow claim` claims projected intents with `gap:claimed`; it reads the local flow sidecar and standing commitments, not steward leases or `via:` records (`elohim/eprfs/epr-cli/src/flow/claim.rs:47-59,171-173,214-254`). Steward leases and `via:` records are projected in `elohim/eprfs/eprfs-peer/src/lease_view.rs:201-223`. There is no finding-claim kind and no `epr flow ready`; top-level `epr ready` is the governance/reach check.

This slice adds the smallest bridge between projected intent identity and the existing berth resource claim. Failure-fingerprint claims remain a later extension.

Each projected work item also gets a stable change id. It is carried as a commit trailer, survives amend and rebase, and is distinct from the content-specific commit oid. Claims key on this id, not on a SHA. The model is [jj's change id](https://docs.jj-vcs.dev/latest/glossary/) and Gerrit's [`Change-Id`](https://gerrit-review.googlesource.com/Documentation/user-changeid.html); Tangled independently keys stacked rounds by jj change id ([source](https://blog.tangled.org/stacking)).

- **Test:** two devices claim one projected gap within 30 s; the second is refused, naming the holder, before any build.
- **Rollback:** revert. **Wire:** in-epoch.

### A7. Byte handoff: an offer names an oid, never prose

**A7b (primary, local-first).** Add `epr peer offer` and a `bundle:` carrier. The sender writes the Git bundle into the steward space's own eprfs tree; the receiver fetches it over the tier and verifies the content-addressed Git oid. The artifact is stored as CIDv1 chunks no larger than 1 MiB and named by a manifest entry that states what it is, its base and its tip. No `epr peer offer`, `bundle:` carrier, or peer-offer protocol exists today (`elohim/eprfs/epr-cli/src/peer.rs:76-108`), so this protocol lands before byte handoff is claimed.

**A7a (fallback, no Rust).** Introduce a proposed `dev-landing@1 offer {oid, base, carrier}` crossing. Today the sole crossing measure is `dev-berth-events@1`, admitting only `moor`, `unmoor`, `claim`, `renew`, `release`, `override`, `note`, and `shed` (`genesis/agentic/pool-policy.json:151-157`). `refs/offers/<lane>` on origin is used only when the peer link is down. `feat/<lane>` branches are never offer carriers. The carrier is posted landing intents only, never drafts.

The offer record carries `carrier: bundle:<cid> | ref:refs/offers/<lane>`. Switching carrier is data, not a branch convention.

The offer carries the A6 stable change id as a commit trailer and keys the handoff on it, not on the offered oid. The oid still identifies the exact bytes for verification; the change id identifies the claim across rebases ([jj glossary](https://docs.jj-vcs.dev/latest/glossary/); [Gerrit Change-Id](https://gerrit-review.googlesource.com/Documentation/user-changeid.html)).

Every offer also carries a `walkthrough` record: claims, proof entries by kind (`attestation_cid`, `gate_verdict`, `build_number`), and an explicit `unproven` list. The walkthrough is frozen with the landed revision and cites the stable change id plus the exact offered oid. Garden's walkthroughs make claims and `unproven` first-class ([source](https://docs.gardenscm.ai/concepts/walkthroughs/)); the overnight re-typed prose fix would have been checkable under this shape.

**Rakia primitive.** The bundle is an artifact from day one: ≤1 MiB CIDv1 chunks on the dataplane, named by a manifest entry carrying kind, base and tip. The bundle emits a ValueFlows `produce` event beside the artifact, accounted to PRINCIPAL `human:matthew` with agent persona, session and device as provenance, and carries `depends-on` and `cites` edges to upstream contributions so later value tracing does not require reconstructing Git history.

**Test:** land a sibling fix with identical `git diff --stat` over the tier; repeat with the peer link down through `refs/offers/<lane>`; measure a 40 h sprint's bundle set against the 1 MiB chunk bound and store growth.

**Rollback:** revert. **Wire:** proposed additive ALPN request (A7b); proposed in-epoch fallback crossing (A7a).

### A8. Sibling verdict reuse (strictly gated on A0 probe 1)

Today reuse is strictly self-signed: `gate-evidence.mjs:283-296` refuses a row whose `attestation.agentId` is not this workspace, and tests expect `(signer this workspace)` (`genesis/orchestrator/gate-evidence.test.mjs:447-470,622-648`). There is no `binds-signer` participant row—current rows are `Genesis`, `Binding`, `Contest`, and `PeerEndpoint` (`elohim/epr-rea/src/participants.rs:42-111`)—and no independence field or k-of-n evidence count. The visible caller stores `{exit, seconds, jobs?}`; brit's schema is unverified while its submodule is uninitialized.

The slice initializes and verifies brit, defines proposed `binds-signer` vocabulary (the current validated Mishpat action is `binds-identity`), specifies signer admission and signature verification, adds explicit sibling independence without counting siblings in governance quorums, and extends tests from current other-signer refusal to admitted-sibling reuse.

**Rakia primitive.** The verdict is a brit attestation whose subject is the closure key and, when the run produced bytes, whose `output_cid` is the artifact CID; today that is a run-record CID under rakia ruling R12 c. Reusing a sibling's verdict is reusing an attested artifact: the build cache. It emits a ValueFlows `use` event beside the artifact, accounted to PRINCIPAL `human:matthew` with agent persona, session and device as provenance, and carries the artifact's `depends-on` and `cites` contribution edges.

A8 remains blocked on A0 probe 1 and a reviewed participant-binding design.

### A9. Landing-intent queue; `dev-integration` shrinks to the CAS window

Everything here is proposed unless identified as current. No landing queue, train absorption, change-class resolver, or landing trailers exist today. Current integration claims before gates, defaults to a 1,800-second TTL, renews every TTL/3, holds leases through gates, and then pushes (`.husky/pre-push.bash:115-145,194-227,255-327`). Current `BERTH_NEEDS` remains commit-tag-derived at lines 260-270; untagged storage or doorway paths do not add `alpha-fleet`.

**Record:** introduce `dev-landing@1 intent {change_id, base, tip oid, carrier, per-gate closure keys + evidence hit/miss/signer, eta, actor, lane, state}`. The `change_id` is the A6 stable trailer identity, so the intent and its attestations survive rebase; base and tip still bind the exact candidate bytes. The integrator recomputes class, paths, and dispatch from the fetched oid and base-tree manifests.

The first working form is a brit note in `refs/notes/brit/proposal`: a forge-independent proposal written by agents and invisible to Jenkins. It is the implementable precursor to brit's planned `MergeProposalContentNode`, carrying the intent, walkthrough and evidence links around the commit. Governance tallies do not enter brit: brit reads the consent requirements, while qahal owns deliberation and tallies. Git notes are already demonstrated as a mergeable review and CI sidecar by [git-appraise](https://github.com/google/git-appraise).

**Rakia primitive.** A landing intent references closure CIDs and artifact CIDs; the queue orders the constellation, not branch names. It emits a ValueFlows `deliver-service` event beside the artifact, accounted to PRINCIPAL `human:matthew` with agent persona, session and device as provenance, and carries the artifact's `depends-on` and `cites` contribution edges.

**Change-class resolver:** add one resolver behind a loosely coupled interface; none is currently declared in `genesis/orchestrator/.epr-meta/`, whose manifest adds no author-time rule (`manifest.md:18-32`). The proposed classes remain 0, low, channel, governance, high, and global.

**Queue:** add a landing-intent projection. `lease_view.rs` currently projects steward leases and tie-breaks concurrent leases by `claimed_at`, device DID, then session (`elohim/eprfs/eprfs-peer/src/lease_view.rs:16-28`); it does not implement causal landing order or `env:order=cid-1`.

Each landing-tip advance is the A3 compare-and-swap over one expected prior tip. The proposal note and candidate content are immutable writes completed before that move; queue projection never turns a set of content writes into an implicit multi-head transaction ([Anthropic Scion](https://www.anthropic.com/engineering/redesigning-source-control)).

**Liveness:** current heads contain only `device`, `root`, `seq`, `at`, `folded`, and `signature` (`elohim/eprfs/eprfs-peer/src/head.rs:26-40`). Proposed `dormant_after_s`, `live[]`, dormant skipping, and absorption require A12 first.

**Pre-push transition** (current target/message collection at `.husky/pre-push.bash:238-253`; lease block at `:255-327`):

1. Phase V gates without a lease and posts an intent.
2. Phase L claims CAS resources for a proposed 600-second TTL, recomputes dispatch from the diff, and pushes only on evidence-complete fast-forward or admissible disjoint rebase.
3. Replace tag-only resource derivation only after the resolver exists.
4. Add train absorption and trailers only with the queue.

**Acceptance at S1:** preserve class × reach. High-class merge and adoption remain distinct requirements, but no `scripts/ci/edge-adoption-gate.sh` or `genesis/manifests/adoptions/` gate exists. Current controls are runtime flags and release-lineage machinery. On an app with cells, `ALLOW_DNA_REINSTALL` no longer authorizes integrity-hash drift: migration needs `DNA_MIGRATION_INTENT` covering each drifted role's new bundle hash, or destructive `FORCE_DNA_REINSTALL=wipe` (`elohim/elohim-storage/src/happ_manager.rs:24-38,121-153`). A9 must compose with that machinery.

**Rollout:** `LANDING_QUEUE=shadow` for one sprint, then `on`; remove the lease-over-gates path only after equivalent coverage.

**Tests:** retain the original outcomes—CAS lease p90 under 120 s, no gates inside it, queue liveness, path-derived claims, base-recipe governance judgment, and adoption hold—as red-first specifications.

### A10. Jenkins as witness of train tips

No ark-supervised `lane:ci-witness` process or lease exists. The current observer is `.claude/scripts/ci-harvest.py`, with cursor and JSONL state; its SessionStart hook runs unconditionally and does not inspect a witness lease or EPR fold (`.claude/settings.json:43`; `.claude/scripts/ci-harvest.py:913-926`). Its `--wait` mode performs bounded polling (`:27-28,932-942`), so witness reads are not the repository's only polling.

The slice adds a non-LLM witness behind a replaceable observer interface. It records one verdict per orchestrator run, carries landing-intent references once trailers exist, and holds derived resources only while their wave is in flight. Until A9 lands, it cannot infer trains or trailer membership.

**Rakia primitive.** The witness's tip verdict attests the same closure CIDs. Jenkins is one forge among many, and its outputs land as custodied artifacts. The verdict emits a ValueFlows `custody` event beside each artifact, accounted to PRINCIPAL `human:matthew` with agent persona, session and device as provenance, and carries the artifact's `depends-on` and `cites` contribution edges.

**On a red tip:** re-gate locally with evidence reuse disabled; classify no-local-repro as environment/unknown; revert only on reproduced intent-specific evidence; allow at most one class-0 auto-revert per sprint; never auto-revert governance or higher classes.

**Done:** a green witness verdict on the tip carrying the intent.

**Tests:** one read per wave for the new witness, zero JSONL merge conflicts, no revert without local reproduction, and no edge wave cut by a derived-resource landing.

### A11. Wire expand: the tolerant reader everywhere

Current verification reconstructs canonical DAG-CBOR from known `Body` fields; it does not verify the original received byte sequence (`elohim/eprfs/eprfs-peer/src/head.rs:43-55,88-99,108-125`). `HeadVerdict` has no `PeerNewer`; signature failure remains `Forged` (`announce.rs:109-145`). Current ALPN is `/elohim/eprfs-tier/1.0.0` and the topic prefix is `elohim/steward-channel/1/` (`tier.rs:38-39`; `announce.rs:29-37`).

This slice designs a tolerant reader and dual-v2 subscription explicitly. It chooses what exact bytes signatures cover and encodes that choice in tests. The proposed v2 ALPN and space topic are not current.

**Test:** mixed-version peers settle both ways without misclassifying an understood valid head as forged. **Exit:** every admitted device advertises the v2-capable release.

### A12. Wire contract: HeadV2, liveness in the head, space topic, native verbs

Only v1 `ChannelHead {device, root, seq, at, folded, signature}` exists today. There is no `HeadV2`, `live/1`, top-level `epr space`, or native `say|inbox|ack|view`. `berth say` remains Python and already exits 1 when its note does not reach the channel store. Berth and pre-push continue renewal records.

The proposed contract remains: HeadV2 adds epoch, capabilities, advisory rules, nack digest, liveness, and binary hints; topic v2 is space-derived; native space verbs replace wrappers only after the CLI exists; renewals stop only after measured deployed liveness on every admitted peer; mixed-epoch and partition tests gate contraction.

HeadV2 does not weaken A3: publishing a new signed space head is one compare-and-swap naming the exact prior head. The records and content root are immutable writes; lease, lane, landing and rakia-channel consumers each reject a stale expected prior rather than reconciling two notions of `current` after the fact ([Anthropic Scion](https://www.anthropic.com/engineering/redesigning-source-control)).

Current bin status exposes `head` as a release CID plus `resolvedCid` and `resolved`; binary members live in `Release.bins`. There is no current `head.bins.epr` or head `caps` (`device-runtime/src/bin_store/mod.rs:213-221,276-293`).

### A13. The channel's own store and retention (triggered tail)

Current `epr peer serve` exposes `--root`, not `--store-root`, and has no plane/origin/week sharding, per-plane retention, or channel tombstones (`elohim/eprfs/epr-cli/src/peer.rs:76-89,113-120`). Default TreeLimits are 4 MiB and 100,000 entries per node; the earlier 35–40k flat-node ceiling is unverified and needs a named measurement plus entry-shape assumptions.

Triggered by measured pressure or before S2, this slice may add an explicit PVC root, idempotent import, sharding, retention, and tombstones. The server-forgery claim is unverified: no precise accepted record type, attack, or predicate was identified. Define those before relying on it.

**Test:** three daemons, where C verifies A's record fetched from B; 100,000 representative records project within declared limits.

### Cost and operator hats
- **Wakes** happen only at prompt or idle boundaries, and only for: head of queue, ejection, halt, or a red tip from the witness.
- **No agent wakes another agent.** There is no ark auto-spawn of headless sessions at S1. The ark-supervised witness is a script.
- **Polling:** the proposed witness bounds and accounts for its own reads; it is not the repository's only polling.
- **Token spend:** declared unwired; the sprint brief states a budget.
- **What reaches the human:**
  - governance-class verdicts, batched per sprint;
  - high-class verdicts and adoptions;
  - pre-expiry fleet overrides;
  - unhalt;
  - one escalation per unexplained red.

  These come one at a time and are routed by stakes.

**Threat model at S1 (stated):** an injected or confused delegate inside the grant can write any record its shell can write. The defences are:
- framing on the comprehension path;
- hook-stamped clauses on the human-only acts, cooperative until that hook ships;
- the governance class, so the gate is never judged by its own candidate;
- the base-tree recompute of class and selection;
- GitHub CAS as the last word.

Device keys per act are the S2 upgrade.

---

## 3. The habits Arm A serves (and the WIP fence)
The fence is full: the repository reports 57 declared habits, with `dataplane-convergence` and `push-delivers-within-budget` active. Nothing is displaced. Arm A is licensed as the upstream station of the active red.

**Atom, minted `status: red`, `active: false` (decision D1):** `genesis/orchestrator/.epr-meta/sprint-lands-each-closure-once.habit.md`.
```yaml
id: sprint-lands-each-closure-once
invariant: >
  Within one steward's grant: a closure is gated once across his devices; a backlog item is
  claimed by one actor; a handoff carries bytes, never prose; dev-integration is held only for the
  CAS window; a dormant author never blocks the queue; every dev tip names the landing intents it
  carries; the one Jenkins verdict on that tip reaches every device's fold; no change to the gate is
  judged by its own candidate bytes; and a said obligation reaches an actor who can act on it, or its
  sender learns by exit code that it did not.
checks:
  - "a2o @concern:sprint-lands-each-closure-once (genesis/a2o/features/devflow/landing-queue.feature,
     @act:host): duplicate claim refused naming holder; offer fetched by oid; sibling green reused
     and labelled independence:sibling; intent queues, never refused; dormant head skipped and the
     next lands within dormant_after_s+600s; CAS lease < 120 s; edge-selecting train waits on
     alpha-fleet during an edge wave; governance-class change judged by base recipe; red with no
     local repro reverts nothing; note to lane:X reaches its current holder"
  - "the same tag and @requires:shem (joined real-workspace receipt)"
  - "node --test genesis/orchestrator/gate-evidence.test.mjs (sibling-signer cases)"
  - "per-sprint bounds over gate-cycle-seconds@1 + delivery-series.mjs: duplicate gated closures = 0,
     duplicate claims = 0, lease p90 < 120 s, >= 2 intents per Jenkins wave (unwired until A0)"
retire-when: >
  landing intents, verdicts and change-class acceptance are served by brit-native integrate over the
  dataplane in a collective space (S2), so this repo-local queue adds no information; or it retires
  together with push-delivers-within-budget.
```

**Channel reliability → the existing `devices-share-one-berth`** (green, but `last_measured: never`). The new stations are:
- quarantine and nack without a wedge (with old and new authors);
- an oversized lease reads `refused by` and never `effective`;
- a partition yields at most one effective holder throughout;
- a comment-only policy edit causes no flap;
- `kill -9` under ark;
- the predecessor fallback records its lease;
- a 20-min relay outage yields 0 exits;
- the first relay dead;
- mixed-epoch settle;
- sender delivery-state coverage beyond the already-landed non-crossing exit-1 behavior.

The red-first evidence is recorded honestly before the green.

**SLO for the 40 h sprint:**
- 0 silent losses;
- carried→accepted p90 ≤ 15 s;
- 0 s link-down from our own upgrades;
- ≤ 5 min link-down while both devices are up;
- 0 wedged folds;
- idle RSS ≤ 150 MB;
- 0 human asks caused by an idle device restart.

**Other DELTAs:**
- A5 → `acts-attributed-to-participants`
- A8 → `push-gate-pays-once`
- A9/A10 → `push-delivers-within-budget`
- A13 → `ledgers-fold-once`
- Arm B → `authority-in-integrity` and `constitution-ratified` (unwired)

**Why proposed-inactive is honest:** A9–A10 move the active red's measured ratio, and A0–A8 are its preconditions. Whether and when to mint or activate it is resolved by a decision agent at the §3 milestone.

---

## 4. Entity table: p2p-design-gate × local-first × gradient

**Nothing in Arm A adds an `#[hdk_entry_types]` variant or moves a DNA hash.** The only entry-type addition in the whole plan is the mishpat `CouncilRoster` entry, shipped inside the single mishpat lineage-authority crossing (§5.5).

| Entity | Class (S1 → later) | Address | Reach | Retention | Head-plane cost 1 yr | DNA hash | Alone on one device | Where friction sits |
|---|---|---|---|---|---|---|---|---|
| Space | B2 lineage-derived → links to imagodei `Collective` (S2) | chain root → `CID(genesis{chain_root,salt})` | self → familiar | forever | 0 | none | founded alone | founding = human act |
| Rulebook | S1 C (daemon-loaded policy, advisory digest) → B2 records (S2) → A on `Content` valve (S4 only) | sha256 → CID + `version_parent[]` | as space | append-only | 0; at S4 1 head, 10–50 versions | none | device enforces its own | self-narrowing immediate; narrowing others = widening; widening by steward (S1), non-author Steward (S2), k-of-n (S4) |
| Commitment record (claim, withdraw, intent, verdict, override, halt/unhalt) | B → origin-signed (A13) | content CID | intimate | durable | 0; ~18k/yr pre-A12 | none | effective at once (queue of one) | evidence by class |
| Conversation | C | CID, `to`, `in-reply-to` | intimate | author tombstone; worst holder assumed to retain | 0; 5–20k/yr | none | yes | free in grant; framed on receive |
| Presence | C (head state after A12) | in head | members | seconds | replaces ~25k renews/yr | none | yes | none |
| Nack leaf | C inside signed root | `channel/nacks/<device>` | members | latest | 1 per peer | none | n/a | owed unconditionally |
| Quarantine entry | B device-private | (origin, cid, path, reason) | self | until resolved | 0 | none | n/a | counted and surfaced |
| Outbox / `surfaced` | B / C device-private | CID | self | until flushed / derived | 0 | none | yes | none |
| Work claim | B lease + C view | (resource, holder) | intimate | bounded work leases have TTLs; the `mesh` daemon lease is the explicit no-TTL exception | 0 | none | yes | free |
| Landing intent | B (S1) → **B collective-space record held by members (S2)**; only the *landed release / ratified lineage* is notarised on the valve | event CID | intimate → space | landed + 30 d, then the `landed` row | 0 on any DHT | none | queue of one | class × reach |
| Queue position | C | (resource, intent) | local | none | 0 | none | yes | none |
| Offer | B | git oid / bundle manifest CID | intimate (`bundle:<cid>` primary; `refs/offers/<lane>` only while the peer link is down) | landed + 7 d | 0 | none | written and readable locally | tier transfer; origin fallback only |
| Sibling gate verdict | B2 brit-signed | closure key | intimate | per policy | 0 | none | unchanged | reuse only; refused in k-of-n |
| proposed `binds-signer` (current Mishpat vocabulary is `binds-identity`) | B2 | (did, agentId) | self | until rotated | 0 | none | yes | enrolment = human act |
| CI witness verdict | B, `service:jenkins`, actor = holder | CID | intimate | durable | 0 | none | lands unwitnessed, named | witness only |
| Rules digest | C (daemon status → head) | sha256 | intimate | current | 0 | none | yes | advisory, never gating |
| Halt / unhalt | B commitment kind; scope = halter's grant (all rungs); S2 adds the algedonic deliberation record | (scope, CID) | grant | until cited by unhalt | 0 | none | stops own sessions | take: any actor, immediate; unhalt: hook-stamped human clause |
| Hook-stamped clause | B device-private, written by the hook process | tool_use_id | self | session | 0 | none | yes | the S1 human-presence proof |
| Adoption record | proposed S1 gate composed with current migration-intent/release-adoption machinery → A `Commitment` adoption (S3+) | (role, dna hash) | alpha → lineage | durable | a handful/yr | none | household receipt local | human verdict + household receipt |
| Change class | C computed from base-tree resolver over oid diff | — | — | recomputed | 0 | none | yes | strictest wins |
| Agent standing | C derived **per space** | — | space | recomputed | 0 | none | yes | never authority |
| Delegation key + lane cap (S2) | B2 | CID | self → space | caps live + 7 d | 0 | none | checked locally | ≤ 1 human ask per device per 30 d |
| **Mishpat `CouncilRoster` (lineage-authority crossing)** | A, mishpat integrity entry, hash-chained to predecessor; genesis signed by mishpat progenitor | entry hash | lineage → global | permanent | a handful/yr | **part of the one mishpat move** | proposal and simulation local | roster change = k-of-n of the prior roster |
| Ratification | A mishpat `Commitment` (`migrates-lineage`/`sunsets-lineage`; `amends-constitution` at S4), integrity-validated against `CouncilRoster` | entry hash | lineage → global | permanent | a handful/yr | **one mishpat hash move (§5.5)**; other roles unchanged | local | highest at global |
| qahal thread (S2) | derived `in_reply_to` links over note CIDs; per-member B | first note CID | space | as conversation | 0 | none | yes | — |

**The qahal DNA-hash choice stands.**
- Message and thread are never imagodei entry or link types.
- No extension of mishpat `Discussion`.
- Governance anchors ride `Commitment` metadata.
- Per-message value coupling needs a P2P-gate pass first.

**Build vs adopt:**
- **iroh-docs:** rejected for this commitment plane. The earlier last-writer-wins rationale is unverified; the repository records only future read-cap work (`2026-09-12-memory-search-scale-three-seams-design.md:208-216`).
- **p2panda:** its ladder is borrowed; the dependency is decided at the iroh 1.0 migration.
- **automerge:** not limited to co-edited plans today; storage projects broadcast-tier content metadata and bodies through it (`elohim/elohim-storage/src/sync/projector.rs:1-7,42-55,96-120`). This design does not extend it into the commitment plane.
- **MLS:** private S2 space remains blocked on an encryption decision. MLS was considered and set aside as premature (`genesis/research/p2panda-cross-pollination-2026-08-04.md:31-33`); no adoption is recorded.
- **Nostr:** NIP-29/`previous`/NIP-17 are borrowed; relay-as-authority is rejected.
- **Fossil:** prior art only.
- **Countersigning:** rejected at every rung.
- **Warrants:** adopted at S3/S4.

---

## 5. Arm B: graduation rungs S2→S4
Each rung works with the rungs above absent. Triggers are evidence, never dates.

### 5.1 S2: a stewarded dev collective
**Trigger (all of):**
- a second human holds an enrolled device;
- A12 is closed and A13 is green;
- `sprint-lands-each-closure-once` has been green for 2 or more sprints;
- MLS-class and iroh 1.0 / iroh-docs decision records exist;
- `epr` runs outside Che.

**Mechanism:**
- **Sibling space.** It links to `collective:ethosengine/elohim` (`familiar`), with admission by `Membership`. Matthew's personal space continues. One store serves both topics.
- **Willow shape.** The collective space is owned; each member owns a communal subspace `space/<plane>/<origin>`.
- **Resources move one at a time with a dual window:** first `dev-integration`, then `jenkins-dispatch` and `alpha-fleet`. This is this design's proposed order; `genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md:110-118` specifies draw admission, not this migration.
- **Lock-free where divergence is cheap.** Notes, threads, lane-local heads and other reversible records accept concurrent writes and surface divergence for resolution. Leases remain only for irreversible or expensive acts such as Jenkins dispatch, alpha-fleet use and canonical landing. [jj's operation log](https://docs.jj-vcs.dev/latest/technical/concurrency/) proves the detect-and-resolve shape; a refusal is not the default coordination mechanism.
- **Rulebook as a governed EPR:**
  - `rulebook-amend` records take effect on fold. Git is a mirror (G4).
  - **Narrowing your own grant is immediate.** A narrowing that cuts another member's resources, kinds or standing is priced as a widening of the narrower's power: it needs a non-author Steward verdict and expires after a short window unless confirmed.
  - **The amend, contest, halt and unhalt kinds and the relays list cannot be narrowed by any attenuation.** This is a required S2 implementation; the current compiled crossing floor does not contain those kinds.
  - Devices bind `latest-earned` or `pin:<cid>`. The head arm is standing-based, never root-author.
- **Halt at S2.** A member's halt pauses **the halter's own landings and delegates** at once (their floor). In the space it is an **algedonic signal**: it opens a bounded deliberation window under the rulebook, decided by k-of-n over independent lineages with no species requirement. A whole-space pause is a decision of that window, never one member's act. Un-pausing the space is any non-author Steward act under the rulebook.
- **Capabilities, full form:**
  - One device-key act per device per ≤ 30 days authorises a delegation key.
  - Lane caps `{actor, session, lane, resources, kinds, override, expires, parent}` are signed by the delegation key, and records carry `bounded-by:<cap>`.
  - Effective permission = standing in the landing space ∩ role ceiling.
  - Expiry is revocation.
  - Rollout: observe, then enforce fleet resources, then the rest.
  - The **unhalt and pre-expiry fleet override** acts move from hook-stamped clauses to a device-key signature behind a TTY confirmation.
- **brit + rakia as the integration and review plane:**
  - brit-native integrate is the CAS, and the forge is a mirror. Canonical `dev` is a cref with threshold 2 including one non-author.
  - Issues, reviews, threads and proposals are typed collective-space records on the same dataplane replication as code. Git refs or notes are only the degraded carrier, never a third transport. This follows Radicle's [Collaborative Objects](https://radicle.dev/guides/protocol) and git-bug's [operation model](https://raw.githubusercontent.com/git-bug/git-bug/master/doc/design/data-model.md) while keeping conversation, commitment and presence as different record types.
  - **Landing intents, review records (`brit attest`, `Reviewed-By:`) and walkthroughs stay collective-space B records held by members.** They are never on the lamad `Content` valve, whose custody is every content-DHT arc holder and whose head plane is global. Only outcomes that need public reach — a landed release manifest, a ratified lineage — are notarised (valve or mishpat), at a few per release.
  - Cross-member verdicts are recomputed, or confirmed by a different-lineage builder.
  - rakia reach climbs by ceremony.
- **Derived views are pure functions of CIDs.** eprfs folds, rakia indexes and brit attribution take content CIDs plus parent-derived CIDs and produce verifiable derived CIDs. Any peer may compute or cache them without trusting the server that served them; this is [Anthropic Scion's derived-data model](https://www.anthropic.com/engineering/redesigning-source-control).
- **Want-based sparse fetch.** Rakia serves a session the root manifest and then only requested subtrees and blobs. The human's device remains the local-first unit and can hold the full set; an agent session is a sparse client of that device and needs no full clone ([Anthropic Scion](https://www.anthropic.com/engineering/redesigning-source-control)).
- **Standing:**
  - Ledgers are **held per space**; there is no global aggregator.
  - Portability is presented by the holder and consented per space.
  - Trailers carry per-space opaque references.
- **qahal:**
  - Per-author B notes are replicated to members.
  - Threads are derived links.
  - Read receipts stay local.
  - The owed response uses the `respond-to-challenge` shape, which is not notarizable until a coordinator-only action string lands.
- **Agents:** `ElohimAgent` membership at standing 0, counted under their delegator's lineage. ACP makes Codex and Claude equal runtimes.
- **Not promised:** private spaces before MLS-class E2EE.

### 5.2 S3: many collectives and forks
- **Forks.** A fork is a legitimate sibling head (`forks: <cid>`, `MemberKind::Collective`).
- **Reconciliation** is a lodged proposal:
  - the receiver recomputes verdicts;
  - its Stewards accept;
  - records cross by an explicit `import{from_space,cid}` that keeps origin signatures;
  - thread ops carry `previous` anchors.
- **Cross-space records** carry their rulebook CID and are judged under both rulebooks.
- **Versions.** Epochs are negotiated per link at the lowest common epoch. High-class lineages need `migrates-lineage` against that lineage's `CouncilRoster`. The simulation gate begins for high.
- **Coordinator changes** propagate by adoption per collective. Adoption is not consent.
- **Adoption** beyond the grant requires a rehearsed `sunsets-lineage` revert.
- **Warrants** answer invalid lineage Commitments.
- **Tie-breaks** among members use signed-head causality plus the rulebook's commit-reveal rule.

### 5.3 S4: the global commons evolving the protocol
- **Trigger:** `authority-in-integrity` is green for mishpat lineage actions, `constitution-ratified` is wired, and the `CouncilRoster` has grown past 1-of-1.
- **Mechanism:**
  - The commons' rulebook is the constitution manifest at `constitution_root`.
  - **`amends-constitution` arrives with an integrity arm validated against the mishpat `CouncilRoster`**, never coordinator-only.
  - The deliberation window is deterministic from the action timestamps of referenced entries.
  - The simulation gate uses at least 3 independent models with pre-registered stopping rules.
  - The `sunsets-lineage` revert is rehearsed before ratification.
  - The dignity floor and the method are unamendable.
  - Ratification is the same cosignature shape as the S1 nack leaf, lifted to k-of-n over a proposal CID and collected off-chain into one Commitment.

### 5.4 How the channel survives the upgrades it carries

Current packaging carries a nonempty `envelope.wireEpochs`, but the rest is proposed. Current status exposes a release CID plus resolved path/CID, not `head.bins.epr` or head capabilities; current CLI has `epr check`, `epr ready [--target REF] [--deep]`, and `epr peer serve`, not `serve --check`. Drop-epoch refusal, predecessor fallback, all-head writer gating, and a dual-reader landing gate remain to be built.

The decision remains expand → contract: readers expand first, writers move only after measured peer support, settlement depends on fold coverage and nacks rather than global rules equality, and channel-class changes require the dual reader before contraction.

**First scheduled live exercise:** the iroh 0.92 → 1.0 migration remains joint across eprfs-peer and elohim-storage. The member manifest contains pins and a comment; the eprfs patch is actually in `elohim/eprfs/Cargo.toml:40-45`, while storage declares its patch at `elohim/elohim-storage/Cargo.toml:482-488`. Repository documentation establishes the external relay as 1.0.3. The conductor submodule is uninitialized and uses `update = none`, so its resolved iroh dependency is unverified here.

### 5.5 The reflexive constitutional path
1. **Proposal.** An agent claims `lane:<id>`, works on a self-reach head and posts an intent (fruit CIDs only).
2. **Classification** is computed by the base-tree resolver.
3. **Merge ≠ ratification ≠ adoption, enforced by mechanism, not prose:**
   - **Merge to dev at S1:** a high class needs evidence plus a human verdict plus the household v1∥v2 receipt.
   - **Adoption at alpha reach:** the edge adoption hold (A9) keeps the alpha fleet on the old hash until an adoption record lands. At S1 that record needs a human verdict and the household receipt. Beyond the grant (S3+) it also needs a rehearsed `sunsets-lineage` revert.
   - **Ratification:** a `migrates-lineage` Commitment validated in integrity against the `CouncilRoster`. That exists only after step 5.
4. **Routing and propagation by class:**

   | Class | Ratified by | Propagates by | Revert |
   |---|---|---|---|
   | low (coordinator) | 1-of-1 steward (S1); non-author stewards (S2) | hot-swap, no hash move; at alpha reach with the household receipt | re-pin |
   | governance (dev process) | human steward verdict (S1); rulebook amendment (S2+) | landing; judged by base recipe | revert intent (never automatic) |
   | high (integrity) | `migrates-lineage` (once step 5 exists); human verdict + adoption record before | v1∥v2 window; adoption hold | `sunsets-lineage` rehearsed first (S3+) |
   | global | `amends-constitution`, S4 only | — | — |

5. **The training wheels today are hollow in five ways, and one crossing fixes them.**
   - **(a)** `progenitor_pubkey: ~` sits in **four** roles: lamad, imagodei, mishpat and node_registry (`elohim/holochain/dna/elohim/workdir/happ.yaml:33,60,74,88`). DNA properties are part of each DNA hash. Keying all four moves four hashes, including lamad's full-arc corpus (re-sync, RAM ∝ corpus). Each needs a reinstall with new agent keys and a partition risk.
   - **(b)** The quorum is checked only in the coordinator (`commitments.rs:875,1255`). `mishpat_integrity` has no lineage arm.
   - **(c)** `bootstrap_steward()` is exported but never called in `commitments.rs`. Mishpat coordinator validation accepts distinct valid keys without authoritative roster validation, while release adoption separately rejects off-roster signers and counts distinct on-roster signers (`elohim/elohim-storage/src/services/release_adoption/verify.rs:1307-1343`). A keyed progenitor alone does not establish integrity authority.
   - **(d)** k comes from the payload (`required_signatures`, default 1, `:1266`). Only storage clamps it (`verify.rs:1226`).
   - **(e)** The exercised roster today is an author-mintable Mishpat `Commitment` (`author-lens`) read through Mishpat; there is no `CouncilRoster` integrity entry (`mishpat_integrity/src/lib.rs:309-321`; `happ-lineage-migration.steps.ts:2330-2348`; `release_adoption/path_evidence.rs:329-343`). **Moving the coordinator check into integrity unchanged would still count keys rather than authoritative members.**

   **Rule (the mishpat lineage-authority crossing). One mishpat DNA-hash move, shipped atomically:**
   1. **Key `progenitor_pubkey` in mishpat's properties only.** lamad, imagodei and node_registry stay `~` until their own planned lineage crossings. The hash move is confined to mishpat. `hc-mesh-bundle-identity.test.sh:88` is updated per role.
   2. **Add a `CouncilRoster` entry type to `mishpat_integrity`.** It holds `{members[], threshold k, predecessor}`, hash-chained. Its validation requires the genesis roster to be signed by the mishpat progenitor (1-of-1) and every successor to carry k-of-n signatures of its predecessor's members. The lamad `council-roster` Content becomes a projection or mirror, never the authority. *Rejected alternative:* pinning the roster root as a DNA property would make every roster change a hash move.
   3. **Add integrity arms for `migrates-lineage` and `sunsets-lineage`** (and later `amends-constitution`):
      - `must_get` the `CouncilRoster` at `roster_cid`;
      - **k = that roster's threshold, never the payload's `required_signatures`** (a mismatch is invalid);
      - every signer must be a distinct roster member;
      - signatures must verify over `signing_payload_cid`.
   4. **Ratification of this crossing.** It is accepted at S1 by the human steward's verdict plus the household v1∥v2 receipt, and adopted on alpha through the adoption hold. It is labelled **the last 1-of-1 act outside the substrate**. That is legitimate because it narrows: it enforces in integrity what the coordinator already claimed.
   5. **Until it lands:** a lineage commitment is labelled *"k signatures, membership unchecked"* in every view and **counts as no quorum**. No ratification in the k-of-n sense exists. Other integrity changes proceed only as S1 human-verdict merges held from adoption by the edge hold, and none is presented as ratified.
   6. **Cost, stated plainly.** It costs one mishpat hash move. `ALLOW_DNA_REINSTALL` alone is insufficient on an app holding cells; the crossing needs `DNA_MIGRATION_INTENT` covering the new Mishpat bundle hash, or destructive `FORCE_DNA_REINSTALL=wipe` (`elohim/elohim-storage/src/happ_manager.rs:24-38,133-146`). It still applies to both alpha genesis peers together and may instead ride a v1∥v2 window. The §9 milestone fork remains Mishpat-only versus batching all four roles.
6. **Council rule for every rung.** **k counts independent lineages, never species.**
   - An agent's lineage is its delegator's until earned standing establishes otherwise under the rulebook.
   - No class is decided by a single lineage above S1.
   - A real council (≥ 2-of-3) needs at least two lineages independent of the proposer's, plus a non-sibling builder lineage for verdicts.
7. **The channel's own rules.** A rulebook amendment is governance class, ratified at the space's reach. Wire changes name their epochs and go expand → contract.

### 5.6 Agent standing: delegates now, earned later
- **Now:** agents are delegates. They propose, prove, simulate, witness and may take halt on their own grant. They are actors of record without authority, acting under hook-stamped clauses (S1) or lane caps (S2).
- **Earned:** a derived ledger **per space** per persona lineage (role + `definition_cid`), never derived from a key or a model label.
  - Fulfilled: landed and not reverted; independently confirmed; owed responses answered.
  - Breached: cap refusals, contested overrides, reverts, quarantines caused, nacked claims.
- **Model swaps:** a new persona version. Carry-over happens only through a discounted, steward-co-signed succession record.
- **What standing buys:**
  - **S1:** a wider ceiling (class-0 CAS, attenuation), granted as a rulebook widening.
  - **S2:** reviewer eligibility for the low class.
  - **S3/S4:** roster eligibility through the epic's earned ceiling — never a sole signer, a majority, or a widener of its own power.

### 5.7 Rakia: the system under its brand

Rakia names the whole system: repository (brit refs and attestations), registry (manifest entries over CIDs, with channel heads as declared dependencies), build cache (attested closures → artifacts), CI/CD (stages forge stars; verdicts are weighted by independence and standing), and experience. Stars are artifacts: content-addressed blobs on the dataplane. Their dependency graph is the constellation.

brit does not replace git transport. Git over p2p and fork governance stay off the critical path. brit owns the social and evidence layer around commits—proposals, reviews, attestations and walkthroughs—on the signed, content-addressed note store CI already trusts. Radicle shows social objects beside code ([source](https://radicle.dev/guides/protocol)); git-appraise shows reviews and CI in namespaced notes ([source](https://github.com/google/git-appraise)).

The field has split three ways: agent-scale centralisation ([Anthropic Scion](https://www.anthropic.com/engineering/redesigning-source-control), [GitLab](https://about.gitlab.com/blog/scaling-git-for-agentic-software-engineering/), [Garden](https://docs.gardenscm.ai/) and [code.storage](https://code.storage/docs/getting-started/introduction)); social objects replicated beside code ([Radicle](https://radicle.dev/guides/protocol), [git-bug](https://raw.githubusercontent.com/git-bug/git-bug/master/doc/design/data-model.md), [jj](https://docs.jj-vcs.dev/latest/technical/concurrency/), [Willow](https://willowprotocol.org/) and [Keyhive](https://www.inkandswitch.com/keyhive/notebook/)); and agent provenance as a git sidecar ([Entire](https://docs.entire.com/guides/sessions/overview) and GitLab agent governance). This design composes all three under governance: sparse agent-scale reads, peer-replicated typed objects and brit evidence linked to commits. The scans found no other design doing all three. Entire's transcript capture and GitLab's reasoning visibility remain rejected: thoughts are private; fruit is judged.

The experience is an asked-for, perspective-relative view: a Cybersyn control room showing algedonic signals such as breached bounds, a stalled queue head or a halt. It renders what the primitives already make true and owns no data. It shows your own grant in full, other grants only as their rulebooks allow, and siblings as one constellation, never as independent witnesses. No lever bypasses the holders except halting your own grant.

Value flows back to every steward. Every forge, attest, review, use and custody act is an REA event. The aggregate sky is visible to all—**"everyone gets to look at the sky"**—while per-object detail requires reach or relationship: **"not having a telescope you might not see the intricacies of any particular object (anonymity in aggregate), but they're accounted for"**. This is SDO/RWA boundary 6: participation detail remains sensitive even when the aggregate is visible. Anyone with compute in a community can forge at S2/S3, inheriting the EPR primitives for governance, story, VSM, accountability and REA.

**Guidestar (TBD, not designed):** Inbound value, including a donation, is not enclosed by the steward. It honestly initializes the commons pools and follows contribution edges to nascent contributor presences—dependencies, upstream projects and inspirations—where it can be claimed; those presences remain unclaimed or stewarded until then. The operator names the reason: **"that honesty is our network attractor, and is a part of our epic"**. The only near-term requirement is that artifacts and attestations carry their upstream contribution edges.

---

## 6. The forward-looking arm's standing home
**No new instrument.** Extend the packaged `mem-horizon-scan` skill with `domain: scm-coordination`.

“Scion (Anthropic)” is Anthropic's internal source-control system, confirmed by Anthropic's 2026-10-13 engineering post, [“Redesigning source control at Anthropic”](https://www.anthropic.com/engineering/redesigning-source-control). [Google Cloud's Scion](https://github.com/GoogleCloudPlatform/scion) is unrelated: an agent hypervisor with a hub and messaging plane, and the closest prior art here to `berth say/inbox` ([docs](https://googlecloudplatform.github.io/scion/)).

**Sources:** `genesis/agentic/horizon/scm-coordination-sources.md`, append-only. Seeds:
- Scion;
- Garden SCM, code.storage, Entire;
- GitLab trains, GitHub merge queue, Mergify, Graphite;
- Radicle, Tangled, jj, Pijul, git-bug;
- Willow / Earthstar, iroh / iroh-docs, p2panda;
- Ink & Switch, automerge / samod;
- Fossil;
- RFC 9420, arewemlsyet;
- Nostr NIPs 17 / 29 / 59;
- Holochain dev pulses;
- A2A, MCP Tasks, ACP;
- Claude Code docs;
- SLSA / in-toto / Sigstore;
- REAPI / attic / Turborepo.

**Output:** `genesis/docs/analysis/horizon-scans/<date>-scm.md`, ending in decision-delta rows. Two cycles with no delta halve the cadence.

**Owner:** human:matthew. Execution is delegated to `agent:horizon-scanner@<model>`, and adopt/reject flips need a steward act.

**Cadence:** quarterly, plus event triggers:
- an iroh release past our pin;
- countersigning or warrants going stable;
- an MLS/DMLS or Keyhive beta;
- a Radicle major release;
- a Scion post.

**retire-when:** every adopted primitive sits behind a seam with its own upstream watch, or the S4 commons runs its own horizon function.

---

## 7. NOT building
- A new transport or replication substrate. No iroh-docs or p2panda adoption now.
- A merge-queue server, bot or forge-hosted queue state.
- A Jenkins plugin, lease reads or inline heredocs in any Jenkinsfile. The adoption hold lives in `scripts/ci/`.
- Speculative parallel trains, bisection on Jenkins, or culprit search with evidence reuse.
- An `agent:ci-runner-*` persona, or an LLM witness session.
- Device-key signatures per act at S1, or agent keys at S1.
- A git-notes byte carrier or sealed second byte carrier, or home-grown group encryption. `refs/notes/brit/proposal` carries social and evidence records, not offered code bytes.
- ark `RevertToPreviousClosure`.
- qahal entry or link types; extension of `Discussion`; per-message notarization; landing intents or review records on the lamad `Content` valve.
- `amends-constitution` as a coordinator-only action; any lineage "quorum" validated only by count of keys; keying `progenitor_pubkey` in roles other than mishpat as part of this plan.
- Countersigning, at any rung.
- The channel on kitsune2 or the conductor.
- Transcript or session sharing, or any session commitment on the forge.
- Rules-digest equality as a settlement condition.
- Growth of berth's vocabulary beyond moor/lease/shed (plus withdraw/halt/unhalt as commitment kinds), or an inbox inside ram-guard.py.
- Agent-to-agent wake, ark auto-spawn of headless sessions, or a vendor-relay wake.
- Counting sibling devices as independent witnesses.
- Widening matthew's roster to other humans; re-keying the v1 topic.
- A new register, ranking script, ledger or horizon instrument.
- An active habit slot without the §3 milestone decision.
- Raising head or node caps as a fix.
- A PreToolUse network-blocking halt.
- A halt that binds anyone beyond the halter's own grant without a deliberation window.

---

## 8. Corrections to record in canon

Each correction is recorded through the governed flow; managed citations use `epr flow cites`.

1. Notes are Ephemeral (C) in implementation and the berth design (`genesis/agentic/bin/berth:20`; `2026-09-03-workspace-berth-carrying-capacity-design.md:86,92`).
2. In `agentic-developer`, stale `HUSKY=0` guidance is at `.codex/skills/agentic-developer/SKILL.md:222`, not line 219. The claimed last-eight-run 2.4–4.7 h range has no repository evidence (unverified).
3. brit, rakia, holochain-conductor, and che-devworkspaces are uninitialized (`-`), so branch and behind counts cannot be asserted. Since `a230c83a7`, pre-push immediately refuses missing brit, rakia, or sophia (`.husky/pre-push.bash:41-52`).
4. `progenitor_pubkey: ~` is a placeholder in four roles. In node-registry, `node_registry_integrity/src/lib.rs:383` is only a comment; `node_registry_coordinator/src/bootstrap_steward.rs:47-71` reads the property.
5. Mishpat quorum caveats remain: no integrity arm, unchecked coordinator signers, payload-controlled k, and no authoritative integrity roster.
6. The receive-side allow-list exists at `fold.rs:308`; ack/fold and per-leaf failure behavior are the defects under repair.
7. Current participant standing is attribution, “not a gate” (`elohim/epr-rea/src/participants.rs:14-17`). Space admission and S2 caps remain proposed.
8. “Scion (Anthropic)” is Anthropic's internal source-control system, confirmed by Anthropic's 2026-10-13 engineering post, [“Redesigning source control at Anthropic”](https://www.anthropic.com/engineering/redesigning-source-control). [Google Cloud's Scion](https://github.com/GoogleCloudPlatform/scion) is an unrelated agent hypervisor with a messaging plane, the closest prior art here to `berth say/inbox`.
9. Pre-push target/message collection is `.husky/pre-push.bash:238-253`; tag-derived `alpha-fleet` and `jenkins-dispatch` are at `:260-267`.
10. The Python resolver accepts both `bafkrei…` and `bafyrei…` since `6c0c4f7e7`.
11. No committed `sprint-lands-each-closure-once` habit exists. The active habits are `dataplane-convergence` and `push-delivers-within-budget`; 57 are declared.
12. The delivery-cost hard bound remains one pipeline-hour, but no repository-backed 3.21-hour current reading exists. The latest tracked series is 9/10 delivered with p90 179.6 minutes; the habit remains red for documented checks.
13. There is no packaged `space-inbox-notice` and no PostToolUse `AskUserQuestion` human-presence hook today.
14. The running-daemon release CID was an operator live observation without a checked-in `epr bin status --json` receipt (unverified). Repository-backed resolver and bootstrap behavior is cited in A1.
15. The iroh migration is joint across eprfs and storage; external relay 1.0.3 is documented, while the uninitialized conductor's resolved dependency is unverified.
16. The steward-space rename remains the naming decision; “release channel” keeps its existing meaning.

## 9. Milestone decisions

These forks are not parked for the operator. Per the operator ruling of 2026-10-27, each is resolved in flight at its milestone by a decision agent grounded in the manifesto and canon. Its record includes alternatives, evidence, and a falsifier, and the implementation sits behind a loosely coupled interface so reversal is a data or implementation swap. Only irreversible or outward-facing acts—a fleet DNA-hash move, forge credential changes, or deploys beyond the household—go to the operator, one at a time.

1. **§3 / habit milestone — WIP fence:** whether to mint `sprint-lands-each-closure-once` inactive upstream of `push-delivers-within-budget`, or swap it into an active slot.
2. **A5 — hook write-set:** `_observation.py` attribution, the `ram-guard.py:379` principal, the new inbox-notice package, and the proposed response-reading human-presence hook.
3. **A7 — carrier (resolved: D3):** `bundle:<cid>` over the tier is primary; `refs/offers/<lane>` is the peer-link-down fallback; `feat/<lane>` is never used.
4. **A4 — relay topology:** steward-owned relay versus direct plus alpha; whether n0 public relays are a last fallback. Alpha is never the only path.
5. **§5.5 — Mishpat authority:** Mishpat-only progenitor key plus `CouncilRoster` integrity entry and roster-derived k, or all four roles in one rehearsed v1∥v2 window; standalone timing versus the next Mishpat crossing. The fleet DNA-hash move goes to the operator.
6. **A10 — class-0 auto-revert:** retain or narrow the locally reproduced, one-per-sprint rule.
7. **S2/S4 — council doctrine:** thresholds and windows while preserving independent-lineage counting.
8. **§5.6 — standing succession:** carry-over discount for a persona model swap.
9. **A13 — conversation retention:** proposed 14-day tombstone collection with the worst-holder assumption.
10. **Operations milestone — revoked-device credentials:** operator procedure; any forge credential change goes to the operator.
11. **Canon milestone — naming and horizon arm:** steward-space rename, owner, and cadence.
12. **A9 — governance class:** membership, base-recipe judgment, and one batched human verdict per sprint.
13. **A4 — daemon start:** postStart background launcher versus SessionStart-only.
14. **A9 — adoption hold:** interface and composition with migration-intent/release-adoption machinery; low-class alpha reach and receipt pricing. Any deploy beyond the household goes to the operator.

## Decision log

- **D1 (fork 1, §3 milestone, 2026-10-27): mint `sprint-lands-each-closure-once` red and inactive, upstream of `push-delivers-within-budget`; displace nothing.**
  - **Alternatives:**
    - Swap it into the slot held by `dataplane-convergence` for the sprint window.
    - Leave it unminted until A0 reports.
  - **Evidence that chose it:**
    - CLAUDE.md: "a plan, spec, or sprint that serves no habit belongs in `held/`", so the spec needs a declared habit to stay live.
    - The WIP fence is "max 2 active", and both slots carry active reds with their own owners.
    - A9/A10 move `push-delivers-within-budget`'s measured ratio, so the work is already licensed as that red's upstream station.
  - **Falsifier:** a sprint in which A0–A8 land and `push-delivers-within-budget` does not move would show the station is not upstream. Then the habit needs its own slot.
  - **Interface:** activation is one frontmatter flip in the atom (`genesis/orchestrator/.epr-meta/sprint-lands-each-closure-once.habit.md`).
- **D2 (fork 11, canon milestone, 2026-10-27): rename "steward channel" to "steward space" in new canon. "Release channel" keeps its meaning. The forward-looking arm is a `scm-coordination` domain of the packaged `mem-horizon-scan` skill, run quarterly and on events, owned by human:matthew.**
  - **Alternatives:**
    - Keep "channel" for both.
    - Give the horizon arm a new instrument and register.
  - **Evidence that chose it:**
    - Challenger H9: "channel" names a headed release channel and an unheaded membership space, and fusing them invites conversation onto a head plane.
    - CLAUDE.md warns that a new register or ranking script "is almost certainly one of the four above", which points at extending an existing skill rather than minting an instrument.
  - **Falsifier:** an existing canon document that uses "steward space" for something else.
    - A grep of `genesis/docs` on 2026-10-27 found one use: "separately admitted steward spaces" at the planetary rung of `2026-10-16-commons-reach-spaces-physical-holon-design.md:154`.
    - That use means a membership-admitted space held by stewards, the same concept class as this one, so the falsifier does not fire.
  - **Interface:** older documents keep "steward channel" as a synonym until their next edit, so nothing is renamed in bulk. The horizon domain is a sources file plus a skill mode, so dropping it is one deletion.
- **D3 (fork 3, A7 carrier, 2026-10-27): offers travel as a Git bundle written into the steward space's own eprfs tree and fetched over the tier. A7b is primary; `refs/offers/<lane>` on origin is fallback only while the peer link is down; `feat/<lane>` branches are never used.**
  - **Alternatives:**
    - `feat/<lane>` on origin.
    - `refs/offers/<lane>` as the primary carrier.
  - **Evidence that chose it:**
    - A0 probe 6 (2026-10-27, Jenkins `getJobs`) found that `elohim-orchestrator` indexes only `dev` and `PR-*`, while downstream multibranch jobs such as `elohim-eprfs` discover and build `feat/*`, `fix/*`, `claude/*` and `dispatch/*`. A `feat/` offer therefore spends CI per offer.
    - The operator describes GitHub branches, PRs and Jenkins branches as **"BIG friction"** that does not move toward native P2P CI/CD.
  - **Falsifier:** the bundle path cannot carry a 40 h sprint's offers within the tier's limits—leaf size and store growth. Then `refs/offers/<lane>` becomes primary.
  - **Interface:** carrier is a field of the offer record: `carrier: bundle:<cid> | ref:refs/offers/<lane>`. Switching is data.
- **D4 (A0 probe 1, A8 gate, 2026-10-27): A8 proceeds. Sibling devices compute identical closure keys for a clean commit, so a sibling's green verdict names the same inputs. The only remaining barrier is the self-signer check in `reusableHit` (`genesis/orchestrator/gate-evidence.mjs`), which A8's `binds-signer` roster row replaces.**
  - **Alternatives:**
    - Hold A8 until closure keys are printed by `gate-runner` itself rather than a probe script.
    - Drop sibling reuse and let each device gate every closure.
  - **Evidence that chose it:**
    - Ethosengine and shem each computed the six `gate_reuse` closure keys at `c827533ab` in fresh detached worktrees with brit, rakia and sophia checked out. All six matched to 16 hex: elohim-storage `21e704c38ac63719`, doorway `81ca83d8f1374e93`, steward-node `20d11c751837eece`, elohim-epr `1e5b86ad0a9a1219`, eprfs `d020d86776011b2a`, orchestrator `726df8d68401e390`.
    - Shem wrote its probe from the described method; no code crossed in a note, so the match is not one script agreeing with itself.
  - **Caveats the probe found:** a full 40-hex SHA as the commit fails every cargo fingerprint, while `HEAD` in a clean worktree works; a dirty checkout reads `source closure dirty`. Both belong in a future `gate-runner --closure` print mode.
  - **Falsifier:** any later probe or A8 reuse where the two devices disagree on a key at the same clean commit. Reuse then refuses for that project until the input that differs is declared.
  - **Interface:** reuse stays per project behind `pool-policy.json gate_reuse`, and every reused verdict is labelled `independence: sibling` and never counted in k-of-n, so withdrawing it is a policy edit.

## Appendix: minor findings and dispositions
| Finding | Disposition |
|---|---|
| A4 `/opt` premise stale; `EPR_PEER_RELAY_URL` rename | Corrected: runtime path is unverified without a receipt; the rename keeps a one-release compatibility read |
| iroh migration is joint with elohim-storage; conductor dependency unverified | Corrected in §5.4; external relay 1.0.3 is documented, but the uninitialized conductor lock cannot be checked |
| Probe 4 answerable now; A2 idle wording | Fixed: probe closed; nack leaf moved into A2; idle reworded to "skip connect + `folded_from`" |
| No `device-runtime` gate project | Fixed: `just gate eprfs` |
| Uninitialized submodules (brit, rakia, conductor, che-devworkspaces) | Corrected in §8; early refusal landed for brit/rakia/sophia |
| CID tie-break grindable | Bounded: free fields excluded; stated as trustful-self at S1; S2 uses signed-head causality plus commit-reveal (A9, §5.2) |
| Retention self-contradiction / unenforceable pruning | Fixed: §1.3 honest wording; milestone fork 9 reworded |
| Standing ledger and trailers enable cross-holon join | Fixed: ledgers per space, holder-presented portability, per-space opaque trailer references (A9, §5.1, §5.6) |
| Human-only authority claim forgeable at S1 | Fixed: hook-stamped clause; cooperative until shipped, stated in A5 and the threat model. Provider-principal attribution is separate. |
| Resubscribe limit plus governor can strand the daemon | Fixed: unbounded in-process backoff; exit only on local faults (A2) |
| ci-witness MCP vs no-headless-sessions; unmetered tokens | A10 separates current `ci-harvest.py` from the proposed non-LLM witness and retains the declared budget |
| Branch carriers spend Jenkins and forge reach | Fixed by D3: eprfs bundle is primary; `refs/offers/<lane>` is fallback only while the peer link is down; `feat/<lane>` is never used |
