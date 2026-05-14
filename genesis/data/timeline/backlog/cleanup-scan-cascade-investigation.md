---
id: "backlog-cleanup-scan-cascade-investigation"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Investigate cleanup-scan cascade-hidden surface — 67 dangling refs that path-update can't fix"
slug: "cleanup-scan-cascade-investigation"
written: "2026-05-14"
author: "librarian"
status: "proposed"
priority: "medium"
relatedNodeIds:
  - "memory:feedback_cascade_hidden_test_surface"
  - "memory:project_signal_driven_audit_ceremonies"
  - "memory:reference_superpowers_docs_location"
tags: [memkit, cleanup-scan, path-update, cascade-hidden, ceremony-design]
shift_objective: |
  Round 4 Wave 6 confirmed cleanup-scan's "review flags" surface is structurally
  independent from path-update's "git rename" surface. Applied 77 accepted git renames
  (102 files modified, 139 occurrences replaced) and the cleanup-scan count stayed at
  67. The 67 flagged dangling references are NOT renames — they're references to files
  that were planned-but-not-built or deleted-without-rename (e.g.,
  `account-import-result.schema.json`, `genesis/seeder/src/deployment-registry.test.ts`,
  `revocation-self.feature`, `recovery_invitation.rs`). Path-update can't fix these
  because there's no rename evidence.

  Investigate three classes:
  1. Plans/specs that reference files that were never built (status: spec-defunct;
     candidate for archival or status-flag).
  2. Plans/specs that reference files that were deleted-without-rename (status:
     reference-stale; candidate for citation removal or update).
  3. Plans/specs that reference files in a different repo or out-of-tree (status:
     out-of-scope; candidate for opt-out marker analogous to .no-claude.md).

  Done when (a) the 67 flagged items have a per-item disposition; (b) cleanup-scan
  gains a "spec-disposition" classifier to suppress out-of-tree refs from "review
  flags"; (c) cleanup-scan count drops below 20 after the disposition pass.
---

# Cleanup-scan cascade-hidden investigation

## Why this matters

Round 4 Wave 6 Phase 6a applied path-update-apply.py with all 77 accepted entries.
The path-update touched 102 files and replaced 139 occurrences — real, working fixes
to dangling references caused by git renames. But cleanup-scan's count stayed at 67.

This is the same cascade-hidden pattern documented in
`feedback_cascade_hidden_test_surface.md` from the EPR sprint era: fixing one surface
unmasks the real distribution. The 67 dangling references are not rename-fixable;
they're references to artifacts that were planned but never built, or deleted
without trace.

This means cleanup-scan is currently a noisy signal. It will not advance under
path-update ceremony, no matter how thoroughly the operator runs it. Librarian's
hygiene loop needs a different ceremony class — one that triages disposition per
flagged spec.

## What's blocking

Nothing structural. This is investigation + script enhancement work.

## What's ready

- The 67 items are surfaced in `.claude/memory-kit/2026-05-14/cleanup-proposals.md`
- cleanup-scan.py is well-scoped and easy to extend
- Storyteller's disposition pattern (graduate / memorialize / hold / archive) maps
  naturally to spec disposition

## Who knows the area

Librarian (owns the memkit toolkit). Cartographer (synthesis lens; would be the
beneficiary of cleaner cleanup-scan output).

## Convergence

- Librarian Round 4 Wave 6 Phase 6a: cascade-hidden surface confirmed via forced-choice
- Round 3 silent-demote lesson: dimensions must advance OR document why they can't

## Definition of done

1. Each of 67 flagged specs has a disposition tag (defunct / stale / out-of-scope)
2. cleanup-scan.py gains a disposition-aware classifier
3. Out-of-scope references suppressed from "review flags"; spec-defunct items
   surfaced as archive candidates instead
4. Post-pass, cleanup-scan count drops below 20 (target: actual hygiene signal)
5. Memory entry created: `feedback_cleanup_scan_disposition_classifier.md`

## Disposition taxonomy (Run #5 Wave 6 authoring)

The 67 flagged dangling-refs from cleanup-scan are structurally independent of path-update surface (per Run #4 + Run #5 confirmation). They resolve to four disposition classes:

### Class 1: spec-defunct
Planned but never built. The reference points to an artifact in a spec/plan that wasn't realized.
Examples (sampled from the 67):
- `recovery_invitation.rs` — referenced in spec; never created in git history
- `epr-resolver.service.ts` — proposed in plan; superseded by EprService design
- `hub-topology.feature` — drafted; collapsed into household-topology before authoring

Disposition: archive the parent spec/plan with intent-preserved annotation, OR remove the cited line if the spec is still live.

### Class 2: reference-stale
Built then deleted-without-rename. The reference is to a real-once-existed artifact that left git via deletion rather than rename.
Examples (sampled from the 67):
- `REACH.md` — existed in earlier substrate; restructured into reach-as-property memories
- (other 2 examples to be filled by Run #6 librarian during mechanization)

Disposition: remove the citation from the citing document, OR re-link to the successor.

### Class 3: out-of-scope
Cross-repo references, or paths where a `.no-claude.md` marker or analogue suppresses the audit. Currently un-detectable by cleanup-scan because the marker pattern is `.no-claude.md` (CLAUDE.md opt-out) not a generalized cleanup-scan opt-out.
Examples (sampled from the 67):
- `genesis/genesis/docs/...` — typo-class duplicate path (the `genesis/genesis/` is broken; should be `genesis/`)
- (other examples involving `sophia/` submodule paths if any)

Disposition: typo-class is one-edit fix; cross-repo class needs cleanup-scan-opt-out marker generalization (out-of-scope for Run #5; Run #6 librarian).

### Class 4: memorialized-as-decision
The cited file was itself a decision-marker, not a future deliverable. Reference is preserved-meaning, not future-work.
Examples (sampled from the 67):
- (to be filled — sample 3 by Run #6 librarian)

Disposition: re-route the citation to the chronicle entry that captured the decision; remove the artifact-pointer.

### Run #6 mechanization

With this taxonomy, the librarian can write a classifier subroutine in `cleanup-scan.py` that:
1. Reads each dangling-ref's commit history (`git log --diff-filter=D --follow`)
2. Routes to class 2 (reference-stale) if a deletion exists in history
3. Routes to class 1 (spec-defunct) if no creation exists in history
4. Routes to class 3 (out-of-scope) by path-prefix pattern matching
5. Routes to class 4 (memorialized-as-decision) by tags/links in citing document

Each class then has a different disposition action (archive parent, remove citation, suppress, re-route).
