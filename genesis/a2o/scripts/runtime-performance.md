# Runtime performance reporting

For agent-led diagnosis, invoke the package-owned `runtime-performance` skill (for example,
`$runtime-performance` in Codex). Its discovery description covers slow wall-clock time, CPU/memory
pressure, I/O/network watermarks and regressions. It uses the commands below to turn an existing
capture into ranked findings and explicit evidence gaps. It is not a scheduled monitor or Jenkins
stage; automatic signal delivery and recurring CI capture require separate verified wiring.

`runtime-performance.ts report|check|compare|trend` reads bounded `resource-profile.json`
captures and writes only to stdout. These read commands never call or reconfigure a mesh.
The separate, explicit `arm` command requests a bounded diagnostic generation on an already
opted-in private conductor. The separate, explicit `heap-canary` command creates and owns one
fresh offline disposable conductor; reports and checks never launch it. Neither operation creates
a results registry. Run these commands from `genesis/a2o`:

If `pnpm` is absent but workspace dependencies are installed, replace `pnpm exec tsx` with
`node --import tsx`. No install or rebuild is needed merely to read an existing capture.

```bash
pnpm exec tsx scripts/runtime-performance.ts report \
  --input /absolute/capture-directory --json

pnpm exec tsx scripts/runtime-performance.ts report \
  --input /absolute/resource-profile.json --perf /usr/bin/perf

pnpm exec tsx scripts/runtime-performance.ts compare \
  --baseline /absolute/baseline --candidate /absolute/candidate \
  --max-regression-percent 10 --json

pnpm exec tsx scripts/runtime-performance.ts trend \
  --input /absolute/run-1 --input /absolute/run-2 --input /absolute/run-3

pnpm exec tsx scripts/runtime-performance.ts check \
  --input /absolute/capture-directory --perf /absolute/perf \
  --require-coverage all --json

pnpm exec tsx scripts/runtime-performance.ts report \
  --input /absolute/capture-directory \
  --sqlx-log /absolute/bounded-sqlx-log \
  --workflow-events /absolute/bounded-diagnostic-jsonl \
  --network-before /absolute/network-before.json \
  --network-after /absolute/network-after.json --json
```

External evidence paths are explicit and bounded; the reporter does not scan directories or read
an indefinitely growing runtime log. Supply a bounded capture segment, not the household's entire
append-only log. Slow SQL records are threshold-selected observations; diagnostic JSONL must match
the supported event version. Network snapshots without observation timestamps and matching native
process identities remain provisional watermarks, not verified traffic rates. Attaching files never
silently upgrades missing attribution into accepted coverage.

`check` is the fail-loud instrumentation acceptance command. It exits 2 (`refer` / `skipped`)
when required evidence is missing, 1 (`refuse` / `failed`) when available evidence is inadequate,
and 0 only when every requested category is directly evidenced. Missing evidence takes precedence
when a capture also contains measured insufficiencies; individual categories remain visible.
Use `all` for the full RCA bar, or an explicit comma-separated subset of `cpu-leaf`, `cpu-caller`,
`method-counts`, `method-durations`, `method-outcomes`, `method-timeouts`, `workflow-runs`, `workflow-triggers`,
`sql-timing`, `network-watermarks`, `heap-attribution`, and `io-attribution` for a named partial check.
A partial check is not a complete telemetry acceptance. CPU caller coverage must reach 75% for
every requested peer; a readable hot-function ranking alone does not satisfy that requirement.
Method metrics describe instrumented conductor attempts, not invocation counts for every native
function. Timeout attribution requires an exact typed websocket-deadline counter for every observed
endpoint/zome/function/class dispatch series; neither generic errors, outer local-budget drops, nor
abandoned callers supply it. Process RSS/I/O totals cannot substitute for heap/I/O attribution. A
usable I/O trace requires one identity-matched profile per peer and at least 75% resolved leaf and
caller-stack coverage. Slow-only SQL records cannot establish complete statement timing. Current saved household evidence does not
pass `--require-coverage all`.

The implementation is not finished merely because that command exists. Remaining end-to-end
acceptance requires a rebuilt, identity-recorded runtime; matched captures across the household
states; complete per-cell workflow and statement attribution; actual heap attribution and a
qualified `io` or `cpu-io` capture;
two controlled workload points per claimed scaling law; and a runnable runtime regression check.
Neither unit tests nor an adapter that correctly reports missing evidence satisfies those legs.

### Disposable offline heap canary (live lifecycle verified; attribution pending)

`heap-canary` is the only supported launcher for the private native heap control. It accepts fixed
artifacts, not a command, shell arguments, environment additions, PID, state directory to reuse, or
Admin API address. Supply paths and lowercase SHA-256 values from the same approved private build
receipt, and choose an absolute normalized root that does not exist:

```bash
pnpm exec tsx scripts/runtime-performance.ts heap-canary \
  --root /absolute/new-private-canary-root \
  --nonce UNIQUE_CANARY_NONCE \
  --lifetime-ms 120000 \
  --observation-ms 5000 \
  --dump-timeout-ms 10000 \
  --holochain /tmp/private-rearm-target/release/holochain \
  --holochain-sha256 c3890625c25230c028a1c4d4557125d5c4ee7a7ba0e04b7e34fba6295488dd28 \
  --happ /projects/elohim/genesis/local-dev/household-dowell/artifacts/elohim.happ \
  --happ-sha256 1183ad2751f5e513a4802f7bab07e1a2e9ea5aeb6223251279386c232099a524 \
  --jeprof /tmp/private-rearm-target/release/build/tikv-jemalloc-sys-2ed0c57083baa733/out/build/bin/jeprof \
  --jeprof-sha256 b481b296a6149c3eb269c6f5483b479ff3b2736789b854b404320ceab06b6fe6
```

That is the accepted September 21 artifact tuple and timing shape. The native build/source receipt
is `reports/recovery/serving-edge-20260921/private-rearm-runnable-build.md`; the successful redacted
lifecycle witness is `reports/recovery/serving-edge-20260921/heap-canary-native-06/witness.json`.
Choose a new root and nonce for every invocation; never reuse `heap-canary-native-06` or any other
attempt directory. Re-hash and replace every path/hash pair together when using a later approved
build or bundle. The command reads the named source bundle only to make a bounded private snapshot;
it does not install into or otherwise modify the household that stores that file.

