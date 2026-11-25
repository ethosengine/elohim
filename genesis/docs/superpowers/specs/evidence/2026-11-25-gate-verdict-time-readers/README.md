---
title: "Gate-verdict time readers, 2026-11-25"
id: gate-verdict-time-readers-evidence
status: draft
class: process-meta
date: 2026-11-25
context-tier: disclosed
steward: human:matthew
author: "Codex, read-only evidence for the integrator; source inventory at dev c1035eebc"
serves: [push-gate-pays-once]
tags: [gate, verdict, time, evidence]
graduation-trigger: >-
  The steward rules which clock each gate-verdict reader consumes, and this note becomes the ruling's
  appendix or is retired with it.
cites: []
---

# Gate-verdict time readers, 2026-11-25

## Finding

`emitGateVerdict` writes `gate-verdict@1` through `epr flow note` and passes the device's main
checkout as `--root`, not the worktree that ran the gate
(`genesis/orchestrator/gate-evidence.mjs:844-887`). `epr flow note` deliberately derives
`occurred_at` from that root's HEAD author date, resolving Q1 as history-derived and never `now()`
(`elohim/eprfs/epr-cli/src/flow/note.rs:24-32,879-887`); the provenance lookup is `git log -1
--format=%ae%x1f%aI` in that root (`elohim/eprfs/epr-cli/src/flow/mod.rs:2848-2861`).

Observed: the verdict for gated commit `4dc3684f6b06`, authored `2026-11-25T05:07:09Z` and run at
about `05:15Z`, carries `occurredAt=2026-11-24T20:56:01Z`: the author date of unrelated main-checkout
HEAD `f74e8fadd`. The value is therefore neither the gated commit's time nor the observation time;
the dependency on `deviceRoot` is explicit at `genesis/orchestrator/gate-evidence.mjs:520-530,854-855`.

## Readers and decisions

“Order” below means causal/history ordering among verdicts; “age” means elapsed wall-clock time.

| Reader | What it does with verdict time today | Need | Effect of the observed dating | Field under the integrator's lean |
|---|---|---|---|---|
| `epr peer verdicts` | Copies `event.occurred_at` into JSON `at` and the human display (`elohim/eprfs/epr-cli/src/peer.rs:1760-1777,1785-1828,1890-1903`). Its scan preserves sidecar append order and never sorts by `at` (`elohim/eprfs/epr-cli/src/peer.rs:1830-1889`). | Display only; neither order nor age. | **Wrong:** the displayed verdict time names an unrelated checkout HEAD. Decision order remains harmless because it is append order. | `occurred_at` for the gated commit's historical date; `observed_at` only if the report adds run time/age. |
| Sibling reuse | `siblingVerdicts` forwards peer rows, including `at`, without parsing it (`genesis/orchestrator/gate-evidence.mjs:532-545`). `reusableHit` explicitly refuses cross-device time ordering, lets any sibling red veto, and otherwise selects a matching green by row order (`genesis/orchestrator/gate-evidence.mjs:574-621`). `decideReuse` applies those candidates without reading `at` (`genesis/orchestrator/gate-runner.mjs:185-217`). | Neither; existence plus project/closure/commit/success. | **Harmless:** the incorrect time cannot admit or reject reuse. | Neither field. |
| Carry poll | Waits on a local deadline, polls sibling verdicts, and settles on the existence of a boolean `success`; it does not read verdict `at` (`genesis/orchestrator/gate-carry-claim.mjs:14-20,61-79`). | Neither. | **Harmless.** | Neither field. |
| Miss settling | Collects matching `(project, closure)` verdict keys, then removes matching misses; the miss's own `wait_until`, not verdict time, controls offerability (`elohim/eprfs/epr-cli/src/flow/misses.rs:133-223,226-259`). | Neither. | **Harmless:** a matching green or red settles regardless of its date. | Neither field. |
| Live carries | Orders carries by their own `dseq/at_ms/CID`, expires them by their own `valid_until`, and delegates verdict settlement to the key-only miss predicate (`elohim/eprfs/epr-cli/src/flow/carry.rs:433-515,788-805`). `valid_until` is minted from `now + valid_for_s`, not from a verdict (`elohim/eprfs/epr-cli/src/flow/carry.rs:309-313,357-399`). Lander freshness uses receiver-observed peer-head time plus carry deadlines (`elohim/eprfs/epr-cli/src/flow/carry.rs:571-628,1004-1011`). | Carry wall-clock age, but no verdict order or age. | **Harmless:** the verdict date affects neither validity nor freshness. | Neither verdict field; retain carry/head receiver clocks. |
| Attestation enrichment | Resolves the exact verdict CID and copies project/closure/commit/success/seconds, not `occurred_at` (`elohim/eprfs/epr-cli/src/flow/enrich.rs:182-200,273-308`). | Neither. | **Harmless.** | Neither field. |
| Note/context ordering | `notes_across` includes note events and orders them by parsed `occurred_at`, with append position as tie-break (`elohim/eprfs/epr-cli/src/flow/read.rs:192-241,257-284`). A gate verdict is emitted as a repository-scoped Cite/note (`elohim/eprfs/epr-cli/src/flow/note.rs:867-875,976-995`). | Order. | **Wrong:** a repository-scope read can place the verdict beside an unrelated older HEAD. | `occurred_at`; this is historical order, not run age. |
| Ledger report | Includes in-scope notes, copies their `occurred_at`, sorts by it plus record index, and renders its date (`elohim/eprfs/epr-cli/src/flow/ledger.rs:140-184,230-239,301-321`). | Order and historical display. | **Wrong:** the row can be misordered and misdated. | `occurred_at`; optionally show `observed_at` as a separate receipt fact. |
| Participation report | Projects every note's `occurred_at` into testimony/worth `at` without making a time decision (`elohim/eprfs/epr-cli/src/flow/participate.rs:1024-1077`). | Display only. | **Wrong:** displayed `at` is the unrelated HEAD date; report membership is harmless. | `occurred_at`; `observed_at` only for an explicit run-time display. |
| Generic measure folds | Every `run:observation` with measurement slots becomes a Fold carrying `occurred_at` and append `seq` (`elohim/eprfs/epr-cli/src/flow/report.rs:704-725,2021-2080`). Latest/reset readers order by `(occurred_at, seq)` (`elohim/eprfs/epr-cli/src/flow/report.rs:1031-1038,1548-1558`); window and staleness readers compare `occurred_at` to wall clock (`elohim/eprfs/epr-cli/src/flow/report.rs:1726-1762,1913-1944`). Gate verdict is declared as an active observation measure (`.claude/epr-meta/measures.yaml:1461-1476`), while the registry creates report bounds only from `lenses` and watermark-bearing measure policies and no present row selects gate verdict (`elohim/eprfs/epr-cli/src/flow/measures.rs:699-726`). | Order for latest/reset; age for windows/staleness. | **Harmless today, latent:** the fold exists but no gate-verdict report consumes it. If one did, both ordering and wall age would be wrong. | `occurred_at` for latest/reset order; `observed_at` for windows/staleness. |
| Crossing and fold storage | Channel projection walks admitted events in store order and carries their canonical bytes (`elohim/eprfs/eprfs-peer/src/channel.rs:89-143`); receipt validates the event CID and appends it without a time comparison (`elohim/eprfs/eprfs-peer/src/fold.rs:721-790`). | Neither. | **Harmless:** transport does not interpret verdict time. | Neither field. |
| Retention/compaction | Channel-root retention is by installed root order/count, not event age (`elohim/eprfs/eprfs-peer/src/channel.rs:210-269`). The flow sidecar is append-only (`elohim/eprfs/epr-rea/src/store.rs:194-201,303-331`), and interrupted bytes are not silently truncated (`elohim/eprfs/epr-rea/src/sidecar.rs:3-10`). | Neither. | **Harmless:** no stored verdict is expired or compacted from this date. | Neither field. |

