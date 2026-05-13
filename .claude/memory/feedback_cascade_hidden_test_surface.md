---
name: Cascade-hidden test surface — failed-count is a misleading single metric
description: When tests can't even start, sprint-report shows "98 fail" out of 98 visible — fixing the cascade root reveals 262 scenarios with 90 fails (better visibility, not regression)
type: feedback
originSessionId: cdffa1f9-7b63-4657-ae44-2cafff5156bf
---
A failure-count target like "≤ 50 failures" is misleading when a cascade root keeps tests from running. Build #968 showed 98 findings out of ~98 visible scenarios because the cucumber-expression bug + Gherkin parse error blocked discovery. After fixing those, build #971 surfaced 262 scenarios with 90 failures — the absolute count barely dropped (98 → 90), but the proportion improved dramatically (98/98 → 90/262).

**Why:** When prior cascades collapsed, the test surface was hidden — sprint-report only saw the scenarios that managed to start. Fixing cascade roots *unmasks* a wider surface, so failed-count goes up before it goes down.

**How to apply:** When measuring shift progress on a multi-cascade pipeline, track three numbers, not one — total scenarios visible, total failed, and ratio. A drop in failed-count alone is ambiguous. Surfacing 2.7× more scenarios with the same failure ratio is *progress*, not regression. Communicate this distinction in the journal and in shift bail messages, especially when the predicate is "≤ N failures" — the predicate may need restating once the test surface stabilizes.
