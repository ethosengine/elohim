---
name: sccache-spawn-enoent-rca
description: "RCA for the sccache ENOENT bug that disabled sccache on the sweettest stage (efbac2938 → a92d91c2b). The prior framing ('cache-hit missing binary' or 'sccache cannot be exec'd in subprocess context') is WRONG. Actual failure: cargo intermittently fails to spawn the sccache binary itself with os error 2, ~1.7% of rustc invocations, after thousands of successful spawns in the same build. Matches sccache upstream issue #2023 class. Build-script involvement is incidental — the failing-to-spawn binary is sccache, not build-script-build."
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 3b93d5d1-a372-4195-9ba9-6d2c9b0faa75
---

## The bug, correctly stated

When `RUSTC_WRAPPER=sccache` is set for the sweettest stage in the elohim-holochain DNA pipeline, cargo intermittently fails to spawn the `sccache` binary itself with `os error 2` (ENOENT). Verified from Jenkins build #1225's `dna-integration-sccache-stats.txt` + the failing-stage log:

- **Compile requests executed:** 1857
- **Cache hits:** 1548 (85.52% overall; 100% C/C++, 94% Assembler, **0% Rust** — bucket had no Rust entries this run)
- **Compilation failures:** 32 (~1.7%)
- **Cache write errors:** 0 (writes are clean — cache is healthy)
- **Non-cacheable reasons:** crate-type 58 (sccache correctly identifies `--crate-type bin` as non-cacheable per upstream docs)

The cargo error format on a failed invocation:

```
error: could not compile `hashbrown` (lib)
Caused by:
  could not execute process `sccache rustc --crate-name hashbrown ...` (never executed)
Caused by:
  No such file or directory (os error 2)
```

**The process cargo cannot exec is `sccache`, not `build-script-build`.** Standard `std::process::Command::spawn()` returning ENOENT (= execvp returned ENOENT) means the named binary was not found on PATH at the moment of the spawn syscall. But the binary IS on PATH — 1548 prior invocations in the same build proved that. So this is an intermittent spawn-time failure, not a configuration failure.

## Why the prior framings were misleading

Two prior commit messages framed this bug wrongly:

1. **`efbac2938` (2026-05-09):** "sccache returns a 'cache hit' response for the build-script's rustc compilation, but doesn't materialize the linked executable." This is internally inconsistent with the observed stats: the failing Rust hit rate was 0%, meaning no Rust cache hits were returned at all. The framing also assumed the failure was at cargo's *exec of build-script-build* — actually it's at cargo's *exec of sccache* itself.

2. **`a92d91c2b` (2026-05-11):** "the failure is in build-script subprocess context where sccache cannot be exec'd." Half-right: sccache cannot be exec'd. But this isn't specific to build-script subprocess context — the spawn-ENOENT happens on regular library compiles too (hashbrown, named in the build #1225 log, is a library compile, not a build script).

3. **The current Jenkinsfile comment (line 589-590):** "Re-enable when tiered-quilt substrate hardens that path." **Misleading** — the bug is unrelated to tiered-quilt or S3-backend substrate. Cache writes are clean (0 errors). The bug is at the local `sccache` binary spawn syscall, not at the cache layer. The tiered-quilt cutover landed 2026-05-11 and would not affect this class.

## What it actually matches upstream

- **sccache issue #2023** (closed Feb 2024): "Intermittent CI failures `error: No such file or directory (os error 2)`" — original symptom shape, with sccache server starting on a port and then immediately failing intermittent invocations. The fix that closed #2023 was specific to server-startup race, but the broader spawn-ENOENT class clearly recurs.
- **sccache issue #2687** (open Apr 28 2026, 2 days before 0.15.0 released): "Failed to open file for hashing: <crate>.rmeta — No such file or directory (os error 2)." Different exact mechanism (sccache fails to hash an --extern dep .rmeta during cache-key calculation) but same ENOENT-under-load family.

sccache 0.15.0 (released 2026-04-30) is the latest published version — no upgrade target exists. We are on the bleeding edge of the bug.

## Why: How to apply

1. **Don't trust the existing Jenkinsfile comment.** "Re-enable when tiered-quilt substrate hardens" is wrong. The substrate is fine. The bug is in sccache process-spawn behavior under parallel cargo load, not in S3 cache state.

2. **Don't use `grep -c 'ENOENT.*build-script'` as a recurrence classifier.** The actual log signature is `could not execute process \`sccache rustc` followed by `os error 2` on the next `Caused by:` line. "build-script" does NOT appear in the failure line — it's a downstream side-effect when the failing crate is a build dependency. The correct grep pattern is `could not execute process .sccache rustc`.

3. **Workaround paths in order of preference:**
   - **Pin to sccache 0.14.0** (released 2026-02-09) — predates 0.15; if the bug intensified in 0.15, downgrade may stabilize it. Low risk, easy A/B test.
   - **Use the target-cache PVC instead of sccache for sweettest** (the proposed Wave 1 of the sweettest-stage-efficiency sprint). Cargo's incremental cache via persistent disk gives most of the speedup without depending on sccache's spawn behavior.
   - **Last resort:** Filter sccache invocations via a small shell wrapper that retries on ENOENT — hides the symptom; doesn't fix root cause; should be a stopgap not a fix.

4. **File an upstream issue.** Mozilla has the symptom class on file (#2023, #2687) but not specifically a 0.15.0 + parallel cargo + Holochain dep tree repro. Filing with our build #1225 stats + log excerpts gives them something concrete. This is the "right road" fix.

5. **Update the Jenkinsfile comment** when the spec doc lands, replacing the misleading tiered-quilt reference with a pointer to this memory entry and to the actual upstream issue numbers.

## Provenance

- Build #1225 console log: `https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/1225/execution/node/142/log/`
- Build #1225 stats artifact: `https://jenkins.ethosengine.com/job/elohim-holochain/job/dev/1225/artifact/elohim/holochain/tests/sweettest/dna-integration-sccache-stats.txt`
- Build #1225 daemon log artifact: 0 bytes (the diagnostic plumbing from `0b4055851` setting `SCCACHE_LOG_FILE` did not capture daemon output in 0.15.0 — separate misconfiguration worth knowing)
- Commits: `efbac2938`, `0b4055851`, `cc419f83d`, `e8c5c94a1`, `a92d91c2b`
- Shift journal: `.claude/shifts/2026-05-11T02-24-fix-sccache-unbound-on-elohim-holochain.journal.md`
- Upstream: sccache issues #2023, #2687; docs/Compilers.md (crate-type bin non-cacheable rule)
- RCA conducted: 2026-05-24