## Similar names that are not readers of this field

Local “newest green” and “green older than red” logic orders Brit build attestations by
`attestation.builtAt`, not flow-event `occurred_at`
(`genesis/orchestrator/gate-evidence.mjs:593-600,624-640`;
`genesis/orchestrator/gate-runner.mjs:207-215`). Handover likewise sorts own attestations by
`builtAt` and treats sibling verdicts as an existence check
(`genesis/orchestrator/handover-record.mjs:96-119`). Those wall-clock readers are independent of
the observed bug; they keep `builtAt` (or could use `observed_at` only after an explicit unification).

`gate-cost.mjs` reads `record.occurredAt`, but filters for `gate-cycle` and intentionally uses ledger
append order (`genesis/orchestrator/gate-cost.mjs:32-51,90-99`). `gate-live.mjs`'s `gate-verdict`
records are a separate device-local run-log schema with their own `at`
(`genesis/orchestrator/gate-live.mjs:740-773`). The habit projection only accepts `run:ruling`, not
this `run:observation` (`.claude/scripts/habits-project.py:102-126`), and saga status only consumes
Produce/Dismiss events (`.claude/scripts/saga-status.py:212-253`). None reads the flow verdict's time.

## Options for the steward's ruling

The integrator's lean, recorded without adding a recommendation, is:

1. Keep `occurred_at` history-derived, but derive it from the **gated commit's author date**. That
   makes it device-independent and tied to what was judged. Order/history readers use it: peer
   historical display, notes/context, ledger, participation, and generic latest/reset folds.
2. Add a separate `observed_at` for the gate run observation. Only wall-clock readers use it:
   any future verdict freshness/age/window/staleness reader. Current sibling reuse, carry, misses,
   enrichment, crossing, and retention use neither. Current Brit-attestation readers retain
   `builtAt` unless deliberately migrated.

Changing stored meaning is not an in-place correction. A flow event's canonical fields, including
`occurred_at`, determine its CID (`elohim/eprfs/epr-rea/src/model.rs:1-20,373-404`), and note dedup is
CID equality (`elohim/eprfs/epr-cli/src/flow/note.rs:20-28,1001-1028`). Adding `observed_at` or changing
`occurred_at` therefore mints a new verdict CID; old verdicts and enrichments that name their exact
CIDs remain as written and are not re-keyed (`elohim/eprfs/epr-cli/src/flow/enrich.rs:182-200`).

Legacy and new verdicts can consequently coexist for the same logical gate result. Their
`occurred_at` values have different semantics, so any “newest green” or green-versus-red ordering
introduced over verdict events must distinguish them: recognize the new shape/version, or recover a
legacy historical date from its `env:commit`; it must not compare the two meanings as one clock. A
wall-clock reader must use `observed_at` and treat its absence on a legacy verdict as **unknown**, not
fall back to the legacy `occurred_at`. Readers that ignore time need no distinction, and CID-based
dedup does not collapse old and new records (`elohim/eprfs/epr-cli/src/peer.rs:1785-1828,1830-1889`).
