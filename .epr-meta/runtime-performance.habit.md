---
epr-habit-version: 1
id: runtime-performance
invariant: >
  A local runtime performance report never turns an incomplete or incomparable observation into
  a quiet zero or a green trend. It carries the declared operator cohort and threshold, preserves
  the canonical CheckWitness vocabulary, and distinguishes measured regression, invalid evidence,
  and an unanswered comparison so an operator or LLM can act without mistaking absence for health.
status: red
active: false
checks:
  - "runtime instrumentation acceptance: from genesis/a2o, `node --import tsx scripts/runtime-performance.ts check --input CAPTURE --perf ABSOLUTE_PERF --require-coverage all --json` — explicit local artifacts required; missing coverage exits 2, measured insufficient coverage exits 1, complete direct evidence exits 0. This does not replace a matched runtime budget comparison."
  - "@concern:runtime-performance — source-backed operator story in genesis/a2o/features/deployment/runtime-performance-report.feature; the focused commands below are its current proof boundary, not a live household acceptance claim."
  - "focused TypeScript tests: `pnpm --dir genesis/a2o exec tsx --test scripts/__tests__/runtime-performance.test.ts` — the local runtime-performance CLI emits the canonical `CheckWitness`-shaped report, validates declared cohort/threshold inputs, and compares only valid samples without treating missing data as zero."
  - "focused TypeScript tests: `pnpm --dir genesis/a2o exec tsx --test scripts/__tests__/performance-prometheus.test.ts` — Prometheus/runtime counters distinguish sampled CPU/resource observations from actual call counters and make timeout, cancellation, missing series, and invalid windows non-green rather than silently successful."
refs:
  - "genesis/a2o/scripts/runtime-performance-governance.md — guarantee matrix and explicit seam-registration gap"
  - "genesis/a2o/features/deployment/runtime-performance-report.feature — source-backed operator story; @wip until a live A2O lane owns it"
  - "elohim/epr/src/verdict.rs — protocol-owned CheckWitness / CheckOutcome / Decision vocabulary"
  - "elohim/elohim-storage/.epr-meta/idle-is-free.habit.md — existing RED idle budget remains unchanged"
retire-when: >
  every supported runtime yields an attributed cost series and a bounded trend by construction,
  with declared cohort and threshold validation, and comparison cannot produce a false green from
  missing, timed-out, cancelled, or incomparable samples; at that point this report habit is
  enforced by the runtime/report path rather than by a separate watch.
---
RED written 2026-09-20: governance and source-backed story are declared; focused CLI and Prometheus
tests are the proof boundary. No live household or fleet claim is made here. The existing
`idle-is-free` habit remains RED and is not altered by this reporting capability.

2026-09-20 evidence: the four focused collector/parser/process-witness/report test files pass;
the CLI subprocess tests cover compare exits 0/1/2, and a saved restored-household capture
reports 8.778 observed CPU cores without inventing a comparable cohort. Collector hardening
passed independent review; the A2O lint/format/typecheck gate and 220-feature Gherkin parser
passed. Still RED: native method/workflow/SQL attribution and automatic runtime acceptance
wiring are incomplete; successful report-tool tests do not establish household budget health.

2026-09-20 review delta: independent reporter re-review approved raw-witness recomputation,
endpoint-specific intervals, explicit invalid-evidence handling, signed I/O adjustments and
profile identity matching. Restored-cast CPU profiles remain roughly 99% unknown for James
and Jessica, with caller clusters withheld; Matthew's truncated profile is excluded.

2026-09-20 first-report correction: exercising saved evidence against perf's own self-CPU report
revealed that the initial roughly 99% unknown result conflated absent DWARF caller frames with
unresolved leaf symbols. Matching binaries resolve James/Jessica leaf samples (under 0.3% unknown);
James SHA-512 accounts for 45.469% of sampled self CPU. The previous unknown-symbol interpretation
above is superseded, not evidence that a rebuild is required. Caller/workflow attribution remains
unproved, Matthew remains excluded, and the habit remains RED. The regression story separates leaf
resolution from caller coverage; the private first report and repeatable reconciliation loop name
the next measurement rather than requiring reconstruction from chat history.