`node --import tsx scripts/runtime-performance.ts heap-canary ...` is equivalent; the worker uses
the repository's resolved `tsx` loader rather than inheriting arbitrary parent Node arguments.
All eleven base flag/value pairs are required exactly once. The only optional pair is the explicit
`--verify-controls true` private-control smoke described below; `false`, a bare flag and arbitrary
additional flags are refused. The nonce is 1-64 ASCII letters, digits, underscores or hyphens.
Lifetime is 1-900000 ms, observation is 1-600000 ms, and dump timeout is a whole number of seconds
from 1000 through 30000 ms. The request must also satisfy
`30000 + observationMs + (2 * dumpTimeoutMs) <= lifetimeMs`; the 30000 ms term is bounded
preparation before the baseline phase. Each approved input artifact is limited to 512 MiB. Each
heap FIFO persists at most 32 MiB plus one observed but unpersisted overflow sentinel, and overflow,
deadline, invalid EOF/native-completion pairing, parser failure, or unresolved cleanup prevents a
successful lifecycle result.

The launcher snapshots and re-hashes the three approved artifacts into its private root, binds the
running conductor's PID/start ticks and executable device/inode to that snapshot, and executes no
caller-supplied argv or environment. It starts the entire worker under fresh Linux network, mount
and PID namespaces with private mount propagation, `--kill-child`, a fresh `/proc`, and the worker
as PID 1. Only loopback is raised. Admin port 4444 and the deliberately dead bootstrap/relay target
`127.0.0.1:9` exist only inside that namespace: bootstrap uses HTTP, while relay uses HTTPS because
the production transport correctly refuses a plaintext relay. The conductor uses fresh state below
the new mode-0700 root and exactly six environment keys by default. `MALLOC_CONF` and
`_RJEM_MALLOC_CONF` both carry the identical fixed value
`prof:true,prof_active:false,prof_gdump:false,prof_final:false,lg_prof_interval:-1`, covering the
approved unprefixed allocator and a prefixed variant without inherited allocator policy. The other
keys are `PATH=/usr/bin:/bin` and the native canary root/lifetime/dump-deadline tuple. The one
30-second preparation budget shrinks across agent-key generation, app installation and enablement
and is passed to both each native client request and its outer cancellation timer.
Do not point this command at the canonical household, a preserved specimen, a copied conductor
root, or public bootstrap/relay infrastructure.

To exercise the private SQL/workflow controls on that same disposable conductor before app
provisioning and before either heap phase, append exactly:

```text
--verify-controls true
```

This opt-in adds only the fixed mode-0700 SQL/workflow directories, both startup durations set to
zero, and `RUST_LOG=holochain::diagnostics::workflow=trace`; callers still cannot add environment or
paths. It sequentially admits SQL generations 1 and 2, then workflow generations 1 and 2, using
distinct private nonces and two-second native windows. A one-second resource bracket is taken inside
each active gate, after which the existing bounded artifact readers require the returned basename,
admission tuple, process identity, monotonic enclosure, strict terminal parse and same per-family
producer. Every generation must reach its terminal before the next admission. Refusal, timeout,
missed enclosure, incomplete terminal or changed producer stops preparation; an unknown transport
outcome is never retried. All four windows and later provisioning share the existing 30-second
preparation budget—this flag does not widen lifetime, output or cleanup bounds.

Live verification on September 22: `reports/recovery/serving-edge-20260921/heap-canary-native-09/witness.json`
records an accepted lifecycle pass using the artifact hashes above, `--verify-controls true`,
120000 ms lifetime, 5000 ms observation and 10000 ms dump deadlines. All four control generations
bound and expired; both heap dumps and their private native receipts completed with confirmed cleanup.
The baseline had no sampled allocations; the after-profile contained samples. This proves the isolated
capture lifecycle only, not representative workload coverage or scaling. The preserved approved inputs
are also available under `heap-canary-native-06/approved-artifacts/` as `holochain`, `canary.happ` and
`jeprof`, avoiding reliance on the temporary build directory. Always choose a new output root.

If the parent refuses a worker summary, `canary.rejected-worker-evidence.log` retains its exact bounded
stdout privately (exclusive mode 0600, at most 1 MiB plus 512 bytes), with a finite parse/validation
reason. Treat those bytes as rejected evidence, never as a reconstructed passing witness.

The same redacted witness gains `observed.controlVerification` only when the flag is present. It
records exact family/generation order and binding states but no nonce, producer, path, event detail
or raw artifact. Its `coverageEligible` remains false: a quiet two-generation result proves bounded
admission, re-arm, terminal persistence and identity/window binding only. It is not SQL/workflow
workload coverage, cell coverage, CPU attribution, heap attribution or a scaling point. The opt-in
is source-verified by the focused launcher suite; retain a live result only after the root-owned
isolated smoke completes.

Native stderr is continuously drained without an accumulating buffer or child backpressure. Its
first 64 KiB is retained only at `<root>/canary.stderr.log`, mode 0600; overflow replaces the tail
with a truncation marker and all further bytes are discarded. A separate mode-0600
`<root>/canary.preparation.log` is capped at 4 KiB and contains only bounded preparation stage and
timeout/refusal/cancellation classifications. Neither private log path nor its contents enters the
stdout witness. Preserve a failed root for diagnosis; do not reuse or automatically retry it.

The whole disposable conductor is independently bounded by GNU `timeout`; each admitted native
dump has its own 1-30 second monotonic kernel `SIGKILL` deadline. FIFO EOF, native completion,
identity, byte bounds, and a zero-exit `jeprof` parse are checked separately. Parser and outer-worker
completion additionally require the direct watchdog to be reaped and no executing process-group
members to remain; the fresh PID namespace contains detached inner groups when its init exits.
An unresolvable stop/reap state is reported as incomplete evidence rather than success.
An empty jeprof stdout is accepted only after the pinned tool exits zero with safe-stop evidence and
the bounded raw artifact starts with `heap_v2/<decimal>` plus the exact zero sampled aggregate
`t*: 0: 0 [0: 0]` (horizontal indentation is allowed). Its explicit parser reason is
`empty-sampled-profile`. A nonzero raw aggregate, malformed header, nonzero tool exit, or arbitrary
empty output remains invalid; the general `parseJeprofText` contract is not relaxed.

