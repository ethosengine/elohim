---
name: feedback-sweep-leftovers-before-closing
title: Sweep side-findings into homes before closing a pass
description: "Operator 2026-09-26 — before closing a pass, fix or record every defect and stale claim noticed; nothing stays only in chat."
metadata:
  node_type: memory
  title: Sweep side-findings into homes before closing a pass
  type: feedback
  originSessionId: 4750a644-a508-47ec-9220-8a193f383cf6
  modified: 2026-09-27T00:47:16.334Z
---

"Update anything you noticed along the way, we don't want to leave anything leftover." Asked at the
end of a long pass, after I had reported side-findings only in chat.

**Why:** a finding that lives only in a session transcript is lost at the next compaction; the next
agent re-discovers it at full price. Small tooling defects and stale docs compound into the
"cryptic" failures this repo keeps paying for.

**How to apply:** at the end of any pass, list every side-finding and give each exactly one home:
- small and bounded → fix it now, with its gate (a false preflight refusal, a verb the gospel names
  but the wrapper rejects, a doc with a stale path);
- a harness trap → the owning CLAUDE.md watch-outs (e.g. genesis/a2o/CLAUDE.md);
- a research-derived design finding → a ROW in the matching backlog cluster (CLUSTERS.md first —
  the backlog .epr-meta refuses standalone entries for these);
- a slice of the claim in flight → the spec's slices / owed decisions;
- remaining steps of a lane → its plan line; evidence → the habit DELTA.
Packaged docs (root CLAUDE.md, skills) change through their package + re-projection, never the
runtime file alone. Related: [[feedback_stale_record_feeds_memory_ceremony]], [[feedback_managed_surface_edit_discipline]].
