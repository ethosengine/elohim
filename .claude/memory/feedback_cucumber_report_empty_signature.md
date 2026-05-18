---
name: cucumber-report-empty-signature
description: Jenkins Cucumber plugin "Missing report result → UNSTABLE" + present-but-empty sprint-report JSON = Gherkin parse error elsewhere in the feature set; read E2E log for "Parse error in features/" first, NOT runner config / tag filter / hook bail
metadata:
  type: feedback
---

When elohim-genesis/dev returns UNSTABLE with `Findings: 0, scenarios: 0` in the sprint-report (md + JSON both present, both empty of completed scenarios), the cause is almost always a **Gherkin parse error somewhere in the feature set** — NOT a runner configuration issue, tag filter exclusion, or pre-test hook bail.

**Signature in the Jenkins Cucumber plugin output (E2E Verification stage tail):**

```
[CucumberReport] Missing report result - report was not successfully completed
[CucumberReport] Build status is changed to UNSTABLE
```

**Why it looks like a runner config bug:** the stage timing is short (sub-2-minute E2E "run"), the sprint-report exists but has no scenarios, JSON files are well-formed but empty arrays, and Cucumber escalates to UNSTABLE (not FAILURE) — superficially identical to a tag filter that excludes everything.

**Actual root cause:** Gherkin's AST construction happens BEFORE tag evaluation. If any feature file in the discovery path has a parse error, cucumber-js drops the entire run, writes empty output, and exits with code 1. The Cucumber Jenkins plugin reads the empty output and reports "Missing report result → UNSTABLE."

**How to diagnose:**

1. Pull the E2E Verification stage log (mcp__jenkins__getBuildLog around the stage start marker).
2. Search for `Parse error in features/` (mcp__jenkins__searchBuildLog with that pattern).
3. The error names the file and the line number. The grammar issue is usually a continuation line on a step (Gherkin requires `And`/`But`/another keyword to continue), a misplaced docstring delimiter, or an inconsistent indent.
4. The fix is in the feature file. See [[shift-can-fix-unparseable-test-fixtures]] for the principle-#6 navigation when this comes up in a shift.

**Confirmed surface area on 2026-05-17:** three bare continuation lines across `recovery-shamir-optional.feature:30,43` and `transport-perf.feature:143` caused the cucumber-report-empty class to land in elohim-genesis #1011, #1012, #1013. Fix in commit 9090b8388 dropped the measure from 500 (scenarios=0) to 34 (scenarios running, 34 failed) on first run.

See also: [[shift-can-fix-unparseable-test-fixtures]], [[cascade-halt-masks-failures]] (this is the inverse pattern — cucumber-report-empty masks the genuine 30+ failing scenarios behind a single AST-construction gate).