2026-09-20 atomic-instrumentation delta: storage source now records completed conductor-attempt
duration/outcome after admission, separately counts caller abandonment, and exports a stable
metrics-start producer marker. Independent review approved the bounded labels and drop semantics;
Rust tests are NOT verified (our compile was stopped and lease released because another session
owned the operator's single-build lane). The report surfaces method latency separately from
dispatch counts and atom latency. A2O lint/format/typecheck and four focused TypeScript test files
passed; the expanded source story received a context-isolated READY review. Native tests, a fresh
runtime capture, per-cell workflows and statement timing remain the RED frontier; exact resume
commands are in genesis/a2o/scripts/runtime-performance.md.

DELTA 2026-09-23 (fork half committed; first live household admission receipt; still RED): the
conductor-side instrumentation that had sat uncommitted in the fork's working tree since
2026-09-21 is now fork commit 915acf6bc on int/2026-09-23-diagnostics-throttle-perf (SQL timing,
workflow wake/run attribution, heap-capture canary, admin wire types, seam registries), verified
on rust 1.96.1: holochain_conductor_api 31 tests, holochain_trace 29 + 1 SQLite-worker integration,
holochain lib 29 passed / 1 pre-existing ignored (the three Linux-native heap tests need the
jemalloc-prof feature and did not run). Live: the household's three conductors ran the
production-feature release build with HOLOCHAIN_SQL_DIAGNOSTICS_DIR authorized; `runtime-performance
arm --family sqlTiming --seconds 30` against matthew's admin websocket answered admitted
(producer sql-f720b-1a0cf94e8c9, generation 1) and the conductor wrote
sql-timing-g01-hh-20260923-a.jsonl (20 511 bytes, mode 0600) carrying the kernel process witness
(pid, start ticks, boot id, executable) and HMAC statement identities with counts and elapsed —
no statement text. That is admission and a terminal artifact, not coverage: no matched CPU
capture accompanied it, the workflow and heap families were not armed live, and `check
--require-coverage all` is unchanged. Private receipt: the artifact stayed in the session's
private directory; nothing from it is imported here.

2026-09-20 framework-reuse delta: audited existing perf/libdw, pprof-rs, conductor timed tracing,
metrics, jemalloc and Diesel query instrumentation; documented what is wired versus optional,
including pprof's missing route authentication/response ceiling. Added and independently reviewed
CPU-only 8/16/32 KiB DWARF stack-budget selection with persisted capture settings and unchanged
sampling/window/byte caps. Four focused TypeScript test files pass. No new live capture or native
test: both mesh and cargo remained leased elsewhere. The normal A2O gate encountered duplicate
project manifests in another session's source export; direct project checks are recorded separately.
Caller recovery remains an experiment, not a completed attribution claim; habit remains RED.

2026-09-20 verification completion: A2O's declared checks run directly (`pnpm run lint &&
pnpm run format:check && pnpm run typecheck`) exited 0: 0 lint errors (348 existing warnings),
formatting clean, typecheck clean. This validates the project changes but does not clear the
separate gate-registry duplicate-manifest failure or the outstanding native/live checks.

2026-09-20 first live 32 KiB preflight: private `serving-edge-20260920/cell-startup-dwarf32`
has three identity-valid, nontruncated profiles and no reported sample loss. Larger stacks did not
make callers usable (James 0%, Jessica 2.459%, Matthew 0.155%); clusters remain withheld. The
CellDisabled investigation also found already-live SQLx slow-statement events; publish backlog
queries dominate the censored >1s sample. Readiness snapshots and caveats are preserved in
`FINDING-CellDisabled-startup-readiness.md`; no reset/restart/reinstall was performed. These are
startup diagnostics, not an idle budget pass or a proven causal bottleneck; habit remains RED.

2026-09-20 bounded-telemetry implementation evidence: the runtime-settings library tests pass
41/41, including diagnostic generation/rearm and concurrent boot/runtime writer coverage.
Collector tests pass 17/17 with explicit frame-pointer selection; the reporter's reviewed
descriptor-bound ingestion and exact lifecycle-pairing regressions pass 32/32. The expanded
10-scenario source story parsed and received an independent READY review. Source additions now
include bounded default-off storage SQL/site and conductor-attempt diagnostics, bounded per-cell
workflow lifecycle events, and pprof cancellation/output-size hardening. Native storage tests are
still pending; conductor telemetry is formatted/reviewed but uncompiled. No telemetry binary was
installed or mesh state changed. `check --require-coverage all` still refers/exits 2 on saved
household evidence: complete SQL identity, usable callers, heap/I/O attribution, matched-state
scaling and live runtime acceptance remain outstanding. This is deliberately not a green flip.

2026-09-20 native verification delta: runtime-settings 41/41 remain green; storage's focused
diagnostics (5), pprof (11), conductor-call (10), bounded DB labels (1), admission timing (1),
and process-start identity (1) tests all pass in the standalone warm pool slot. Pprof includes
a real gzip profile and deterministic abandoned-waiter singleflight regression. The full owning
gate and conductor build/live acceptance are still not claimed. The initial pool resolver chose
the excluded crate's parent slot; that compile was stopped and the manifest-authoritative
`elohim__elohim-storage/dev` slot used instead. No caches were removed or limits raised. Cargo
was released after verification; mesh remained owned by the other session. Habit stays RED.

2026-09-20 attribution review delta: bounded `io` and `cpu-io` capture now reuse perf trace;
the collector/reporter tests cover full argv, event-cap races, failed-syscall stacks, spawn
cleanup and public-report redaction. A bounded self-only host fixture verified real failed-read
syntax; no household was attached. Root independently reran the collector, reporter and
Prometheus-parser test files (all three exit 0; collector 20 and reporter 38 assertions).
Typed websocket deadline counters now preserve each admitted method's zero series and distinguish
transport deadlines from caller abandonment; their new native tests are pending the cargo lane.
The isolated conductor build and remaining network/heap/statement attribution are not acceptance
evidence yet. No binary was installed, no pin moved, and this habit remains RED.

2026-09-20 native build blocker: the isolated pinned-conductor test compile exited 101 after
40m26s on nine existing errors in the optional `instrument` feature, outside the three-file
telemetry patch. Receipt: private `serving-edge-20260920/conductor-telemetry-native-test.txt`.
The new workflow events and tests are unconditional; the approved retry omits only that broken
optional feature, not the telemetry tests. Retry, new typed-timeout native tests, and release build
are pending the other session's actual Cargo test. Mesh remains leased elsewhere. No release
binary exists from this attempt; source review is not native verification.

Chain / between bounded capture -> complete RCA acceptance / missing station: match workflow,
SQL, network, and heap observations to the same native process and measurement window, with
explicit completeness/loss witnesses. Probe: `check --require-coverage all` on that capture.
Current state: RED; standalone parsers and synthetic fixtures cannot discharge this station.

2026-09-20 final source tranche: root independently reran the actual host telemetry runner after
review fixes (84 tests across 9 suites, all passing). Coverage includes syscall failure formats,
bounded native network transport, strict missing-versus-empty network evidence, and the existing
jeprof heap adapter's private input snapshots and child-process cleanup. The heap CLI is descriptive
only until native identity/dump evidence exists. All three storage role dispatches now share the
same admitted-attempt observer; independent static review passed, new Rust tests still await Cargo.
The saved-capture `check --require-coverage all` exits 2 and prints each missing category; no
acceptance claim is inferred from parser tests. Complete SQL attribution, matched native workflow
and heap evidence, caller recovery, three household states and scaling proofs remain outstanding.

2026-09-20 Cargo handover verification: with one build job and one test thread, the current
storage tree passes 26 focused native tests (attribution 7, conductor-call metrics 2, typed timeout 1,
dropped-call censorship 1, pprof/cap/gzip 15) and `cargo clippy -j1 -- -D warnings` exits 0.
Persistent `serving-edge-20260920/storage-*.log` receipts carry the individual commands/results.
The `gzip` helper is test-gated; the reported warning does not reproduce. Conductor verification
continues separately without the broken optional `instrument` feature. The operator still owns
the mesh; no live telemetry acceptance or full owning gate is claimed. Habit remains RED.

2026-09-20 native diagnostic receipt: the isolated conductor queue module passes 15 tests,
including all seven new telemetry tests (one pre-existing ignored). The one-job release build
passes without the pre-existing broken optional `instrument` feature; source/patch/binary hashes
and flags are recorded in `serving-edge-20260920/conductor-diagnostic-build-receipt.txt`.
The artifact is not launched and the pin is unchanged. One subsequently authorized read-only
15-second capture, `telemetry-ssr-confounded-20260920T233706Z`, exits 0 but its all-coverage check
exits 2: capped I/O, failed network collection, insufficient CPU callers and unbound metrics.
The mesh lease was released immediately after capture for the operator's SSR cleanup. This
285 MB bloated-bundle household is not a healthy baseline or scaling proof. Habit remains RED.

2026-09-21 evidence delta: the isolated telemetry-only storage export excludes the concurrent
1.4b migration/write work; its `868e7c8a9c1ac272c87743aab221fa23747556296f256ccd6fcc6843b2a20c63`
candidate passed eight focused native attribution tests and the dual-feature build (receipt:
`serving-edge-20260921/BUILD-elohim-storage-telemetry-only-export.md`), but is not adopted.
Root independently passed the actual-host runtime report suite (42 tests). The saved
`cpu-io-network-15s-corrected` all-coverage CLI check still exits 2: network watermarks are
present, but CPU callers are insufficient; method metrics were not requested; and SQL, workflow,
and heap evidence are missing. Native SQL capture remains pending. Habit remains RED.

2026-09-21 drilldown delta: 97 telemetry tests across 9 suites, full telemetry-owned ESLint and formatting pass; SQL static-site attribution passes 18 native tests plus a real nested-async SQLite-worker test (source-only, not in the running binary). Saved FP60 evidence now exposes contiguous caller prefixes without relaxing strict coverage; SQL Markdown exposes capture-local statement/site timing and mapping gaps. Earlier pending-launch notes are superseded by the isolated and guarded live receipts in `serving-edge-20260921/FINAL-TELEMETRY-VERIFICATION.md`; startup-capped SQL is not idle attribution. The unsafe workflow-lifecycle attempt was restored byte-for-byte, not shipped. Native lifecycle, synchronized collector binding, bounded heap capture and controlled scaling remain open; habit stays RED. Projection check still refuses duplicate declarations in shared worktrees/source exports; the generated register was not hand-edited.

2026-09-21 binding/lifecycle delta: 129 targeted tests across 12 suites and full A2O typecheck pass (private `serving-edge-20260921/telemetry-binding-lifecycle-tests.log`). The collector saves bounded private SQL artifacts; the reporter rederives PID/start/boot/executable and bracketed monotonic-window binding from raw bytes without granting SQL coverage. Workflow readers validate quiet close, counters and declared censorship; late terminal emission cannot dilute active-window rates. A bounded FIFO experiment proves a 32 KiB persisted-byte cap but not cancellation of jemalloc traversal. A read-only census confirms 95 running cells / 19 unique running agents; no controlled scaling pair is claimed. Native workflow compilation, re-arming after startup, hard-deadline heap capture and matched live acceptance remain open. No runtime was restarted or pin moved in this tranche; habit stays RED.

2026-09-21 resumed private-control delta: the explicit local-only `runtime-performance arm` adapter passes eight tests, including the installed Holochain client's real WebSocket/MessagePack transport against a local test server; the preceding combined control/report run passed 54 tests. Root-owned CLI/control lint and A2O typecheck passed before the final wire test was added. Admission is never completion or coverage, and ambiguous transport failure never auto-retries. Native re-arm and heap controls remain under implementation/verification; independent review rejected the first canary helper's busy empty-pipe loop and unsupported isolation claim, with corrections pending. No conductor was armed, launched or restarted. Habit remains RED; no end-to-end or push acceptance is claimed.

2026-09-21 verified-control delta: frozen native source `73418bc13a10f6f80e1e39eba15ba6b87e7d946532e1fb8dd73db6d6aa0d7293` passes the jemalloc-prof binary check, heap 5/5, workflow 14/14, SQL 29 plus one SQLite integration, and admin API 30/30 (private `serving-edge-20260921/private-rearm-native-verification.md`). Synchronized SQL opt-in passes 34 tests; root's combined control/heap/collector/report run passes 107 tests, and subsequent parser group-stop hardening passes 15/15 heap tests. GNU timeout supplies the independent parser deadline; native completion, collector completeness, stopped process-group work and reaped watchdog remain distinct. An actual diagnostic executable, isolated native dump pair, synchronized live receipt and controlled scaling remain outstanding. Active household, specimen and conductor pin are untouched; habit remains RED.

2026-09-21 live isolated-heap delta: timer-enabled binary `c3890625c25230c028a1c4d4557125d5c4ee7a7ba0e04b7e34fba6295488dd28` built with heap 8/8 and admin API 31/31; `heap-canary-native-06/witness.json` records CLI exit 0, two bounded native dump completions with matching identities/EOF, parser acceptance, fresh namespace checks and confirmed cleanup. Both are explicitly empty sampled profiles, not zero process heap or hot-path attribution. Five preserved failed attempts exposed and regression-tested allocator-variable, relay-policy, request-budget and native-format mismatches (private `serving-edge-20260921/TELEMETRY-INTEGRATION-CHECKPOINT.md`). Earlier assembled TypeScript tests passed 155/155; latest launcher tests pass 13/13. Durable per-phase heap binding, synchronized live workflow/SQL evidence, qualified CPU/I/O and controlled scaling remain open. Household, specimen and pin untouched; habit remains RED.

2026-09-22 isolated re-arm/binding delta: `serving-edge-20260921/heap-canary-native-09/witness.json` records CLI exit 0, SQL generations 1/2 and workflow generations 1/2 with exact process/admission and enclosing resource brackets, normal expiry, two bounded heap completions and confirmed cleanup. Native per-phase heap receipts persist and rebind to final artifact hashes; the after-profile contains sampled allocations. Combined telemetry tests pass 189/189. Native producer-format compatibility, early-timer re-arm and bounded private rejected-summary retention have regression tests and independent review. This is a quiet isolated lifecycle proof, not workload attribution or controlled scaling; all coverage flags remain false. Whole-A2O checks have unrelated shared-tree errors, and the operator-authorized one-file external story review returned insufficient provider credits without a verdict. Active household, specimen and pin untouched; habit remains RED.

2026-09-22 discovery/exercise delta: package-owned `runtime-performance` skill projected to Claude, Codex and Antigravity, with high wall-clock/resource-pressure/watermark triggers and the existing CLI as its only reporting workflow. Read-only exercise of `serving-edge-20260921/cpu-io-network-15s-corrected/resource-profile.json` returned report exit 0 and all-coverage check exit 2; Matthew concentrates about 59% of CPU and 86% of net RSS growth, a drilldown target rather than a threshold breach or RCA. All new package/projection checks pass; unrelated root CLAUDE.md drift remains, generic Codex validation rejects the repository-generated governance field, and compose-graph provenance is pending the missing eprfs-agent binary. Skill discovery is not scheduled CI capture or automatic signal dispatch. Habit remains RED.