The one stdout value is a bounded redacted `CheckWitness`; private paths, nonce, producer identity,
raw dumps and parser scratch are not projected. Its launcher and lifecycle both state
`coverageEligible: false`. Even an `outcome: "passed"` means only that this offline disposable
pipeline smoke completed within its declared bounds. App installation/enablement happens during
preparation and is excluded from the measured interval. Profiling activates immediately before the
baseline dump, so the pair describes sampled retained allocations since activation, not the full
startup heap. Dead loopback relay/bootstrap targets may create retry behavior unlike a live mesh.
This result is neither household idle evidence, a controlled workload/scaling point, a full-heap
measurement, proof of the original bottleneck, nor satisfaction of `heap-attribution` or
`--require-coverage all`.

Attempt 06 on 2026-09-21 exited 0 and retained an exact redacted witness. Its before artifact was
58,153 bytes and its after artifact 58,315 bytes; both had EOF, matching native identity and native
completion, no overflow, parser safe-stop evidence and `empty-sampled-profile`. The requested
5,000 ms observation measured 5,000.520204999999 ms. The native watchdog was reaped, native exit
was confirmed, and both parser groups had zero executing members. Fresh PID/network namespaces and
the fingerprinted private snapshots were also verified. This proves the bounded offline lifecycle,
not sampled allocation attribution: both profiles reported zero samples since activation, which is
not zero process heap. App provisioning remains outside the measured interval.

The broader September 21 checkpoint passed 155 tests across 15 suites and full A2O TypeScript
checking; subsequent final launcher and private-control opt-in corrections passed its 15/15 focused
suite. Tests include both
documented TypeScript invocation forms, real namespace/PID-1 teardown, OS deadline behavior while
the Node event loop is blocked, TERM-masked and overflow cases, parser cleanup, native stderr caps,
preparation budget propagation, and strict empty-sampled admission. The native build separately
passed heap 8/8 and Admin API 31/31. These tests and the successful smoke still do not make a
household, controlled-workload or scaling receipt.

One binding gap remains even for the successful pair: the trigger validates each native receipt's
producer, generation, exact process identity, dump deadline and monotonic bracket, but the launcher
does not persist the full per-phase native `startedMonotonicMs`, `finishedMonotonicMs` and
`deadlineMonotonicMs` receipt. The redacted lifecycle witness therefore cannot reconstruct the
native before/after windows for later cross-source matching. Until that receipt is durably bound—and
a nonempty sampled pair supplies meaningful rows—the run grants no `heap-attribution` coverage.

### Private re-arm (native checks pass; live integration pending)

```bash
pnpm exec tsx scripts/runtime-performance.ts arm \
  --admin-url ws://127.0.0.1:PORT --family sqlTiming \
  --nonce UNIQUE_CAPTURE_NONCE --seconds 30
```

Use `--family workflow` for workflow diagnostics. All four arguments are required; only numeric
loopback WebSocket addresses are accepted. This is an explicit runtime operation, not a report
read. Do not run it against a household another operator owns. Old binaries and conductors
without startup authorization refuse; this command does not enable them or change logging levels.
Native startup authorization is required; focused native API, SQL and workflow tests have passed.
A rebuilt profiling-enabled runtime and live end-to-end receipt remain prerequisites for acceptance.

Exit 0 means **admitted**, not complete or healthy. The response identifies a private artifact
basename and producer generation; it never grants coverage. Read the completed artifact through
the existing capture/report path and require its native identity/window evidence. A refusal exits 2. A transport timeout has an unknown native outcome and is never automatically retried: inspect
the authorized output directory before any further request.

For synchronized SQL capture, opt in with
`capture ... --arm-sql-timing matthew=/absolute/startup-authorized-private-dir` (repeat for
at most eight configured peers). The directory must already be canonical, owned and mode 0700.
Do not also supply `--sql-timing` for that peer. Fingerprints and preliminary metrics/network
observations precede arming; process and admin-listener identity are checked around admission,
and the returned artifact is descriptor-pinned before the resource window begins. The native
window adds two seconds for bookkeeping; overrunning that margin produces incomplete evidence,
not a silently shorter measurement. A partial multi-peer failure closes clients/descriptors and
fails the capture; already-admitted native windows expire without a cancellation claim.
Focused tests cover this orchestration; the real native synchronized capture remains pending.

### Fork half committed; first live admission receipt (2026-09-23)

The conductor-side instrumentation this runbook has described as "source implemented; native
verification pending" is now a fork commit (`915acf6bc` on
`int/2026-09-23-diagnostics-throttle-perf`, parent `25dd2d0be`), no longer a dirty working tree, and
the branch also carries the receive throttle and the two perf branches that were unpushed. The
household's three conductors ran the production-feature release build of that branch with
`HOLOCHAIN_SQL_DIAGNOSTICS_DIR` authorized; `arm --family sqlTiming --seconds 30` against
matthew's admin websocket was admitted and the conductor wrote
`sql-timing-g01-<nonce>.jsonl` (mode 0600, kernel process witness, HMAC statement identities, no
statement text). Workflow and heap families were not armed live. The handoff below is otherwise
unchanged: admission plus a terminal artifact is not coverage, and `check --require-coverage all`
remains RED until a matched capture accompanies it.

### Current verification handoff (2026-09-21)

An earlier source tranche recorded 129 focused telemetry tests across 12 suites and a full A2O
TypeScript pass. The new collector can retain an already-open native SQL JSONL
descriptor, then save a private bounded artifact and verify its process identity and
monotonic interval against both complete resource scans. Native verification has since passed
SQL 28/28, SQLite-worker integration 1/1, an initial Admin API 30/30, and workflow 22 tests with one
pre-existing ignored test. The later integrated checkpoint supersedes that test count with 155
tests across 15 suites, and the timer-enabled native build passed heap 8/8 and Admin API 31/31.
These remain source and isolated-canary evidence, not a rebuilt/live household receipt.

