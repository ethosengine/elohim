---
name: feedback_memory_index_is_projected_never_hand_edited
title: "MEMORY.md is projected; pytest lives in the wiped home"
description: "The repo's .claude/memory/MEMORY.md is projector output and a hand-added line fails the pre-push parity gate; a workspace restart also removes user-site pytest, which a pre-push harness imports."
id: feedback-memory-index-is-projected-never-hand-edited
metadata:
  node_type: memory
  title: MEMORY.md is projected; pytest lives in the wiped home
  type: feedback
  originSessionId: 09892ed7-62ff-451d-a4cc-7871ccfa5ccb
  modified: 2026-10-18T10:59:48.260Z
cites:
  - .claude/hooks/memory-index-projection.py
  - .claude/hooks/__tests__/memory_index_projection_test.py
---

Two pre-push refusals on 2026-10-18 cost ~55 minutes of gate time each, both environmental:

1. **MEMORY.md is generated.** The repo-level index `.claude/memory/MEMORY.md` is the output of
   `epr flow memory project --index --budget memory-index-bytes@1 --out .claude/memory/MEMORY.md`,
   and the memory-ceremony gate's `memory_index_projection_test.py` (golden parity) refuses a push
   when re-projecting renders different bytes than the tree holds. A memory entry born in a session
   with no actor claim is left out of the projection as unattributable (the PostToolUse hook says
   so), so hand-appending its index line breaks parity. **How to apply:** write the entry file,
   never the index; run the projector; list unattributable entries for the steward of record to
   claim (`epr flow memory import <entry> --session steward-of-record --steward-of-record`).
2. **pytest is in the user site.** `$HOME` is wiped by a workspace restart, taking
   `~/.local` pytest with it; `.claude/scripts/_lib/__tests__/deprecation_sentinel_capture_test.py`
   imports pytest and the agentic-library harness leg then fails. **How to apply:** after a restart,
   `python3 -m pip install --user pytest` before the first push (alongside `git config --global
   --add safe.directory '*'` and `rustup toolchain install 1.100.0`, the RUSTUP_TOOLCHAIN pinned in genesis/agentic/pool-policy.json).

**Why:** the full pre-push gate runs ~50 min; a refusal on the last leg wastes the whole run.
Related: [[feedback_push_branch_discipline]], [[project_shem_workspace_runs_long_sprints]],
[[project_devspace_recovery]].
