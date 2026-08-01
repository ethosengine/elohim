---
id: "backlog-recording-provenance-quiesced-stamp"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Recording provenance: quiesced stamp in sprint-report + fulfill refusal of unquiesced Dismisses"
slug: "recording-provenance-quiesced-stamp"
written: "2026-08-01"
author: "quiescence-gated-saga-recording shift (RCA of the premature edge #1280 recording run)"
status: "backlog"
priority: "medium"
jobs: [elohim-edge]
---

# Recording provenance: quiesced stamp in sprint-report + fulfill refusal of unquiesced Dismisses

**Status:** open
**Domain:** CI measurement / eprfs recording ledger
**Spawned by:** 2026-08-01 quiescence-gated-saga-recording shift (RCA of the premature edge #1280 recording run)

## Concern

The saga recording ledger sync (`genesis/scripts/jenkins-sync.sh` → `epr flow fulfill`)
pulls `sprint-report-dataplane.json` from the edge job's `lastSuccessfulBuild` and
writes a Dismiss (recorded regression) for any red concern. Edge #1280 (2026-08-01)
closed **SUCCESS at build level while its report carried churn-window reds** — a
contaminated report can therefore become the recording source and downgrade
recorded chapters. The fleet-quiesce gate (landed this shift) prevents the report
from being *generated* during churn, which closes the main path — but the fulfill
side still trusts any report it is handed.

## Cure shape (deferred — touches epr-cli Rust, out of the gate shift's scope)

1. `run-dataplane-validation.sh` / `build-sprint-report.ts` stamp the report with
   quiescence provenance (e.g. `quiesced: true` + gate observation timestamps)
   only when `fleet-quiesce-gate.sh` exited 0 in the same stage run.
2. `elohim/eprfs/epr-cli/src/flow/fulfill.rs` refuses to write a **Dismiss** from a
   report lacking the stamp (Produce from an unstamped green run may also be
   refused, or accepted with a warning — decide at implementation).
3. Contract test: a stampless report cannot move recorded state downward.

## Related

- `genesis/data/timeline/backlog/` REA heal classify-write TOCTOU transactionalize (79cb11402) — same shift family.
- Museum candidacy: "measurement during post-restart catch-up window" — first instance
  recorded 2026-08-01 (edge #1280 RCA); a recurrence earns a museum-table row in
  `genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`.