Add `--sql-timing matthew=/absolute/private-native-sql.jsonl` to `capture` (repeat for
other configured peers, at most eight). The file must already exist before capture;
the collector does not arm diagnostics or restart a conductor. It waits at most five
seconds after the final resource/metrics observations for the terminal witness, saves
at most 2 MiB per source with mode 0600/create-new, and exits nonzero on binding issues.
`report` re-reads `<capture>/matthew.sql-timing.jsonl` and recomputes the binding rather
than trusting stored flags. To render statement/site rows too, add
`--sqlx-log /absolute/capture/matthew.sql-timing.jsonl` to `report`.

Binding requires PID, start ticks, boot ID and executable to agree, plus a native
monotonic interval enclosing the start/end brackets of both resource scans. Missing
legacy fields remain unavailable. Enclosure does not make aggregate SQL timings
coextensive with a shorter CPU window, prove continuous instrumentation readiness,
or establish complete SQL coverage. `check --require-coverage all` remains RED.

The workflow reader now validates start/close controls and detail counters, preserves
quiet expiry, and distinguishes declared right-censored runs from missing records.
Rates use the active gate duration, not a late terminal's emission delay or CPU time.
Whole strict gates imported from a log remain provisional even when outside the report
window. Startup-only gates cannot prove settled behavior; bounded re-arming remains
an implementation prerequisite for the controlled scaling experiment.

Private receipts: `serving-edge-20260921/telemetry-binding-lifecycle-tests.log`,
`FIFO-CAP-PROBE-RECEIPT.md` and `SCALING-PREFLIGHT.md`. The earlier FIFO experiment verified a
32 KiB persisted-byte cap, not a production 32 MiB capture or a cancellable jemalloc dump; that
dated result remains historical evidence. The later disposable launcher enforces the 32 MiB cap
and native kernel dump deadline, and attempt 06 completed a real bounded pair as described above.
Its zero sampled aggregates still provide no allocation attribution. The live read-only census at
19:14Z confirmed 95 running cells across 19 unique running agent keys; it is a denominator
observation, not a matched scaling result.

The diagnostic conductor was launched briefly on September 21 and then restored. The private
`serving-edge-20260921/conductor-only-telemetry-20260921T133521Z/` directory contains the actual-host
restoration witnesses and `WORKFLOW-STARTUP-FINDING.md`. All three storage health probes returned
200 with live zome paths after restoration; original storage and pinned conductor hashes were
verified. `ALLOW_COORDINATOR_UPDATE=false` deliberately remains on storage to avoid an automatic
coordinator change during profiling. The mesh lease was released.

That startup-only window contains 2,449 paired workflow runs over 78.257 seconds: publish is
cell-bound (95 producer-local cell instances), while validation/integration are DNA-bound
(15 producer-local DNA instances). Publish's 390.741 summed wall seconds overlap and are **not CPU
seconds**. No perf capture accompanied this window. Native identity/window binding remains
provisional; this is a profiling lead, not a scaling law.

The separate `serving-edge-20260921/cpu-io-network-15s-corrected/` capture verifies native network
watermarks on all three peers, but still fails full coverage: sparse CPU samples and lost I/O
events prevent acceptance. Its command did not request metrics; `metrics: []` there is not proof
of an exporter failure. Future capture commands must explicitly name all three metrics URLs.

Do not adopt the combined storage candidate with SHA beginning `2a4f5993`: it includes concurrent
story 1.4b migration/write behavior, outside the telemetry experiment. A telemetry-only source
export receipt is `serving-edge-20260921/BUILD-elohim-storage-telemetry-only-export.md`: its
`868e7c8a9c1ac272c87743aab221fa23747556296f256ccd6fcc6843b2a20c63` candidate passed the focused
eight-test `hc_client::attribution_tests` module and the dual-feature build. Its earlier
not-adopted status is superseded by
`serving-edge-20260921/storage-telemetry-live-20260921T142306Z/LIVE-INSTRUMENT-VERIFICATION.md`:
it was adopted for one guarded instrument-verification window and then restored. That run proved
method counts/durations and typed timeout series, but did not make the full gate green.

The newer
`serving-edge-20260921/storage-sql-live-20260921T151100Z/LIVE-SQL-INSTRUMENT-VERIFICATION.md`
records a guarded live adoption of storage SHA `14bad655a8e4acbe592390810ca263a1f9e6c4f71f3e9e54c0e2f0c9efa1a700`
with SQL-enabled conductor SHA `a0aa86b0364e05cb7f7e2b4405868ee06c3d5c0953b51107beffdbe940305103`.
All three peers exposed method counts, durations, zero-materialized outcomes, and typed timeout
series. Each private SQL logger reached its 10,000-event cap during startup in under five seconds,
so those source-site/HMAC rankings are provisional and incomplete, not complete SQL coverage or a
causal join to the later CPU window. The isolated five-second smoke at
`serving-edge-20260921/isolated-sql-smoke-20260921T145504Z/RECEIPT.md` separately recorded normal
expiry with 37 completed events and 14 capture-local statement identities. Its completeness is
only for logger events delivered during that isolated gate; it is not household coverage.

The latest live CPU capture resolved leaf symbols but had poor caller coverage (James 66.67%,
Jessica 1.68%, Matthew 13.21%). Its separate one-second I/O capture was clean but contained only
one successful eight-byte write per peer, enough to prove the stop/parser shape but not characterize
cost. Cleanup restored storage SHA `a19f…`, pinned conductor SHA `283909…`, unchanged configs, and
`ALLOW_COORDINATOR_UPDATE=false`; all storage health probes returned 200. Jessica's final zome
probe was live, while Matthew and James were serving without anchored content and therefore
inconclusive rather than failed. Full heap capture, same-window cross-source attribution, matched
household states, controlled state comparisons, and scaling evidence remain pending.

