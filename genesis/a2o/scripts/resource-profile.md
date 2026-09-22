# Bounded process resource diagnostics

`resource-profile.ts` observes conductors in an already-running local household. It never starts,
stops, reconfigures, or calls a conductor. Every mode captures per-process CPU time, RSS, and
Linux process I/O counter deltas with stable PID/start-time/executable/config identity checks.
Before the timed window it also streams each distinct conductor executable through SHA-256 and
records its device/inode. This is a verification fingerprint for detecting replaced binaries, not a
protocol content address.

## Diagnostic ladder

Use the lowest level that can answer the question, for a declared finite window:

1. **Metrics baseline:** start with `--mode metrics`. It has no sampler and establishes CPU time,
   RSS, and process I/O deltas against stable process and binary identity.
2. **CPU attribution:** use `--mode cpu` only when the baseline is over budget. Supply the exact
   `perf` executable and a required `--max-bytes` ceiling per conductor. This adds sampled
   user-space CPU stacks.
3. **User-syscall I/O attribution:** use `--mode io` for bounded read/write/fsync stack evidence, or
   `--mode cpu-io` when CPU and I/O must share the same identity/resource window.
4. **Targeted module logging:** when stacks identify a subsystem but not its decisions, an operator
   may restart the conductor with a narrow module-specific `RUST_LOG`/`tracing_override` for a
   bounded experiment. This tool does not set logging and does not implement dynamic log reload.

Do not enable global `trace` or leave high-cardinality diagnostics unbounded in production. Broad
trace formatting and I/O can perturb the workload being measured, while labels keyed by peers,
cells, operations, or hashes can grow without an operational ceiling.

Keep captures in a private, persistent evidence directory, not a tracked source directory.
DWARF stack samples can contain process-memory bytes, and logs may contain private identifiers.
Share reviewed symbol summaries rather than raw profiles; do not publish captures automatically.

```bash
pnpm exec tsx scripts/resource-profile.ts \
  --mesh-dir /absolute/path/to/household \
  --seconds 60 \
  --output /absolute/path/to/new-evidence-directory \
  --mode metrics
```

Add `--network-stats` to bracket the same window with the existing conductor Admin API's
`dumpNetworkStats`. The collector accepts only loopback admin ports declared consistently by the
household fixture and conductor YAML, then proves the listening socket inode belongs to the exact
captured conductor PID. Each connect/request is bounded to five seconds and the installed
`@holochain/client` 0.21 Node transport is regression-tested with a 4 MiB pre-decode websocket
`maxPayload` ceiling. Before/after requests are identity-checked independently. Persisted private
capture data contains aggregate counters, capture-local salted SHA-256 connection-membership
tokens, and the native identity witness needed for validation—never peer keys, peer URLs, or raw
API responses. The public Verdict strips native paths and membership tokens. Membership change,
process change, counter reset, timeout, or any
collection issue makes network coverage unavailable rather than producing a rate.

CPU mode additionally runs Linux `perf` against every identified conductor. It is deliberately
opt-in and requires an explicit executable and byte budget; permission or sampling failures are
written as errors and produce a non-zero exit rather than an empty “zero CPU” result.

```bash
pnpm exec tsx scripts/resource-profile.ts \
  --mesh-dir /absolute/path/to/household --seconds 60 \
  --output /absolute/path/to/new-evidence-directory \
  --mode cpu --perf /usr/bin/perf --max-bytes 134217728 \
  --dwarf-stack-bytes 16384
```

I/O mode uses `perf trace` for `read`, `write`, `pread64`, `pwrite64`, `readv`, `writev`, `fsync`,
and `fdatasync`. The window is at most 60 seconds, `--max-events` defaults to 10,000 and is capped
at 100,000, and its private artifact is capped at 64 MiB per conductor. `cpu-io` records separate
CPU and I/O artifacts concurrently against the same pre/post identity witness; `--max-bytes`
remains the CPU ceiling and the I/O artifact independently uses the smaller of that value and 64
MiB. A planned end-of-window SIGINT is completion; event/byte limits, lost records, or forced kill
mark the I/O profile unusable.

```bash
pnpm exec tsx scripts/resource-profile.ts \
  --mesh-dir /absolute/path/to/household --seconds 30 \
  --output /absolute/path/to/new-evidence-directory \
  --mode cpu-io --perf /usr/bin/perf --max-bytes 134217728 --max-events 10000
```

`--call-graph dwarf|fp` applies to CPU and I/O stack collection and defaults to `dwarf`. Use `fp` to test a diagnostic
binary built with frame pointers; merely selecting it cannot repair a binary that omitted them.
Frame-pointer mode records `perfCapability.callGraph = "fp"` and `dwarfStackBytes = null`,
and rejects an explicit DWARF stack budget. Sampling frequency, duration, byte watchdog and
identity checks are unchanged. Caller coverage still has to pass the report's independent check.

`--dwarf-stack-bytes` applies to CPU and I/O stack collection and accepts exactly `8192`, `16384`, or `32768`; it defaults to
`8192`. The selected value is persisted in `perfCapability.dwarfStackBytes` and in the exact
`callGraph` setting so runs with different unwind budgets are not mistaken for equivalent evidence.
Larger stacks can improve caller recovery, but also increase recording overhead and artifact size;
the frequency, duration, and per-conductor byte ceilings remain unchanged.

`--max-bytes` is a per-conductor ceiling and must be at most 1 GiB. The example permits up to 128
MiB for each conductor; on a three-conductor household, the aggregate artifact budget is therefore
384 MiB. Native `perf --max-size` is used when available. Otherwise a 100 ms watchdog treats the
value as a soft stop threshold: the artifact can overshoot during the check interval or a `perf`
buffer flush. Reaching either native or watchdog limit marks the capture truncated and invalid for
CPU attribution rather than presenting it as a complete profile.

The CPU profile is sampled `cpu-clock:u` at 49 Hz with bounded call graphs. The I/O profile is
event-driven and has no CPU sampling frequency. Reported I/O bytes are successful user-syscall
return bytes, including files, sockets, and logging—not physical/device I/O. Counts are syscall
observations, not method invocations. Arguments, descriptor paths, payloads, and raw artifacts are
never copied into the report. `mmap`, `io_uring`, kernel writeback, physical-device attribution,
and heap profiling remain unsupported.

Holochain logging is configured only at conductor startup through `RUST_LOG` or the config's
`tracing_override`; this tool never raises it. Existing conductor OpenTelemetry can be enabled at
startup with a distinct `HOLOCHAIN_INFLUXIVE_FILE` and `HOLOCHAIN_INFLUXIVE_HOST_TAG` per process.
That exporter appends forever and has no rotation, so use a new bounded-experiment file per process
and archive it after shutdown. `OTEL_METRIC_EXPORT_INTERVAL` is milliseconds. Those metrics measure
workflow completion duration by workflow/DNA; they do not provide sampled CPU or per-cell trigger
counts.
