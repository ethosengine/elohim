---
title: "Gradient sprint 2026-10-08 — the performance index's open items, read through trust, cut into packets"
id: gradient-sprint-20261008
date: 2026-10-08
status: in-flight
author: "claude-fable-5-1 (orchestrator), plan reviewed by claude-opus-5-5, at the operator's direction"
habits: [dataplane-convergence, idle-is-free, zome-call-cost-bounded, runtime-performance]
cites:
  - genesis/data/timeline/backlog/performance-concern-index.md
  - "shem-performance-deep-dive-report-20261002 | the measured findings this sprint turns into packets; its operator decisions are taken in D0 | sha256:df11f4c346d960e2 | path: genesis/docs/superpowers/sprints/2026-10-02-shem-performance-deep-dive-report.md"
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
  - genesis/data/timeline/backlog/feedback-discovery-sweep-is-o-n-in-history.md
  - genesis/data/timeline/backlog/head-authority-carried-with-content-sync-unit.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - "conductor-store-growth-report | section 7.1 classes the peer-health observation as ephemeral, the placement Lane 3 implements | sha256:a9ad5a0b8e1b12dc | path: genesis/docs/content/elohim-protocol/architecture/2026-09-24-conductor-store-growth-report.md"
  - "native-delivery-sprint-plan | lane H2 decided samples leave the DHT and only transitions are notarized; Lane 3 is its first half | sha256:583c586a70649e89 | path: genesis/docs/superpowers/plans/2026-09-24-native-delivery-sprint-plan.md"
  - "commons-pool-as-collective-party-design | Lane 5 runs the design gate on this spec and adds the steward-to-custodian question | sha256:23c5312f29bb4e08 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
---

# Gradient sprint 2026-10-08

This is a plan. Results go to each habit's own atom as a DELTA line, never here.

## What the reading found, and what changed since the index

The index (2026-10-02 snapshot) is read through the gradient's eight questions. Four of its entries moved on
dev before this sprint began and the plan starts from the corrected state:

- The cap-grant scan cure is on the fleet and settled (2026-10-04, `hc_db_cap_grant_unkeyed` 0). What is still
  owed is a fleet zome-call latency reading that includes authorisation.