Specimen safety: the preserved
`/projects/elohim-specimens/household-dowell-20260920-conductor-cpu` is **not relocatable merely
by setting `MESH_DIR`**. Read-only inspection found canonical-household paths in `conductors/.hc`,
each conductor's `data_root_path` and `keystore.lair_root`, and each Lair config's `connectionUrl`,
`pidFile`, and `storeFile`. Explicit mesh roots skip migration; resume does not rewrite these
paths. Do not run the specimen README's start command. Preserve the original, prepare a separate
working copy after mesh handover, and structurally relocate and validate every operational path
there before preflight/start. Preserve credential/query material and historical logs; never apply
a global replacement or `MESH_RESET=1`. Directory relocation is not isolation proof.

The saved large household and fresh 14-name household are candidate scaling points, not yet a
controlled pair: use the same measured binaries, transport/diagnostic settings and settling
criteria, and count actual apps, agents and running cells separately. The specimen records an
older storage binary than the acceptance household; otherwise a binary/state change could be
misreported as a cell-count scaling law. Do not inspect or profile the active acceptance runtime
until its owner explicitly releases the mesh.

Full `--require-coverage all` remains **RED**. The reader and bounded adapters are exercised, but
there is no matched real heap dump pair and no household capture proving all categories. The typed
websocket-timeout observer for all three storage roles passed independent static review and native
verification: attribution 7, conductor-call metrics 2, typed timeout 1, dropped-call censorship 1,
and pprof/cap/gzip 15 tests passed. The manifest's storage Clippy shape, `cargo clippy -j1 -- -D warnings`,
also exited 0. Logs are private `serving-edge-20260920/storage-{attribution-tests,conductor-call-tests,typed-timeout-test,dropped-call-test,pprof-tests,clippy}.log`.
Earlier storage settings and telemetry slices recorded 41 and 29 passing tests respectively.
A conductor test compile ran for 40m26s and exited 101 on pre-existing
optional `instrument` errors. After the operator released Cargo, one-job storage verification
completed under `codex-telemetry-native-final`. The authorized conductor retry without that feature
passed: the queue module has 15 passing tests (including all seven new telemetry tests) and one
pre-existing ignored test. The one-job diagnostic release build also passed; its binary SHA-256 is
`bc4064d9a85c66a02e1e391c417cf8e4b15bc7762e2c83ebee1c75961bb2290b`.
The persistent `serving-edge-20260920/conductor-diagnostic-build-receipt.txt` records source,
patch, flags, commands and logs. The September 21 launch/restoration above supersedes its earlier
not-yet-launched status; the production pin is unchanged. The later `a0aa…` SQL-enabled conductor
was also launched only for the guarded verification described above and then restored. Frame-pointer
or larger DWARF settings do not prove assembly/JIT caller recovery.

After explicit mesh release, one read-only 15-second CPU/I/O/network capture was taken in
`serving-edge-20260920/telemetry-ssr-confounded-20260920T233706Z`; the mesh lease was released
immediately afterward. Its `READOUT.md` records commands and exits. This household still contained
the independently diagnosed bloated 285 MB SSR bundle: the capture is not a healthy baseline or
controlled scaling point. Collection exited 0, but the all-coverage check exited 2: I/O event caps,
network connection failures, sparse CPU samples and unbound metrics prevent complete attribution.
The operator owns the SSR correction and subsequent cleanup/re-stage window; do not delete live
renderer scratch or duplicate that fix.

Still unimplemented or unproven are full SQL statement identity, runtime causal binding, matched
captures across all three household states, and scaling measurements. The heap tuple is descriptive
only until native identity and real dumps are matched. A separate ignored source export currently
duplicates a build manifest, so no owning gate is claimed green from this work.

`--json` emits the existing canonical Verdict wire shape. Human output is Markdown. Report and
trend are descriptive, so their check is `skipped`, decision is `refer`, and exit is zero after a
valid read. They do not turn an idle window into acceptance evidence. Compare requires an explicit
local threshold; it exits 0 for a measured permit, 1 for a measured regression, and 2 when captures
are invalid or not comparable. No default threshold is implied or ratified.

Percentage change is defined only for a positive baseline. A zero baseline followed by zero passes;
zero followed by a positive value is refused by an explicit absolute zero-baseline guard, not
misreported as a percentage regression.

Comparison requires the same non-empty `telemetry.cohort`, identical peer sets, stable boot/clocks,
valid process/binary witnesses, and no counter reset. A changed binary SHA is reported because it is
usually the intended comparison dimension; it is not a rejection. Older captures without a cohort
remain useful for `report` but are not comparable. Each JSON input must be a regular file no larger
than 128 MiB (checked before and after reading); trend accepts at most 32 inputs and never scans
directories recursively.

CPU and process I/O counter deltas are divided by actual observed elapsed time. RSS is an endpoint and a
growth delta, never a rate. Prometheus method calls are ranked by calls/minute. Atom duration
histograms are ranked separately and are never joined to per-function call counts as if they were
method latency. HTTP status, load-shed, and timeout counters remain distinct observations.
Approximate histogram quantiles are labeled as such by the parser: they are observed upper bucket
bounds, not interpolated estimates. Metrics without `process_start_time_seconds` remain useful as
provisional observations, but explicitly lack producer-restart identity and cannot support a green
comparison verdict.

With `--perf`, the reporter invokes the exact absolute `perf` executable as a subprocess (no shell),
with separate self-symbol and caller-stack projections, each bounded to 15 seconds and 16 MiB
of output per profile. Truncated, failed,
identity-invalid profiles and profiles reporting lost samples, events, records, or chunks are
excluded with a reason. CPU percentages are weighted
by `cpu-clock` sampled period in nanoseconds. Sample count is not invocation count. Caller clusters
are shown only when stack coverage is reliable; unknown/stripped-frame coverage is reported.
Missing unwound callers do not erase resolved sampled leaf symbols. A failed caller projection
leaves the self ranking readable and names the caller-analysis issue.

`ioProfiles` ranks bounded `perf trace` observations separately. Bytes mean successful user-syscall
return bytes (files, sockets, and logging included), not physical/device I/O; failures and counts
remain separate and counts are never called method invocations. Reports omit syscall arguments,
descriptor paths, payloads, DSOs, and capture paths. `mmap`, `io_uring`, kernel writeback, and device
attribution remain explicit gaps. Use a `cpu-io` collector window when one report must evaluate both
CPU and I/O coverage against the same cohort, peer, binary, and process witnesses.

