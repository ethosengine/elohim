---
id: "backlog-gate-reuse-dark-without-rakia-cli"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gate evidence reuse never hits on a device without a rakia CLI, so every push pays the full storage gate"
slug: "gate-reuse-dark-without-rakia-cli"
written: "2026-10-29"
author: "claude (overnight integration pass, session 3f9afd7a, steward human:matthew)"
status: "open"
priority: "high"
ci_status: none
jobs: []
tags: [gate, reuse, rakia, push-gate-pays-once, performance, friction-wait, trustful-self]
---

# Gate evidence reuse never hits on a device without a rakia CLI

`GATE_EVIDENCE=reuse` refuses reuse whenever the rakia oracle is unavailable
(`genesis/orchestrator/gate-runner.mjs`: `rakia oracle unavailable; reuse needs it`).
`resolveRakiaBin` (`genesis/orchestrator/gate-oracle.mjs`) looks at `RAKIA_BIN` and
then at `PATH`. shem has neither.

**Evidence (2026-10-28/29, shem, prepared worktree `.worktrees/prepush-2`).**
Push logs `genesis/local-dev/overnight-20261027/push-lanes{2,3,4}.log` print
`[gate] rakia unavailable — path-only selection` on every gate. Lanes 2 recorded
storage evidence at `a70838fad`. Lanes 3 found it (`evidence: hit green (signer …,
commit a70838fad)`) and still refused reuse, then paid the storage gate again (about
1782 s per push). That happened on four pushes in one night.

**Why there is no binary.** The pinned `elohim/rakia` (`9db775b`) declares one
`[[bin]]`, `compute-executor`. The `rakia affected` command is captured in
`elohim/brit/tests/baseline.md` from a build on another machine, but no crate in
either pinned submodule here builds a `rakia` binary.

**Done when** a `rakia` binary that answers `rakia affected --repo <r> --files <f>`
is built from a pinned source into the device bin-store (as brit is). `RAKIA_BIN` or
`PATH` must resolve to it in the pre-push environment. A push log must then show
`reused` for elohim-storage on an unchanged storage closure. That push log is also
the evidence that flips `push-gate-pays-once` green.

**Decide first:** which repository and commit hold the rakia CLI source. Either it
is a crate that has not reached the pinned rakia yet, or the reuse rule should accept
brit's `build plan` as its oracle in place of rakia.
