---
id: rg1-shem-household-recast-selfcopy-a24504bb
title: "shem household recast for the uploader self-copy fix (a24504bb on dev 78e0ba08c)"
---

# shem household recast: uploader self-copy fix (a24504bb)

Device shem, session `617b9272-adc3-4226-9bd4-cec1ac027767`, for orchestrator `184beb01`.
Steward of record: Matthew. Work ordered by the storage owner's ruling on the residual
`chaos-peer-churn.feature:112` red (berth note dseq 5142).

## The change

Commit `a24504bb` on `regenesis/shem-self-copy`, branched from dev `78e0ba08c`:

- `elohim-storage/src/p2p/mod.rs`, in `distribute_shards`: after placement, a node whose own pantry
  returns the whole payload with its BLAKE3 recomputed (`BlobStore::get`, C5) records each custody unit
  the pantry holds as `self-held`. It announces that unit as its holder (`AnnounceCustody`) and pledges
  the blob (`ensure_active_self_custody`), whether or not `PeerSelection` picked it.
  - Placement still pushes to the selected holders.
  - The node never dials or pushes to itself (enforced: elohim/elohim-storage/tests/blob/distribute_shards_diversity.rs).
  - `self-held` stays distinct from `announced` and `verified`.
  - Uploader rows do not count toward `distributed`, so placement gaps still describe the selected
    holders alone.
  - Derived stripe shards that the pantry does not hold are not claimed. The blob-level pledge still
    stands for them.
- `tests/blob/distribute_shards_diversity.rs` adds two tests:
  - an unselected uploader with a verified copy records one `self-held` row and one announcement, and
    never pushes (enforced: elohim/elohim-storage/tests/blob/distribute_shards_diversity.rs);
  - without a verified copy it records and announces nothing.
  - Result: 6 passed, 1 ignored (the ignore predates this change).
- `genesis/a2o/steps/mesh/household-chaos.steps.ts`: the custody-baseline step now reports the measured
  shortfall (held copies fewer than expected). It no longer names "member agent keys not bound", which
  it did not check.
- RS-GOV-04 paydown: the `p2p-iroh` `dead_code` allow on `AcquisitionIngestCtx` now carries its reason.

## What ran

1. The storage binary was built from the branch with the mesh recipe's flags (`p2p p2p-iroh`, storage
   RUSTFLAGS, toolchain 1.96.1). It was installed to the household's `mesh-bin/` path.
2. The main tree was fast-forwarded to dev `78e0ba08c`.
3. `MESH_RESET=1 just mesh start`: ready in 345 s.
4. `just mesh prologue`: every MUST-SUCCEED leg was green.
5. Three lanes ran, each as `BERTH_CLASS=verify BERTH_TTL=3600 just test mesh <scope>`. They ran from
   the main tree's a2o, which lacks only the step-message change; that changes failure text, not
   verdicts.

The reports are stamped `78e0ba08`, the main tree's HEAD. The storage they ran against was built from
`a24504bb`.

| Lane | Result | Report |
|---|---|---|
| `features/resilience/chaos-peer-churn.feature` | **3/3 passed** (34 steps) | `sprint-report-household-20261107T030545Z-78e0ba08` |
| `@concern:blob-durability and not @wip and not @browser-only` | 14 scenarios: **8 passed, 0 failed**, 3 pending (grandma-photos `:20`, `:32`, `:83`, unchanged), 3 skipped (held for alpha-cluster capabilities) | `sprint-report-household-20261107T030927Z-78e0ba08` |
| `features/dataplane/epr-app-deliverability.feature` | **7/7 passed** (135 steps) | `sprint-report-household-20261107T031337Z-78e0ba08` |

On the 89d4a22ff recast the chaos lane passed 2 of 3, and durability showed 7 passed and 1 failed (the
same chaos `:112`).

## Custody announcements

`custody-announce-metrics.txt` holds `elohim_custody_announce_total` per peer, after the prologue and
after each lane. `refused_binding`, `refused_signature` and `refused_structural` are 0 on every peer in
every snapshot.

After the prologue (sent / received / applied):

| Peer | Sent | Received | Applied |
|---|---|---|---|
| matthew | 13 | 30 | 15 |
| jessica | 13 | 30 | 15 |
| james | 2 | 52 | 26 |

The chaos lane restarts storage peers, so later snapshots count only since that restart.

## How much the lane pass shows

**What this run shows directly:**

- The new uploader path fires on a live household.
  - matthew and jessica each logged "Uploader copy recorded as self-held (verified full copy; not
    selected, no push)" for six prologue items where placement picked another peer.
  - Those items are `community-garden-club`, `elohim-host-landing`, `elohim-protocol`,
    `foundations-christian-technology`, `lamad-spa` and `manifesto`.
- No peer refused any announcement.
- No lane regressed.

**What this run does not show:** the chaos 3/3 is not, by itself, evidence for the fix.

- On this recast every peer recorded its chaos-ladder copy through the existing path, where placement
  picks the node itself ("Shard recorded as self-held" on matthew, jessica and james).
- This recast's keys sort matthew `uhCAk2…`, then jessica `uhCAk7…`, then james `uhCAk8…`. That order did
  not reproduce the 89d4a22ff failure, in which james sorted first and matthew was unselected.

The discriminating evidence is:

- the unit test `uploader_not_selected_still_records_self_held_and_announces`;
- a rerun on a recast whose key order leaves an uploader unselected for chaos-ladder (for example, the
  orchestrator's device).

## Logs (on shem, not committed)

`genesis/local-dev/regenesis-1-shem/` holds:

- `selfcopy-test.sh` and `selfcopy-test.log`;
- `selfcopy-build.sh` and `selfcopy-build.log`;
- `recast-selfcopy.sh` and `recast-selfcopy-{stop,start,wait,prologue}.log`;
- `lanes-selfcopy.sh`, `lanes-selfcopy.log` and `lane-selfcopy-{chaos,durability,deliverability}.log`;
- `custody-selfcopy.txt`.

The previous mesh binary is kept as `mesh-bin/elohim-storage.pre-selfcopy-89d4`.