- Option E shipped (fork `901b02607`, edge #1529, 2026-10-03): publish time fell 14–62% on five of six peers,
  conductor CPU did not move. Nothing to build.
- The sync document already carries `headActionHash`. The dataplane-convergence DELTA 2026-10-08c and the
  feature header comment say it does not; what is missing is the declaration's *ordering* (clock, tier, link),
  and two comparisons key on hash alone. The 21-row incident is a different-head case on the receivers that the
  author never announced because only the tier moved.
- The serving-slug regression scenario lives in `features/delivery/app-bundle-elected-delivery.feature:136-147`.

Lane 1 is on the active habit. Every other lane lands as a DELTA on a non-active red habit and flips nothing.

## Lanes

### Lane 1 — `dataplane-convergence` (active)

**1A. A re-elected head announces, and the receiver re-verifies it through its own conductor.**
Scenario `features/dataplane/federation-version-convergence.feature:184-191` (six undefined steps; the
household satisfies `@requires:owned-substrate`).

- Producer (`elohim/elohim-storage/src/sync/projector.rs`): project ONE key, `headOrdering`, holding a
  serde-serialised `DocHeadOrdering { head, canonicalDeclaredAt, earned, tiebreak }` defined beside
  `CanonicalOrdering` in `db/content_diesel.rs` and convertible to and from `CanonicalElectionWire`, so the
  producer, the receiver and the stamp share one type and `ElectionLink` stays the only tiebreak parser. One
  key is one LWW register, so concurrent writers cannot converge on a tuple no peer held. `tiebreak` is the
  column as stored (the effective tiebreak, not a proof locator); the clock is `canonical_declared_at`.
  `headActionHash` is kept for old peers. The key is not named "election": `HeadElection` already names the
  write-mode enum. REQ-N5 unchanged: the value is a hint that routes attention to the own conductor and is
  never written to a head or anchor column. Producer acceptance includes proof that a tier- or clock-only
  `Refreshed` stamp reaches the projection listener; if it does not, the producer adds that emission, or it
  announces nothing for the incident class.
- Receiver (`services/head_adoption_trigger.rs`, `db/content_diesel.rs`): `should_probe` and `decide` take a
  `LocalDeclaration { head, ordering }` read at `declared_head_with_election` in place of the string tuple,
  and probe when the hash differs (today's rule) or when `CanonicalOrdering::advances`, a method over
  `canonical_move_verdict`, says the carried ordering would move the local election. Equal ordering costs
  nothing. Each distinct carried claim probes once: a memo keyed by id and carried ordering, hard-capped, in
  the shape of the courier refusal memo. The retained-hint pass skips `attestation:%` rows (immutable Creates,
  no election) and counts every enqueue decision under a `source` label (`sync` | `retained`), warning once per
  pass on `dropped_full`. `election_refreshed` is emitted only from `StampOutcome::Refreshed`. The new probe
  arm is registered in `elohim-storage/seam-registry.yaml` as bounded work with the C2 cross-reference, and
  the liveness contract's scope note that excludes same-head refresh is updated. The adopt arm is unchanged:
  the stamp persists ordering from the conductor's answer, never from the document.
- Out of this lane, two backlog rows: the reconcile sweep classes a same-anchor election mismatch as in sync
  (`projection_reconcile.rs:4551-4573`), so the 60 s retained pass is the backstop for this class; and the
  wire ordering types are built by service code from db types, whose future home is a crate-root
  `head_election` module beside `epr_head.rs` (moved only when a second consumer exists).
- Wording: the feature header (`:174-176`) and DELTA 2026-10-08c are corrected to "the head IS projected; the
  ordering is not".
- Done-when: unit tests named in the orchestrator's packet table pass; `just gate elohim-storage` EXIT=0;
  the feature passes twice on the household (both scenarios); DELTA 2026-10-08d names the receipt.

**1B. The served slug follows the elected release.** The cure is on the fleet: edge #1578 rolled it and
doorway-alpha serves the current release (DELTA 2026-10-08d). The second doorway stayed stale because the
peer it reads followed no app-bundle channel, a declaration gap cured on dev by deriving the follow set from
the doorway manifests. Owed: that roll, then `/lamad/version.json` on both doorways naming the same stamp and
`elohim_head_adoption_trigger_total{outcome="held_by_release_channel"}` > 0 on matthew. DELTA.

### Lane 2 — `idle-is-free`: a member that can never enumerate stops being retried

Content-target members must be ActionHashes (the zome's `get_feedback_signal_refs_for_target` takes one); the
`sha256-…`/`bafkrei…` members enter through the steward INSERT, which admits any non-null `dht_anchor_hash`
(a content address in a column whose meaning is an action reference: a kind error). The parse fails locally
before any zome call, so the cost is log volume and a wasted sweep slot, not conductor time. Cure: a
`substr(dht_anchor_hash,1,5)='uhCkk'` prefilter on the INSERT (an admission prefilter, not the type check),
decode at admission with the existing public `conductor_writes::decode_action_hash` and delete the projector's
private parser, refuse a malformed routing key in `admit_notified_signal_with` (admit the act, skip the
target), purge the existing malformed rows once at boot, three tests. No retire state and no new metric: the
sweep slot is a mechanical bound owed by the code, and the habit's probe is a log-line count. The census must
name the writer path that put non-action values into the column before any backlog row names a culprit: the
collectives inventory arm puts its CID into the wire slot, not the column. Done-when: `just gate
elohim-storage` EXIT=0; household storage log shows zero `discovery enumeration failed` lines in a one-hour
window; alpha matthew+adam show zero after the roll (baseline 2,852 / 1,995 per six hours).

### Lane 3 — `zome-call-cost-bounded` + dataplane-borrows §plane-separation: machine bookkeeping leaves the person's cadence

The doorway's peer-health probe, in which a doorway operator observes a sibling doorway every five minutes,
notarizes each observation on the operator's lamad chain as 1 Create + 2 CreateLink (≈72 actions/h/peer idle)
and collides with foreground writes. The placement decision already exists and this lane implements its
first half: the conductor store-growth report (§7.1) classes the observation as ephemeral, "not a notarized
Content node", and the native-delivery plan's lane H2 decided that samples go to a retention table and only
**transitions** are notarized. The bound `dna-actions-per-day-ceiling@1` was sized on that decision. The
operator's preference for the node-registry cell's `attest_health` is not taken this sprint for two reasons:
the doorway has no `NodeRegistration` on that DHT and the roster carries doorway ids, and, decisively, moving
the writer moves the writes without reducing them. So this sprint notarizes **on transition only**, with no
keepalive interval (an unchanged-status write has no reader), counting a transition only after the new status
holds two rounds so a peer at the latency threshold does not flap the chain. The hold lives inside the
existing round-based `ProbeRoster`, pure and tested, with a new doorbell outcome; it is the reset-and-time
half of hysteresis that the algedonic module says it lacks, recorded as a debt row to promote when algedonic
emission is wired. A failed write retries next round. In Viable System terms sibling probing is the System 3*
audit channel; its destination is the observer-signed observation plane, which adds a doorway→storage call
and needs its own gate, not this sprint. Done-when: `just gate doorway-service` EXIT=0; idle household
before/after (conductors stopped, `conductor-store-sniff.py`) read as `dna-actions-per-day@1` for lamad
within `dna-actions-per-day-ceiling@1`, zero `Failed to record health attestation`, retained `dropped_full` = 0.
DELTA on `zome-call-cost-bounded`; the "smallest next measurement" in `arch-dataplane-borrows-backlog.md`
answered.

### Lane 4 — measurements only (`runtime-performance`, `zome-call-cost-bounded`, `dataplane-convergence`)

- **4A** Close the publish-time lock's 90-minute window on adam (Loki, after the next roll); done-when < 100
  lines and a genesis build 25 minutes after the roll reads adam's custody row. Today this is behind the
  operator's dataset fix: adam's and eve's conductor volumes are full (DELTA 2026-10-08e), so the reading
  waits for that move and the line says so until then.
- **4B** Fleet zome-call latency including authorisation, as a difference-in-differences:
  `elohim_conductor_call_duration_ms{zome,fn,class,outcome="ok"}` on matthew and on susan, 24 h before and after
  2026-10-04 12:03Z. Matthew is CFS-throttled, so convergence on susan is not the test; the differential is.
  The habit's entry-type-only check stays not wired; named, not built.
- **4C** Heap per hosted agent on the household: direct-launch mesh, `awk` over `/proc/<pid>/smaps` summing
  anonymous bytes and mapping count by permission, at baseline, after two successive hosted installs, and
  after an idle hour. Done-when the executable share and count of the per-agent delta are numbers. Recorded in
  a new backlog atom; extending `SmapsAnonBreakdown` for the fleet is the follow-on.
- **4D** Option E: recorded as shipped; residual idle CPU on a patched conductor remains the open question.

### Lane 5 — commons pool: design gate only

`p2p-design-gate` on `2026-10-04-commons-pool-as-collective-party-design.md` with the gradient's design-time
questions (reach of bytes vs references, custody holder set + threshold + how independence is observed,
freshness, linkability, cost bearer) and the operator's agency-curve question in the vocabulary that already
exists in imagodei: steward, subject, the guardian tier of `STEWARD_CAPABILITY_TIERS`, authority basis,
`StewardshipGrant` as the scoped grant, `StewardshipAppeal`, and `ActivityLog` with `subject_can_view`;
`Collective` plus `Membership` plus epr-rea `Scopes` for the pool as a party. The words "custodian" and
"ward" are not coined for the human role: custodian already names artifact custody, a fold-shard holder and
the custody-shape tags. The gate question is whether drawing on a pool for a subject rides an existing
`StewardshipGrant` capability scope or needs a new one (an integrity change that moves the DNA hash), and the
spec must name where the subject sees a draw and how they appeal it, so variety is amplified with the subject
rather than attenuated to them. Output: a dated spec section naming the a2o scenario and the habit, and
backlog row 31 updated. No code.

### Lane 6 — index and register upkeep (last)

Re-tag at the source (`conductor-cap-grant-scan` → cured; `storage-sqlite-locked` → cured; `conductor-residual-cpu`
split), add rows for the four new concerns with their gradient tags, re-derive the tally tables with the grep
commands at the top of the index, one dated section. The index stays a snapshot. Six debt rows go to their
matching atoms: the `head_election` module home, peer-health observations to the observation plane, the
hysteresis reset in the algedonic module, same-anchor blindness in the reconcile sweep, the writer of
non-action anchor values once the census names it, and shared head-ordering types to a crate only at a second
consumer. One doc-comment fix in the algedonic module, which still calls its latch "the hysteresis predicate".

## Sequence

Lane 1A producer → receiver (opus-reviewed) → household run → land → operator roll → fleet readings. Lanes 2
and 3 in their own worktrees in parallel, cargo gates serialized, each landing with its DELTA. The household is
one berth: idle baseline first on current dev binaries, then the Lane 1 run, then the after-window, then the
heap read in direct launch. Fleet reads and the pool design gate run off the critical path. Status flips only on
fleet evidence.
