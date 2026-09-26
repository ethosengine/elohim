---
name: project_household_conductor_roll_recovery
title: "Household conductor roll — carried-record cure, stranded cell, infrastructure BlockSpan"
description: "2026-09-25 mesh proof: after a conductor roll propagate-landing-to-B recovers only with a carried record; jessica's cell can stay STRANDED (CellDisabled, database is locked) so the shared lamad readiness rail refuses; a mutual BlockSpan sits on the infrastructure DNA."
metadata:
  type: project
---

**Seen 2026-09-25 (post-station-4 mesh proof, Opus agent, household-dowell fixture, fork pin 25dd2d0be144):**

- `just mesh storage-restart` after a pool `elohim-storage` rebuild also rolled the three
  conductors. Doorway B's bridge and storage's app-interface tokens were stranded; the first
  `just mesh prologue` failed on 503s and the second passed the whole seed chain ~50 min later
  (the known 20–45 min post-roll cell-not-running window).
- **Carried-record cure.** The prologue's `propagate-landing-to-B` leg 503'd both runs. A
  hash-only `DECLARE_ONLY` declare is refused six times (`declare_canonical_head: target action
  … is not retrievable`) because jessica's cell cannot gossip-fetch matthew's action. The same
  declare with `SOURCE_DOORWAY_URL=http://localhost:8888` supplying the 5120-char head record
  from doorway A answered `✓ canonical head propagated` on attempt 1: the zome verifies the
  carried record in wasm without gossip. The leg degrades silently to hash-only when its
  `curl -fSs` head-record fetch fails while doorway A is mid-`catching-up`; that degradation,
  not the declare, is what makes the leg unrecoverable.
- **Stranded cell.** jessica logged `STRANDED: this role's cell is ABSENT from the conductor's
  running-cell map while its app's persisted status IS Enabled. enable_app … CANNOT lift this`;
  seeding through doorway B failed `CellDisabled(…)`; ~90 min later her `head-record` went 404 →
  30 s timeout under sustained `apply_delta: database is locked`. She never authored a head (the
  prologue's B legs point at matthew's action), so `just test mesh`'s shared lamad readiness
  rail (`jessica:8091 … head-record`) cannot pass; scoring runs used `MESH_ALLOW_NO_PROLOGUE=1`
  and measured doorway alpha. Cure is a conductor recycle (`just mesh conductors-restart`) or
  `MESH_RESET=1`; the cell does not self-heal.
- **infrastructure-DNA BlockSpan.** `GET /db/p2p/conductor-diagnostics` showed
  `blocked_message_counts` on DNA `uhC0kYVFIpz1CIpaXG_Yo…` = `infrastructure.dna` (matthew 85
  in, jessica 38 in / 18 out). `lamad` is unblocked, which is why content search works.
  `hc-mesh.sh blocks` needs `hc-dbtool` (not built); 0.7 exposes no unblock; lifting is
  stop → build hc-dbtool → unblock → start.
- Sibling subagents of one session share the scratchpad; a `session_id.txt` there was
  overwritten by another lane. Use uniquely named files per agent.

**Why:** these are the three shapes that turned a 1-hour proof into a 3-hour one; none is
visible from the code. **How to apply:** after any storage rebuild that rolls conductors, wait
for `zomePath: live` on every peer before the prologue; if `propagate-landing-to-B` 503s, run
the declare leg with `SOURCE_DOORWAY_URL` set; if the readiness rail names a peer the features
do not measure, check that peer for `STRANDED`/`database is locked` before bypassing and write
the bypass into the habit delta. Related: [[project_devspace_recovery]],
[[project_conductor_arc_resources]], [[project_receipt_lane_quiet_host_worktree_prep]],
[[project_native_delivery_sprint_2026_09_24]].
