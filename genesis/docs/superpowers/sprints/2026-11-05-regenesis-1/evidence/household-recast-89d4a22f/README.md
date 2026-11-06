---
id: rg1-shem-household-recast-89d4a22f
title: "shem household recast on dev 89d4a22ff — second-device validation of R1 custody over iroh"
---

# shem household recast on dev 89d4a22ff — second-device validation of R1

Device shem, session `617b9272-adc3-4226-9bd4-cec1ac027767`, for orchestrator `184beb01`.
Steward of record: Matthew. Recast authorized by the orchestrator's 89d4a22ff landing note.

## What ran

- Main tree fast-forwarded to dev `89d4a22ff` (carries the node-registry `MAX_SHARD_INDEX = 65_535` ceiling).
- Rebuilt from that tree: `just mesh build storage` and `just mesh build doorway` (mesh-bin), `just build` in the
  lamad, imagodei, infrastructure and node-registry DNAs, and sophia UMD + elohim-app + lamad dist.
- `MESH_RESET=1 just mesh start` → ready in 344 s; `just mesh prologue` → every MUST-SUCCEED leg green.
- Three lanes, each `BERTH_CLASS=verify BERTH_TTL=3600 just test mesh <scope>`:

| Lane | Result | Report |
|---|---|---|
| `features/resilience/chaos-peer-churn.feature` | 3 scenarios: 2 passed, **1 failed** (`:112` at step `household-chaos.steps.ts:983`) | `sprint-report-household-20261106T210339Z-89d4a22f` |
| `@concern:blob-durability and not @wip and not @browser-only` | 14 scenarios: 7 passed, **1 failed** (the same chaos `:112`), 3 pending (grandma-photos `:20`, `:32`, `:83`), 3 skipped | `sprint-report-household-20261106T210716Z-89d4a22f` |
| `features/dataplane/epr-app-deliverability.feature` | **7/7 passed** (135 steps) | `sprint-report-household-20261106T211118Z-89d4a22f` |

Per-peer `elohim_custody_announce_total` after the prologue and after each lane: `custody-announce-metrics.txt`.
After the prologue: `refused_binding`, `refused_signature` and `refused_structural` = 0 on all three peers;
matthew sent 2 / received 64 / applied 10, jessica sent 15 / received 38 / applied 18, james sent 17 /
received 34 / applied 17. The chaos lane restarts storage peers, so later snapshots are per-process
counters since that restart, not cumulative.

## Reading

**R1 holds on a second device.** No custody announcement is refused on any peer, and remote custody now
reaches `shard_locations`. The chaos fold moved from 1 copy (6a31ed270) to 2.

**ethosengine's open observation (jessica received 0) is topology, not a defect.** On shem, jessica received
38 announcements after the prologue and applied 18.

**The remaining chaos red is a different gap: the uploader's own copy is not counted when placement picks
another peer.** The failing step is `household-chaos.steps.ts:983`. It reports that the fold on matthew counts
2 of 3, with the message "member agent keys not bound on the fold peer". That message is misleading. On
matthew, all three agent keys are bound to `household-dowell`.

| View | holders of `bafkr4id5…gvoxqmai` (chaos-ladder, 1 chunk, no stripes) |
|---|---|
| matthew | james `announced`, jessica `peer-announced`. **No matthew row** |
| jessica | jessica `self-held`, james `peer-announced` |
| james | james `self-held`, jessica `peer-announced` |

All three peers serve the bytes (`GET /blob/<cid>` → 200, 362 B on :8090, :8091 and :8092). The logs show why
matthew is missing:

- jessica and james each logged "Shard recorded as self-held" and "self-stewardship: active custody-blob
  commitment ensured".
- matthew logged "Shard distributed … peer uhCAkLGE… (james)" and pushed its single custody unit to james. In
  this run's logs it did not record or announce its own local copy.

The cause is in `elohim-storage/src/p2p/mod.rs` (distribution, around lines 2420–2660):

- `self_placement_recorded` and the custody-blob commitment are written only when `PeerSelection` picks this node.
- `PeerSelection` (`services/peer_selection.rs:319`) sorts candidates by `agent_cid` and takes the first
  candidate in each unseen household.
- For a one-unit blob in a single-household mesh, the unit therefore goes to the lexicographically smallest
  agent key.
- On this recast that key is james (`uhCAkL…` < `uhCAkZ…` matthew < `uhCAku…` jessica).

Agent keys are fresh on every recast. So 3/3 on ethosengine and 2/3 here is the same code under a different
key order. The test is order-dependent: it passes only when each uploader happens to be the first candidate
in its own view.

One possible fix (for the storage owner's ruling, not applied here): an uploader that holds the full blob
locally could record and announce its verified self-held copy, and pledge custody, whether or not placement
selected it. That is the same verify-before-record gate the self-selected arm already uses. The other option
is to keep placement as it is and have the fixture not require the uploader's copy.

Separately, the step-983 message should not claim unbound member keys without checking the bindings.

## Logs (on shem, not committed)

`genesis/local-dev/regenesis-1-shem/`: `recast-89d4.sh`, `recast-89d4-{dna,mesh,app,start,wait,prologue}.log`,
`lanes-89d4.sh`, `lanes-89d4.log`, `lane-89d4-{chaos,durability,deliverability}.log`, `custody-89d4.txt`.