Native `networkWatermarks` come only from a collector capture made with `--network-stats`. Coverage
requires paired before/after Admin API observations for every peer, stable resource and binary
identity, resource-window bracketing, identical hashed connection membership, and monotonic
counters. Reported byte/message deltas are matched-connection transport watermarks, not physical
traffic rates. Peer keys, advertised URLs, and raw Admin API responses remain private and are not
projected into the Verdict. Arbitrary external dump files cannot satisfy this category.

For a bounded trusted-local retained-heap description, `report` and `check` accept only the complete
tuple `--heap-before PATH --heap-after PATH --heap-binary PATH --jeprof PATH`. The dump paths must
be distinct. The existing adapter snapshots bounded regular files and invokes the explicit jeprof
tool without a shell; raw dumps, binary paths, and scratch paths are never projected publicly.
Signed self bytes describe retained growth/shrinkage; cumulative bytes overlap call paths and are
never summed. Unknown absolute self bytes and omitted rows remain explicit. This tuple deliberately
does **not** satisfy `heap-attribution`: local paths and a caller-supplied binary do not prove the
capture PID, start ticks, or exact executable identity. The disposable canary now has a real
identity-matched dump pair, but both profiles contain zero sampled allocations and its full
per-phase native monotonic receipts are not durably retained. Those facts do not upgrade this
caller-supplied tuple; `--require-coverage all` therefore stays fail-loud.

The private native heap control is limited to an explicitly authorized disposable process.
Its before phase activates jemalloc sampling: the resulting pair measures sampled retained
allocations **since that activation**, not the full startup heap. A matching admin receipt proves
only that the native traversal returned; FIFO EOF, byte bounds, parser completion, and process
identity are separate requirements. The adapter checks nonce, phase, producer, generation,
kernel process identity and the request's monotonic-time bracket. It does not grant heap coverage.
The isolated launcher and bounded dump-pair lifecycle are verified by attempt 06, while useful
nonempty attribution and durable native per-phase receipt binding remain pending. Invoke this
control only through `heap-canary` with a new root, never on the household or preserved specimen.

`methodLatency` reports completed storage-to-conductor attempt histograms separately from
`methods` (dispatch counters) and `atomLatency` (storage operation histograms). A caller-drop
counter records abandoned observation, not conductor cancellation, and contributes no completed
response latency. These fields require the corresponding exporter instrumentation; old captures
cannot acquire timings retrospectively.

Trend Pearson correlation is emitted only for at least five unique, timestamped, proven
non-overlapping comparable numeric windows with two non-constant axes. Duplicate or overlapping
captures do not contribute pairs. Its pair count is always shown and it is descriptive association,
never causation. Heap attribution, qualified I/O stack attribution, per-method timeouts, and workflow trigger counts
remain explicit coverage gaps when telemetry does not provide them. Raw perf data can contain
process-memory bytes; keep captures private and share reviewed summaries, not the artifacts.

To use the existing collector through the same entry point, pass its arguments unchanged:

```bash
pnpm exec tsx scripts/runtime-performance.ts capture \
  --mesh-dir /absolute/mesh --seconds 60 --output /absolute/new-directory --mode metrics \
  --cohort household-cast-v1 \
  --metrics-url storage=http://127.0.0.1:8090/metrics
```

Use the same exact cohort bytes only when hardware, workload, population, cadence and diagnostic
settings are genuinely the same. Record filter levels, diagnostic windows, sampling settings,
allocator and build features beside the capture. Measure instrumentation overhead with matched
enabled/disabled observations before treating diagnostic timings as production costs.
Stable boot and clock requirements apply within each capture window; comparisons do not
require two runs to share a host boot.

`arm` explicitly changes a diagnostic window. `capture` observes live processes and forwards to
`resource-profile.ts`; reporting and comparison never make live or remote calls. There is no
permanent watcher or automatic CI acceptance wiring in this tool.

## Agent reconciliation loop

Start with saved evidence; do not claim the mesh or rebuild just to read a report. The first
household exercise is reproducible from `genesis/a2o`:

```bash
node --import tsx scripts/runtime-performance.ts report \
  --input reports/recovery/serving-edge-20260919/resource-restored-cast \
  --perf /projects/elohim/genesis/a2o/reports/recovery/serving-edge-20260919/tools/perf-7.0.0-31/perf
```

Add `--json` for the canonical machine-readable verdict. The private evidence directory also
contains `resource-restored-cast/FIRST-runtime-performance-report.md`, the reviewed interpretation
of this exercise.
These local artifacts are not distributed with the repository; on another machine supply a
locally available capture and matching perf executable instead.

1. Read `comparable`, `comparabilityIssues`, `coverageGaps`, and profile exclusions before ranks.
   Report exit 0 means it was readable, not that performance passed.
2. Separate observed cost from attribution: CPU samples are not calls; wall-clock hold is not CPU;
   RSS is not heap ownership. Unknown symbols are missing evidence, not a cheap method.
3. Choose the smallest measurement that closes the leading gap. Preserve matching binaries and
   build IDs for symbol recovery before requesting another CPU recording. Never repair an old
   capture by inventing a cohort or silently changing its recorded witnesses.
4. For a fresh window, coordinate `berth claim mesh` first and `berth claim cargo` only if a build
   is required. Honor the operator's one-build-at-a-time rule even if berth advertises capacity
   greater than one: inspect `berth who cargo` and do not start while another session holds it.
   Record hardware, workload, population and cadence in the cohort rationale. Use
   bounded collection and a completion callback; do not repeatedly poll during a fixed window.
5. Compare only genuinely matched windows with an explicit operator-selected threshold. An exit
   2 is an evidence gap to resolve, not a regression or a pass. Use `trend` only after accumulating
   independent windows; fewer than five usable windows cannot support its correlations.
