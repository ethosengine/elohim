@e2e @deployment @resource-performance @concern:runtime-performance @wip @act:host
Feature: Runtime performance reports preserve evidence and comparison context
  As an operator or LLM investigating a local runtime budget
  I want a bounded report and trend comparison that names its cohort and threshold
  So that a regression is actionable without turning missing or incomparable evidence into a green zero

  This is a source-backed story for the local TypeScript CLI. It is deliberately @wip and
  tagged for host execution: the focused unit tests are the current proof boundary, not a live household claim.
  The report reuses the protocol CheckWitness vocabulary and does not mint a second decision registry.
  Its executable source checks are scripts/__tests__/runtime-performance.test.ts and
  scripts/__tests__/performance-prometheus.test.ts, run from genesis/a2o with
  `pnpm exec tsx --test scripts/__tests__/runtime-performance.test.ts scripts/__tests__/performance-prometheus.test.ts`.
  An operator reads a capture using `pnpm exec tsx scripts/runtime-performance.ts report --input CAPTURE --json`
  and compares two using `pnpm exec tsx scripts/runtime-performance.ts compare --baseline BASELINE --candidate CANDIDATE --max-regression-percent 10 --json`.

  CAPTURE names a local resource-profile.json or its containing directory. A valid process
  window has both snapshots, the same process identity and host boot identity at its endpoints,
  positive elapsed time measured by a monotonic clock, consistent clock ticks per second,
  finite resource counters and no counter rollback. Missing means the required capture or
  sample is absent; timed out and cancelled mean the collector recorded those failures.
  A timeout observing a server operation does not establish that the server cancelled its work.
  Process identity includes PID, kernel process-start ticks, executable and expected configuration
  path, with a matching executable fingerprint and host boot identity. A reused PID is not the
  same process. Identity is checked at both ends of the observation.

  A cohort is an operator-declared label for the workload, machine, cell population and
  operating cadence being compared. Equal labels declare comparable conditions; they do
  not prove them. Labels match by exact string equality. Comparisons also require the same
  set of peer names from the process samples, independent of ordering. A changed binary
  is shown as a change under test, not silently treated as a different workload.

  Process observations include CPU seconds over the measured elapsed interval, resident
  memory and operating-system I/O bytes. Runtime counters count actual dispatched calls;
  CPU stack samples do not count calls. A method without latency or outcome instrumentation
  is shown as unmeasured, rather than borrowing the timing of its enclosing subsystem.

  For a positive baseline, relative growth is 100 times (candidate minus baseline) divided
  by baseline, using the same units and elapsed-time-normalized rates. A regression exceeds
  the requested maximum growth; equality is allowed. Zero to zero is unchanged, while
  zero to a positive cost is an increase that cannot be expressed as a finite percentage.
  The comparison applies an explicit zero-baseline guard: any positive cost newly appearing
  from zero fails independently of the percentage threshold. The command names that guard,
  rather than claiming a percentage breach or emitting infinity as JSON.

  An assertion carries checkId, outcome, summary and observed evidence. Its outcome is
  passed, failed or skipped. An incomparable comparison uses decision "refer" with reason
  "contested-evidence" and a skipped assertion. Compare exits 0 for a valid comparison
  within its requested limits, 1 for measured regression, and 2 when comparison is unusable.
  Reading a report is not an acceptance verdict: available costs remain readable beside
  coverage gaps, and a missing optional method measurement does not erase valid process costs.
  A coverage check emits one aggregate assertion, with individual category results in its observed
  evidence. Any missing required category takes precedence and yields skipped/refer/exit 2.
  Otherwise any measured insufficiency yields failed/refuse/exit 1; only all adequate categories
  yield passed/permit/exit 0. Caller coverage of exactly 75 percent meets the explicit local floor.

  Scenario: A bounded capture reports sampled cost separately from actual calls
    Given an operator declares a finite observation window and workload/environment cohort
    When the runtime-performance report reads process-resource samples and runtime call counters
    Then the report keeps sampled CPU/resource cost distinct from actual calls
    And the report carries the declared cohort beside the observed values
    And the assertion uses checkId "runtime-performance"

  @regression
  Scenario: A resolved hot function remains visible without an unwound caller stack
    Given a saved CPU recording resolves a sampled instruction to a function symbol
    And the same sample has no usable caller frames
    When an operator reads the runtime-performance CPU report
    Then that function contributes its sampled period to the self-CPU ranking
    And missing caller frames do not turn a resolved function into an unknown function
    And caller coverage is reported separately from leaf-symbol coverage
    And caller clusters are withheld when usable caller coverage is below 75 percent
    And the sampled periods are not presented as invocation counts or elapsed method timings
    # Constraint: DWARF sampling requested does not prove that callchains were actually captured.
    # Probe: focused runtime-performance.test.ts regression using resolved leaves without callers.

  Scenario: A method observation distinguishes a returned response from an abandoned caller
    Given storage dispatches a conductor method after acquiring admission
    When the method returns a response or the caller drops its waiting future
    Then a returned response records duration and outcome for the dispatched method attempt
    And admission waiting and calls shed before dispatch are excluded from response duration
    And a dropped caller is recorded separately without claiming conductor cancellation or completion
    And metric labels contain no payload, raw error text or cell identity
    # Probe: storage's focused method-observation and metric-exposition tests.

  Scenario Outline: Comparable costs make their regression decision visible
    Given two valid observations use the same declared operator cohort
    And both observations have the same peer-name set
    And the baseline costs <baseline> CPU seconds per minute
    And the candidate costs <candidate> CPU seconds per minute
    When the requested maximum relative growth is 10 percent
    Then the comparison outcome is <outcome> and its decision is <decision>
    And its observed evidence names the measured value and threshold
    And compare exits <exit>

    Examples:
      | baseline | candidate | outcome | decision | exit |
      | 100      | 110       | passed  | permit   | 0    |
      | 100      | 111       | failed  | refuse   | 1    |
      | 0        | 0         | passed  | permit   | 0    |
      | 0        | 1         | failed  | refuse   | 1    |

  Scenario Outline: Missing, timed-out, or cancelled evidence is never a zero
    Given a requested comparison has no usable process-cost window because its evidence is <reason>
    When the comparison is assembled
    Then the assertion is skipped with <reason> identified in its summary
    And compare exits 2 for the unusable observation
    And no missing value is rendered as zero or a passing trend

    Examples:
      | reason    |
      | missing   |
      | timed out |
      | cancelled |

  Scenario: Incomparable observations refer instead of inventing a regression
    Given two observations disagree on their declared cohort or named runtime peers
    When an operator requests a trend comparison
    Then the comparison is referred as contested evidence
    And it is not collapsed into failed or passed
    And the report shows the baseline and candidate cohort labels and complete peer sets so the operator can re-measure
    And compare exits 2

  Scenario: An agent checks the complete drilldown before trusting a performance report
    Given an operator requests "check --require-coverage all" for a named capture
    When the CLI evaluates each requested peer and metrics endpoint
    Then it checks CPU leaves and callers, method counts, durations, outcomes and timeouts
    And generic errors or abandoned callers do not establish method timeout attribution
    And it checks workflow runs and triggers, SQL timing, network watermarks, heap attribution and I/O attribution
    And it lists direct evidence or missing evidence for each required category
    And aggregate process memory and I/O do not establish method-level attribution
    And slow-query-only logs do not establish complete statement timing coverage
    And no category passes merely because another peer or endpoint supplied its evidence

  Scenario Outline: Drilldown acceptance distinguishes missing evidence from measured insufficiency
    Given all other required drilldown evidence is available and adequate
    And the requested CPU caller evidence is <evidence>
    When the operator checks that capture's required coverage
    Then the assertion is <outcome> and the decision is <decision>
    And check exits <exit>

    Examples:
      | evidence                                | outcome | decision | exit |
      | absent                                  | skipped | refer    | 2    |
      | measured with 50 percent caller coverage | failed  | refuse   | 1    |
      | measured with 75 percent caller coverage | passed  | permit   | 0    |
      | measured with 90 percent caller coverage | passed  | permit   | 0    |

  Scenario: Diagnostic detail expires without exposing application data
    Given an operator enables a finite diagnostic interval for a runtime
    When explicitly scoped operations execute during and after that interval
    Then detailed operation events are emitted only during the enabled interval
    And events can correlate a local operation with its completed statement timings
    And unscoped statements remain explicitly unattributed
    And exported metric labels contain no correlation IDs, SQL, bindings, raw errors or cell keys
    And dropping a caller does not claim that the underlying work stopped
    # Acceptance target: runtime scope, expiry, privacy and cancellation tests plus a live bounded capture.

  Scenario: Workflow observations preserve notification coalescing
    Given several notifications can wake the same cell's workflow before it runs
    When an operator inspects the workflow's local diagnostic events
    Then notification counts, consumed wakeups and workflow runs remain distinct observations
    And each started run has one terminal result or an explicitly incomplete capture boundary
    And terminal results distinguish completion, retriggering, errors and a dropped workflow future
    And a dropped workflow future ends its observation without proving that dispatched external work stopped
    And local cell attribution does not require public cell-key metric labels

  Scenario: A bounded I/O profile distinguishes syscall attribution from device traffic
    Given an operator captures a fixed window of selected I/O syscalls for an identity-verified process
    When completed syscalls provide return values, durations and resolved user callchains
    Then the report ranks observed successful syscall bytes and counts by their resolved functions
    And failed syscalls remain separate outcomes rather than negative traffic
    And syscall counts are not presented as method invocation counts
    And socket and logging traffic are not silently classified as physical disk traffic
    And unsupported memory-mapped, io_uring and kernel-writeback activity remains outside the stated scope
    And report output excludes file paths, buffer contents and raw argument values

  Scenario: An I/O observation distinguishes its planned boundary from lost evidence
    Given an operator declares a finite capture duration and event and byte limits
    When the collector stops its own profiler at the planned end of the window
    Then the report labels the observation as a bounded window rather than a failed target process
    And work without an observed syscall completion remains outside completed-call totals
    But reaching an event or byte limit, losing trace events, changing process identity or forcing termination makes attribution incomplete
    And incomplete attribution cannot pass its required coverage check

  @wip @regression
  Scenario: Preparation does not consume the synchronized statement observation
    Given an operator explicitly requests a 60 second SQL observation on a startup-authorized conductor
    When executable fingerprinting and preliminary network and metrics observations take time
    Then those preparations finish before the private 62 second SQL window is armed
    And the collector verifies the conductor instance and its admin listener before and after admission
    And the resource observation begins only after the admitted artifact is opened and pinned
    But if the artifact does not enclose the full resource observation the capture is incomplete
    And a partial multi-peer admission failure closes local handles without claiming to cancel native work
    # Constraint: the two-second bookkeeping margin is finite, not permission to shorten the observation.
    # Probe: scripts/__tests__/resource-profile.test.ts; live synchronized acceptance remains pending.

  @wip @regression
  Scenario: A blocked collector cannot extend a disposable heap dump's deadline
    Given a disposable profiling conductor has a startup-authorized one second dump deadline
    And its total allowed lifetime is longer than one second
    When a heap traversal stalls and the collector's event loop cannot run
    Then the conductor's kernel timer sends an unmaskable termination signal at the dump deadline
    And the collector records partial evidence rather than a completed heap observation
    And inability to confirm process termination remains an unresolved cleanup result
    And neither the active household nor a preserved specimen is a termination target
    # Constraint: an event-loop timer or whole-canary lifetime alone cannot enforce a shorter dump deadline.
    # Parameters: production dump authorization is 1..30 seconds; native subprocess timer tests use 100 ms.
    # Probe: heap_capture native timer tests and performance-canary.test.ts; real native capture still required.

  @wip @regression
  Scenario: A parseable prefix does not prove a complete heap profile
    Given a bounded heap parser emits a syntactically valid partial summary
    When the parser exits unsuccessfully or its cancellation has no confirmed process-stop evidence
    Then the collector refuses to mark the heap artifact valid
    And unresolved parser cleanup preserves its private inputs for investigation
    And native traversal completion and parser success remain separate from attribution coverage
    # Constraint: readable stdout and a reaped wrapper cannot substitute for successful, stopped parser work.
    # Probe: performance-heap.test.ts and performance-canary-launcher.test.ts.

  @wip @regression
  Scenario: Ending an isolated profiling worker contains detached helpers
    Given the profiling worker is the initial process of a fresh private PID namespace
    And its conductor and parser may use detached process groups inside that namespace
    When the outer watchdog terminates the worker
    Then the kernel also terminates its contained processes
    And a detached helper cannot perform its scheduled later write
    And no household process or persisted household state is changed
    # Constraint: network isolation and the outer process group alone do not contain detached children.
    # Probe: performance-canary-launcher.test.ts namespace teardown regression; no live household claim.
