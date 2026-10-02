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
tags: [eprfs, epr-rea, performance, dx, perf-latency, perf-scale, unit-history, lane-operator, trustful-self, friction-verify, lane-borrowed, phase-growth, risk]
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

## 2026-10-02 — gradient reading of the sidecar and its callers (static, unmeasured)

A read-only survey of `elohim/epr`, `elohim/eprfs` and the `epr-rea` store with the
`gradient-reading` skill. **Nothing here was measured.** Sizes are what was on disk that day; the
three items marked ✔ were re-read by the controller, the rest stand on the surveyor's reading.

**The sidecar has grown six-fold.** `.eprfs/status/flows.jsonl` is 12,181,720 bytes and 18,629
records (this entry measured it at "2MB+" on 2026-07-25). 4,976 of its 6,709 events are
`run:observation` notes written by hooks. The memory reader refuses outright above 33,554,432 bytes
(`memory/validation.rs:660-666`), so there is a cliff ahead, and raising it would be a limit raise.

**One primitive is under most of the list.** `SidecarFlowTransaction::records()`
(`epr-rea/src/store.rs:276-293`) ✔ reads the whole file, parses every line, re-encodes every record
and recomputes its CID to compare with the stored one. Every default trait method (`events`,
`commitments`, `unfulfilled_in_scope`, …) calls it again. This is `trustful-self` + `friction-verify`
+ `unit-history`: one steward re-verifying its own ledger from the first record on every call. The
verification is deliberate ("Refuse to append behind broken evidence"). The compression is to carry
the verified prefix as a fact — a verified-through offset plus a prefix digest — and verify only the
tail. No check is removed.

| # | Path | What multiplies | Runs on | Confidence |
|---|---|---|---|---|
| 1 | `flow note` re-validates the whole ledger under the exclusive lock to append one record (`flow/note.rs:831`) | history × notes per session; readers wait behind the lock | hook observations on Edit/Write and Bash | high shape, medium impact |
| 2 | `recall open` hashes the 49.6 MB executable, passes the sidecar twice and reads the doc corpus three times (`recall/mod.rs:1390`, `flow/edges.rs:190-294`, `flow/concerns.rs:113-146`) | per user prompt | UserPromptSubmit hook, "a measured 6-second budget" | high / medium |
| 3 | `flow status` calls `unfulfilled_in_scope` per distinct scope against the store, not the loaded snapshot (`flow/walk.rs:474-510`) ✔ — two full passes per scope, 237 scopes today | scopes × history | `jenkins-sync.sh`, interactive | high |
| 4 | `flow report --bound <id>` does the same work as the full report and writes on read (`flow/report.rs:717-763`, `flow/scope.rs:589-667`) | per call; four parallel processes from `habits-status.py` serialize on the lock | SessionStart, sovereignty landings | high shape |
| 5 | `epr govern` hashes its own executable per process and parses the cascade three times (`govern.rs:62-83` ✔, `eprfs-meta/src/lib.rs:90-167`) | per Edit/Write and per staged file | compose gate, pre-commit gate | high shape, low–medium size |
| 6 | `cites seal <doc>` rebuilds the slug index 3–4 times, reading 22 MB of bodies for one `id:` scalar each (`flow/cites.rs:604-637`) | docs × corpus | agent-run after cite edits | high |
| 7 | `memory attribution` resolves all 275 memory entries to answer for one (`memory/attribution.rs:455-509`) | entries × history | every memory Edit/Write | medium–high |
| 8 | `memory import` opens one full-validation transaction per appended entry (`memory/import.rs:430-450`) | appended entries × history | detached import worker | medium–high |
| 9 | The roster signature chain is re-verified on every attributed command (`actor.rs:802-900`) | per call × roster | every `flow note` | medium real, low size |
| 10 | The local store hashes bytes to answer presence; materialise reads twice and hashes three times per blob (`eprfs-storage/src/lib.rs:249-283`) | per manifest entry | projection and plant runs | high shape, low frequency |
| 11 | `labels.json` (1.4 MB) parsed whole and the snapshot deep-cloned per trait call (`flow/walk.rs:142-200`) | per call × history | walk, status, stocks | medium |
| 12 | Bootstrap `open` walks the repo for a habit atom; `flows.jsonl` already exceeds the 1 MiB register budget and is omitted | per new session | SessionStart | high that the code says so |

How this corrects the entry above: the cost of `flow status` is not one re-parse per invocation but
one per scope, twice. `walk()` in the same file already uses a snapshot adapter.

Found fine, and the pattern to copy: the semantic index plan (`recall/index.rs:267-302`) stats first
and hashes only when size or mtime moved. Items 2 and 5 could key the executable digest the same way.

Deliberate checks that stay: verification under the append lock (1), the executor digest as a
session pin (2), `reader.unchanged()` before a memory write (8), store-side verification of a
storage handle (10). Each cure carries an already-verified fact; none skips a check.

One measurement each, before any change: wall-clock of one `epr flow note` against the live sidecar;
opens of `flows.jsonl` during one `epr flow status`; `epr flow report --bound <id> --json` against
plain `--json`; bytes read by one `epr govern --path X` split by executable and everything else.

Not read: `eprfs-agent`, `eprfs-cli`, most of `epr-cli`'s flow verbs beyond those named, all tests,
and how often each conditional hook fires in a session. The build profile of the installed `epr`
is unknown.