6. Record the command, capture path, binary identities, measured finding, excluded evidence and
   next measurement beside the capture. Append the evidence delta to
   `.epr-meta/runtime-performance.habit.md` at the repository root, then regenerate
   `genesis/manifests/habits.yaml` with `.claude/scripts/habits-project.py`. Keep the habit RED
   while native attribution or runtime acceptance remains unproved.

The next agent should need the report and its named next measurement, not the previous chat.
Do not infer a design remedy from a hot library name alone: obtain its caller/workflow context
and a controlled second data point before claiming a scaling law.

### Native metric verification frontier (2026-09-20)

The storage instruments now have focused native verification: 41 runtime-settings tests and
29 storage tests passed (diagnostics, pprof, conductor-call guards, DB labels, admission placement
and process identity). This supersedes the earlier interrupted-test frontier. They are still not
evidence from a rebuilt household binary; do not scrape an old binary expecting the new series.

After the cargo lane is empty and you have moored/claimed it with a unique `BERTH_SESSION`, run
from the repository root (pool path is this workspace's resolved storage slot):

```bash
env RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev \
  CARGO_BUILD_JOBS=1 cargo test --manifest-path elohim/elohim-storage/Cargo.toml \
  --lib process_start_identity_is_registered_once_and_stable

env RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev \
  CARGO_BUILD_JOBS=1 cargo test --manifest-path elohim/elohim-storage/Cargo.toml \
  --lib conductor_call

env RUSTFLAGS='--cfg getrandom_backend="custom"' \
  CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/elohim/elohim__elohim-storage/dev \
  CARGO_BUILD_JOBS=1 cargo test --manifest-path elohim/elohim-storage/Cargo.toml \
  --lib completion_timing_starts_only_after_admission
```

Check each exit status before proceeding. The admission-placement test is source-structural;
the guard tests exercise outcome/drop behavior but do not replace a live call test. Release the
cargo lease after testing. The full owning gate is `just gate elohim-storage`; this exercise found
an unrelated pre-existing rustfmt failure at `src/p2p/acquisition.rs:652`, left unchanged.
Once that owner resolves it, rerun the full gate. A subsequent explicit binary build, coordinated
mesh restart and bounded scrape are still needed before comparing live method durations.

Pool-resolution trap: `cargo-pool key` incorrectly selected the parent `elohim/dev` slot for
this excluded standalone crate during verification. The gate manifest's workspace identity
`elohim/elohim-storage` is authoritative; use the `elohim__elohim-storage/dev` slot above.
Do not cold-rebuild in the parent slot or infer workspace ownership solely by walking Cargo.toml.

## Reuse the existing observability stack

Do not grow this CLI into a profiler, trace collector or flamegraph renderer. Its local job is
bounded capture orchestration, evidence validation and the canonical comparison verdict. Prefer
the following existing instruments for the actual measurements:

| Question                                                      | Existing instrument                                                     | Verified boundary                                                                                                                                  |
| ------------------------------------------------------------- | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Which conductor functions consume CPU?                        | Linux `perf` recordings and its symbol/unwind engine                    | Already exercised; leaf coverage and caller coverage are independent.                                                                              |
| Which storage functions consume CPU?                          | Storage's existing pprof-rs `/debug/pprof/profile`                      | Default-off `ELOHIM_PPROF_ENABLED`; 1–60 second window, 100 Hz, overlapping requests refused. This profiles storage, not its conductor.            |
| Which instrumented async scope holds time?                    | Conductor `holochain_trace` `JsonTimed`/`FlameTimed` and Rust `tracing` | Deep spans require the non-default `instrument` build feature. Timed span duration is not self CPU, and async ancestry is not a native call stack. |
| How many attempts, completions, errors and abandoned callers? | Existing Prometheus registry and conductor OpenTelemetry metrics        | Keep bounded counters/histograms; do not derive exact counts from sampled traces.                                                                  |
| Which allocations remain live?                                | Existing conductor `jemalloc-prof` diagnostic feature                   | Separate heap investigation; enabling CPU tracing does not enable heap profiling.                                                                  |

Source homes: `elohim/elohim-storage/src/pprof_endpoint/mod.rs`,
`elohim/holochain-conductor/crates/holochain_trace/src/lib.rs`,
`elohim/holochain-conductor/crates/holochain/Cargo.toml`, and
`elohim/holochain-conductor/crates/holochain_metrics/src/lib.rs` (paths from repository root).
Storage and conductor filters are startup-configured; no hot-reload logging layer was found.
Conductor OpenTelemetry is presently metrics export, not distributed tracing: an OTLP span
exporter and `tracing-opentelemetry` bridge are not wired. Merely setting an OTEL endpoint cannot
turn the existing metrics pipeline into causal traces.

Storage's pprof route has no route-level authentication. The new source patch bounds the encoded
protobuf to 32 MiB and gzip response to 8 MiB, clamps direct calls to 1–60 seconds, and keeps the
singleflight lease with the worker when its HTTP waiter is abandoned. The 11 focused pprof tests
pass, including a real profile and cancellation/cap regressions; a rebuilt runtime is still needed.
These bounds do not cap the sampler/report's internal memory, and
duration, response-size and concurrency bounds are not a substitute for access control.
Do not enable it on an exposed storage listener: use an operator-isolated/trusted scrape plane,
or address those guards before wider use.

### Bounded storage diagnostics (focused native tests passed; household receipt pending)

The existing watched runtime-config registry now accepts `ELOHIM_DIAGNOSTICS_WINDOW_SECONDS`.
Its default is 0 (off). An explicit nonzero change requests one monotonic diagnostic window,
capped at 900 seconds; an unchanged value does not keep re-arming after expiry. To request a
second window, let the runtime observe 0/removal before setting the desired duration again.
Preserve all other settings when editing the operator-owned runtime-config file. This is a
runtime-file control, not a newly supported boot environment variable.

Detailed logging has a 10,000-event budget and reports dropped events. The central production
Diesel pool records query completion timings without formatting SQL or bindings. Explicit
synchronous scopes currently label the capacity reporter's resolution, measurement and upsert
sites; all other observed pool queries remain `unattributed`. A site may execute multiple SQL
statements: these labels do not establish unique SQL-statement identities. Conductor-attempt
events start after admission and distinguish returned success/error from a dropped caller.
Operations that outlive the window retain completion metrics but their detailed terminal events
are suppressed; unmatched starts remain incomplete observations, not cancelled work.
None of these source additions retroactively instrument old binaries or prove complete RCA coverage.

### Bounded conductor workflow diagnostics (source implemented; native verification pending)

The diagnostic conductor reads `HOLOCHAIN_WORKFLOW_DIAGNOSTICS_SECONDS` once. Unset, invalid or
0 disables detailed workflow events. A positive value requests at most 900 seconds from the
first telemetry opportunity, with at most 10,000 detailed events; the queue-consumer TRACE
target must also be enabled in the existing tracing filter. Use existing structured JSON output
for machine-readable capture. This is a startup control, not a hot-reload logging feature.
Workflow metrics remain separately available through the existing metrics backend.

Events distinguish attempted notifications, consumed wakeups and actual runs. Opaque cell tokens
are shared across that live cell's workflows. Run IDs and a cached producer marker permit local
pairing, but do not replace the collector's process-start/binary identity witness. Execution timing
excludes deliberate retry/backoff waits. A generic dropped future is labelled `dropped`, not proof
of cancellation. Expiry or event-cap truncation leaves an explicit incomplete-coverage witness.

The shared conductor fork also contains unrelated transport edits. Do not describe a build from
that dirty tree as the pinned revision alone. For a controlled diagnostic comparison, use isolated
pinned source plus the scoped telemetry patch; preserve the patch digest, compiler, allocator and
feature flags with the binary. Do not move the gitlink, overwrite mesh binaries, reset state, or
place another source export inside the repository's manifest walk merely to run this experiment.
The diagnostic build should retain the production encryption/Wasmer/jemalloc choices and add
frame pointers (and `instrument` when deep timed spans are needed), not silently change allocator.

For SQL, reuse connection-level instrumentation rather than wrapping every query. Storage's
resolved Diesel 2.3.11 exposes `Connection::set_instrumentation` and
`set_default_instrumentation` with query start/finish events. A thin adapter can measure duration
and outcome, but semantic method labels still need bounded scoped context. Never format the
query, bind values, connection URL or raw error into exported telemetry. Conductor centralizes
its SQLx connection options in `holochain_data/src/lib.rs`; SQLx's statement/slow-statement
logging APIs are a diagnostic candidate, but exact compatibility with the pinned 0.9.0 build
has not been compiled in this audit. Statement logs are not aggregate metrics and may expose
sensitive SQL. Reuse the available hooks without mistaking them for a ready, safe exporter.

Live correction from the subsequent household startup investigation: the running SQLx 0.9.0
conductor already emits `sqlx::query` slow-statement events at a one-second threshold without
any new configuration. The final reproducible extraction for the fixed 2026-09-20
19:16:00–19:25:28.596655Z window contains 387 events; 253 were publish-backlog count/selection
queries (superseding an earlier provisional 374/241 tally). See private evidence
`reports/recovery/serving-edge-20260920/SQL-slow-startup-observations.md`. Start by reading those
existing events. They are threshold-censored elapsed times, not all-query counts/percentiles,
and the multiplexed conductor log does not identify the emitting peer.

Upstream tools already cover the expansion path:
[tracing-opentelemetry](https://docs.rs/tracing-opentelemetry/latest/tracing_opentelemetry/)
connects Rust spans to a trace backend;
[Tokio Console](https://github.com/tokio-rs/console) diagnoses async tasks but requires its own
subscriber and `tokio_unstable` instrumentation;
[samply](https://github.com/mstange/samply) provides an existing CPU call-tree/flamegraph UI;
[Pyroscope](https://grafana.com/docs/pyroscope/latest/) supplies continuous profile aggregation.
These are reuse candidates, not installed/wired capabilities or reasons to deploy another stack
before proving one useful local recording. Keep profiles private; do not publish stack bytes or
source through a viewer by default.

### Caller coverage preflight

The saved 2026-09-20 James recording contains all 2,450 user register/stack samples, recorded with
an 8,192-byte `sample_stack_user` limit, and reports no lost samples. The bundled perf has libdw DWARF unwinding enabled;
the matching conductor binary has `.eh_frame` information. Nevertheless usable caller coverage
is only about 0.2%. Missing framework support is not established, and the reason for poor unwind
coverage is still unresolved. The old stack bytes cannot be enlarged after collection.

The subsequent saved `serving-edge-20260920/cell-startup-dwarf32` preflight already tested
32 KiB stacks: all three profiles were identity-valid and nontruncated, but caller coverage
remained unusable (James 0%, Jessica 2.459%, Matthew 0.155%). Do not repeat that experiment
as an untested hypothesis. The next experiment needs the isolated diagnostic binary built with
Rust and C/C++ frame pointers, preserving the pinned source and recording its patch/binary identity.
The optional `instrument` feature failed compilation in nine pre-existing locations; omit it
for the approved retry. The new bounded workflow telemetry and tests do not depend on that feature.
Validate actual caller coverage before collecting a full report: frame pointers do not guarantee
unwinding through assembly or JIT frames. Runtime restarts need the mesh lease; builds need an
otherwise empty cargo lane. Both lanes were occupied at the last verification checkpoint.

For that short diagnostic experiment, from `genesis/a2o`, after claiming the mesh:

```bash
node --import tsx scripts/runtime-performance.ts capture \
  --mesh-dir /projects/elohim/genesis/local-dev/household-dowell \
  --seconds 5 --mode cpu --max-bytes 67108864 --call-graph fp \
  --perf /projects/elohim/genesis/a2o/reports/recovery/serving-edge-20260919/tools/perf-7.0.0-31/perf \
  --output /absolute/new-private-directory/caller-preflight-frame-pointers
```

This command requires the diagnostic binary to have been built and deliberately installed under
the mesh lease; selecting `fp` does not retrofit frame pointers into the old executable. Read it
with `report --input <directory> --perf <same-perf>`. These five-second probes test diagnostic quality, not idle
budget compliance, scaling, or regression acceptance. The byte ceiling is per conductor; discard
truncated CPU profiles. If another session owns the mesh, prepare these commands but do not run them.
