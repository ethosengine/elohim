---
id: "backlog-gate-reuse-dark-without-rakia-cli"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gate evidence reuse never hits on a device without a rakia CLI, so every push pays the full storage gate"
slug: "gate-reuse-dark-without-rakia-cli"
written: "2026-10-29"
author: "claude (overnight integration pass, session 3f9afd7a, steward human:matthew)"
status: "resolved"
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

## 2026-11-04 note (native-dev-plane-one-derivation-earned-reach-design)

Spec §11 Track B cures this by resolving selection through `brit build affected` rather than the deprecated `rakia` PATH binary (`gate-oracle.mjs:24-37` re-verified). Tracked in `native-dev-track-b-gate-oracle-via-brit-build-affected`; close both together. Receipt: a reuse hit with no `rakia` on PATH.

## 2026-11-21: closed (wall-clock sprint, session 824f0d7e)

`gate-oracle.mjs` resolves `brit` (`BRIT_BIN`, else PATH) and runs
`brit build affected --repo <root> --files a,b`. `resolveRakiaBin`, `rakiaAffected` and every
`RAKIA_BIN` read are deleted, with the a2o devflow fixtures that called them. An unavailable
oracle now prints its reason in the path-only line and in the reuse refusal.

Receipt, shem, at the landing's head with every `rakia` taken off PATH (both
`~/.local/bin/rakia` and `/opt/rust/cargo/bin/rakia` are launcher aliases on this device) and
`brit` left on it, `GATE_EVIDENCE=reuse just gate elohim-compute`:

```
[gate] elohim-compute evidence: hit green (signer 8b4cf6856d65, 4h ago, commit 5535405d4); closure scope paths (0 reads)
[gate] elohim-compute reused: green 4h ago at 5535405d4 (signer this workspace)
```

`brit build affected` and the `rakia` alias printed byte-identical JSON for the same change set
on this device (0.08 s each), so selection did not move. The pin-move receipt of the 2026-11-04
correction (dependents planned with sibling worktrees present) was not re-run.

