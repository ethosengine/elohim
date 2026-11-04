---
id: "backlog-native-dev-doc-drift-rakia-eprfs-epr"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doc drift: rakia CLAUDE.md on rakia-executor, stale rakia and eprfs README verb lists, EprKind 'nine kinds', steward-space 'Rakia serves'"
slug: "native-dev-doc-drift-rakia-eprfs-epr"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "low"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, docs, drift]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §11 closing paragraph, §3 item 9)

Fix in passing by whichever track touches the file.

- `elohim/rakia/CLAUDE.md:~18` lists `rakia-executor` among commented-out, 'planned, not present' members. Spec says it is live as the measure peer-stage worker (see also `rakia-executor-untracked-in-submodule-pin`: it is untracked in the submodule pin, so check that before rewording). It is not a build executor (D8).
- rakia and eprfs README verb lists are stale (not individually re-verified here).
- `elohim/epr/src/kind.rs:1` says 'the nine EPR kinds'; the enum has 12 variants (verified by count).
- `genesis/docs/superpowers/specs/2026-10-27-steward-space-landing-plane-design.md` line ~575 'Rakia serves' wording: the cited line now reads 'rakia reach climbs by ceremony' in the bullet list at 573-577; the exact 'Rakia serves a session the root manifest' phrase is §5.1 text near there. Re-locate with grep before editing; superseded wording: the tier serves, rakia names.

**retire-when.** The four statements match code.
