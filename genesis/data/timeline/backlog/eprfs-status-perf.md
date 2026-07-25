---
id: "backlog-eprfs-status-perf"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "epr flow status takes ~25s in a debug cargo-run build — needs a records() cache or a release build for interactive use"
slug: "eprfs-status-perf"
written: "2026-07-25"
author: "claude (saga-status.py implementation session)"
status: "backlog"
priority: "medium"
relatedNodeIds:
  - "elohim/eprfs/epr-cli/src/flow/walk.rs"
  - ".eprfs/status/flows.jsonl"
tags: [eprfs, epr-rea, performance, dx]
---

Measured directly (T6, 2026-07-25): `cargo run --manifest-path elohim/eprfs/Cargo.toml -q -p
elohim-epr-cli -- flow status` against the live repo took **~25s wall-clock** in an unoptimized
debug build (`real 0m25.076s`), against a `.eprfs/status/flows.jsonl` with 4986 labeled resources,
3485 intents, 534 commitments, 240 events. That's `store.records()` re-parsing the entire
JSONL sidecar (2MB+) from scratch on every invocation, with no caching between calls and no
release-mode optimization. Real output confirmed: `edges: 290 sealed · 0 governed · 87 stale · 0
held · 4 dangling` (see `eprfs-stale-edge-backlog.md`, same corpus).

This is fine for a CI gate (`genesis/scripts/jenkins-sync.sh` calls it once per pipeline run and tolerates the
latency), but 25s is too slow for `flow status`/`flow walk` to be an interactive DX tool an agent
reaches for repeatedly during a session. `saga-status.py` deliberately does NOT shell out to this
binary — it reads `.eprfs/status/flows.jsonl` directly in pure-stdlib Python (measured <100ms) —
precisely to avoid this cost, but that only works because saga-status's read pattern is narrow
(one directory's commitments + events). Confirmed the release-build fix is real and cheap: `cargo
build --release` then a bare `epr flow status` invocation against the SAME sidecar ran in **~2.6s**
(`real 0m2.597s`, identical `290 sealed · 87 stale · 4 dangling` output) — an ~10x win from the
debug binary alone, no caching needed. Candidate follow-up: wire a release-mode `epr` binary into
whatever dev-tool/CI path currently `cargo run`s it debug (this repo's own `genesis/scripts/jenkins-sync.sh` included —
it uses plain `cargo run`, not `--release`, matching the T6 task's given incantation), or add an
in-process cache of the parsed sidecar keyed on the file's mtime/size for callers that can't pay
even a 2.6s cold start.
