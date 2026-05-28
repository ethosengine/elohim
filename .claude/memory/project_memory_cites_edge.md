---
name: project_memory_cites_edge
description: Memory entries declare a `cites:` frontmatter edge to the code/spec/scenario they depend on; a walker + in-flight accumulator hook re-open them when cited sources change — the reconciliation-controller pattern applied to memory.
metadata:
  node_type: memory
  type: project
cites:
  - genesis/docs/superpowers/specs/2026-05-28-in-flight-memory-coherence-design.md
  - .claude/scripts/memory-kit/memory-coherence-audit.py
  - .claude/hooks/memory-coherence-signal.py
---

Memory was the one substrate in the repo that lacked **edge-invalidation on source change**. The build-graph has it (`graph-walker.mjs` walks `build-manifest.json` `inputs.sources`); code→scenario has it (`sync-check.py` + `file-relationships.json`); stories declare `feature:`/`anchors_epics:`; gospel surfaces are walked by `substrate-currency-audit.py`. But a memory entry that said "see `path_service.rs`" had no machine-walkable link — when the code moved, the lesson silently went stale. This is the same drift that let the memory team's own agents carry a deleted "Wave 3 stasis template" for ~2 weeks.

**The edge.** A memory entry may declare an optional top-level `cites:` frontmatter list of repo-relative paths or globs whose change should re-open it for re-verification — the memory-side mirror of a story's `feature:`/`anchors_epics:`. This file dogfoods it (see the frontmatter above).

**Why:** memory is only trustworthy if it stays current with the substrate it describes. Periodic ceremonies catch drift in batch, after it accumulates; the `cites:` edge makes drift surface continuously and cheaply (signal-driven, into the hygiene-sweep), so a lesson re-opens the moment its foundation moves rather than the next time someone happens to re-read it.

**How to apply:** when you write or revise a memory entry that leans on specific code, specs, or `.feature` scenarios, add them to `cites:`. Rollout is organic — `memory-coherence-audit.py`'s `CITE-CANDIDATE` finding nominates entries that have code paths in their body but no `cites:` yet. The `memory-coherence-signal.py` PostToolUse hook bumps an entry's counter when edited code matches its `cites:` (via the cached `cites-index.json`); the librarian surfaces and resets these during `/hygiene-sweep`. Run `memory-coherence-audit.py` to rebuild the index and flag `DEAD-CITE`s (cited paths that no longer resolve).

Future affordance: `cites-index.json` is the index that could later make MEMORY.md injection *scoped* (inject only the entries whose `cites:` match the files in play) instead of flat-injecting the whole index into every process tree.

Related: [[project_memory_in_repo_two_tier]], [[project_signal_driven_audit_ceremonies]], [[project_principle_p1_reconciliation_controller]], [[feedback_agent_prompts_no_process_status]], [[project_memory_lifecycle_comet_shape]].
