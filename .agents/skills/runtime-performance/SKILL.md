---
name: runtime-performance
description: Diagnose runtime performance concerns, high wall-clock time, CPU or memory pressure, I/O/network watermarks, and performance regressions. Use existing capture/report/check/compare tooling to rank costs, expose evidence gaps, and choose the next bounded drilldown; not a background monitor.
metadata:
  runtime: antigravity
  sourceRuntime: codex
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/runtime-performance.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/runtime-performance"
---
# Runtime Performance

Turn a performance signal into measured, actionable findings using the existing telemetry CLI. Do not build another collector, results registry, or profiler.

## When to use

Use for slow runtime operations, unexpectedly long wall-clock time, high idle CPU, memory growth, resource-watermark alerts, or a request for performance trends or bottleneck diagnosis. A signal is a reason to inspect evidence, not proof of a cause. For build-only delays, first distinguish queue/lease wait, compilation, tests and runtime work; use the existing pipeline-diagnostics skill for Jenkins stage evidence rather than profiling an unrelated conductor.

This skill is discoverable guidance, not a daemon, alert subscription or Jenkins stage. It runs when an agent receives the concern or signal. Do not claim scheduled capture or automatic watermark dispatch without verifying that wiring. Periodic CI exercise remains a separate integration task.

## Start from evidence

1. Identify the affected runtime, operation and time window; record the signal, units, threshold and provenance. Separate wall time from CPU time and queue wait.
2. Prefer a supplied saved capture. Read only the opening command/exit-contract section and `Agent reconciliation loop` of `genesis/a2o/scripts/runtime-performance.md` initially; inspect `.epr-meta/runtime-performance.habit.md` frontmatter and latest evidence delta for acceptance status. Read additional runbook sections only for the selected capture mode. Do not infer current readiness from old chat, an old binary or a passing unit test.
3. If the capture path is unknown, use a bounded metadata search in the operator-named evidence directory. Do not ingest whole append-only logs or scan all historical reports. If there is no usable capture, say so and propose one bounded measurement.
4. Run the existing report and coverage check. Report nonzero exit status honestly; a readable report is not an acceptance pass.

From `genesis/a2o`, replacing the uppercase paths with actual approved evidence:

```bash
node --import tsx scripts/runtime-performance.ts report --input CAPTURE --json
node --import tsx scripts/runtime-performance.ts check --input CAPTURE --require-coverage all --json
node --import tsx scripts/runtime-performance.ts compare --baseline BASELINE --candidate CANDIDATE --max-regression-percent THRESHOLD --json
node --import tsx scripts/runtime-performance.ts trend --input BASELINE --input CANDIDATE --json
```

A directory input resolves to its `resource-profile.json`; it is not an automatic scan of adjacent profiler files. Attribution uses the capture's recorded profile references and explicit external-evidence inputs. Do not infer that changing file input to directory input supplies missing CPU evidence.

Use an explicit verified `--perf /absolute/perf` when CPU recordings need symbolization. Use the runbook's bounded external-evidence arguments for SQL/workflow/network/heap evidence. Do not run comparison without a chosen threshold and comparable cohorts. No install or rebuild is needed merely to read existing captures.

## Capture only what is authorized

If fresh evidence is needed, use `runtime-performance.ts capture` / `resource-profile.ts` and the existing runbook. Before launch, specify target identity, workload, duration, event/byte budgets, evidence directory, expected overhead and ownership. Respect berth leases and any ongoing timing-sensitive run. Capture must not silently restart, recast, reset, deploy or change pins, limits or logging.

Start with a bounded process/counter and CPU window appropriate to the concern. Escalate to synchronized workflow/SQL/I/O evidence when it resolves a specific unanswered question. Verify matching process start/boot/executable identities and enclosing observation windows before correlating sources. Never infer a call count from CPU samples, device traffic from syscall bytes, or a cancelled server operation from an abandoned caller.

Private re-arm requires startup-authorized controls. Heap traversal with a hard deadline belongs only in an explicitly authorized disposable isolated canary; never aim termination at the active household or preserved specimen. A quiet canary pass proves lifecycle, not representative workload coverage. Read the canary section of the runbook before using `arm` or `heap-canary`.

Use finite waits matched to capture/build duration and completion callbacks. Do not repeatedly poll unchanged processes or logs. After a failure, inspect retained bounded evidence before retrying; do not retry ambiguous native admission.

## Produce the useful report first

For a qualitative signal such as 'high resource usage', explicitly say when the threshold, baseline or signal provenance was not supplied; report cost concentration rather than inventing a breach. A compact answer is: observed finding + evidence window; inference/confidence; missing proof; next bounded action.

Lead with measured findings, not a catalogue of instruments:

- Largest process costs and top sampled functions/caller chains, with units, interval and denominator.
- Instrumented method calls, durations, outcomes and timeouts; explicitly distinguish uninstrumented native methods.
- Correlated workflow, SQL, network, heap and I/O observations from the same process/window.
- Largest comparable changes, with workload, binary, machine, agent/cell population and cadence differences visible.
- For each actionable candidate: evidence path, confidence, impact, one plausible explanation labelled as inference, and the smallest next measurement that could disprove it.
- Missing, truncated, censored or insufficient evidence, alongside costs that remain useful.

Prioritize high measured cost or repeated unnecessary work over speculative micro-optimizations. Expensive hashing alone is not an RCA: find its callers and repetition driver before recommending a cache or architectural change. Scale claims require at least two controlled points per claimed hot path; one idle snapshot cannot establish a scaling law.

## Acceptance and development feedback

`check` exits 0 only for adequate requested evidence, 1 for measured insufficiency, and 2 for missing/unusable evidence. A named partial category check is not full acceptance. CPU leaf resolution is distinct from caller coverage; the current full caller floor is 75 percent. Process RSS and I/O totals are not method-level heap/I/O attribution. Provisional SQL/workflow data must not be promoted merely because it parses.

Keep the existing `runtime-performance` CheckWitness and habit as the acceptance boundary. Do not invent a parallel registry or flip the habit green from a canary smoke. Save private raw evidence in the existing persistent recovery evidence area; publish only bounded redacted summaries where appropriate. Update the habit's evidence delta when authorized work completes.

For sprint/CI follow-through, reuse these same commands around an identified representative workload and archive the existing report/witness format. Missing required capture is not a green build. Baseline comparisons need matched cohorts and explicit thresholds; until those exist, label findings diagnostic, not budget acceptance. A planted skill alone does not supply this CI wiring.

Finish with the strongest finding, what remains unproved, and the next bounded action. Diagnosis does not authorize implementation, deployment, external sharing or a fresh performance experiment outside the agreed scope.
