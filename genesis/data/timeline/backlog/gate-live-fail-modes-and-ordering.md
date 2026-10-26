---
id: "backlog-gate-live-fail-modes-and-ordering"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gate tests mode and red-first ordering wait on execution-aware method identity"
slug: "gate-live-fail-modes-and-ordering"
written: "2026-10-26"
author: "codex"
status: "open"
priority: "medium"
area: "gate / derivations / execution"
relatedNodeIds:
  - "gate-live-failure-channel-design"
  - "habit:push-gate-pays-once"
  - "habit:gate-failures-arrive-live"
tags: [gate-live, derivation, gate-cost, constraint]
shift_objective: >-
  Make execution-changing modes part of method identity before considering tests
  mode or red-first ordering. Prove incompatible executions cannot reuse one
  derivation. Drop all mode because it would create a second recipe executor.
---

# Execution choices must be visible in method identity

chain: gate execution variants / between A: a declared recipe and its reusable derivation → C: tests mode or red-first scheduling / missing node B: method identity covers every execution-changing mode, including the effective ordering policy where it changes execution; probe: hold inputs constant, change the mode or policy, and demonstrate a distinct method identity and refusal to reuse incompatible evidence / current state: open; tests mode and red-first ordering remain deferred, and all mode is dropped.

The live failure channel observes the recipe already running. It does not own another executor.
The proposed `all` mode is dropped because continuing through every recipe step would require a
second recipe executor beside the declared gate machinery. This item is not an authorization to
rebuild that mode later under a different name, and it is not a statement that any execution-mode
work has been completed.

The narrower `tests` mode could change how a test runner continues after a failing test. Red-first
ordering could move likely-failing gates earlier using prior cycle observations. Both may change
execution, cost and the meaning of the resulting evidence. They must wait until the declared
method covers those choices. A successful run in one mode must never silently satisfy a different
requested method merely because the source inputs match.

[gate-evidence.mjs](../../../orchestrator/gate-evidence.mjs) already defines evidence by its input
closure and recipe/environment material, with admissibility checks for reuse.
[Derivation identity](../../../../elohim/eprfs/eprfs-core/src/derive.rs) supplies the native method
and input pairing. At pickup, inspect the actual method construction and state exactly which mode
parameters it includes; the presence of a method CID alone does not prove execution flags are
covered. Reuse this machinery instead of adding a mode-specific verdict store or hashing another
unrelated identity in JavaScript.

Acceptance must hold input bytes constant while changing an execution-affecting parameter, show
that the requested method changes, and prove that lookup cannot hit the incompatible run. It must
also show that identical method and input retain admissible reuse. Any red-first policy needs a
deterministic fallback when cycle evidence is missing and must preserve the selected gate set and
dependency constraints. These are proposed probes, not verification performed for this entry.

The [gate-live design's frontier](../../../docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md)
names fail modes and ordering; the operator's 2026-10-26 native-device-floor plan, section 7,
narrows that scope to the dependencies recorded here.
