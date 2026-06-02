---
name: husky-bypass-for-ci-only-changes
description: When pushing a CI/CD-specific fix (Jenkinsfile, manifests, orchestrator config) and the pre-push gate fails on unrelated code drift, HUSKY=0 bypass is appropriate
metadata:
  type: feedback
cites:
  - .husky/pre-push
---

When the only changes in a commit are CI/CD configuration (Jenkinsfiles, k8s manifests, orchestrator code) and the husky pre-push gate fails on unrelated drift in code surfaces the commit doesn't touch, `HUSKY=0 git push` is the right call.

**Why:** Pre-push gate is meant to catch breakage in the code surfaces being modified, not to force the pusher to fix inherited branch drift. When a CI-only commit blocks on Rust fmt drift in elohim-storage that predates the commit, the gate is over-triggering — it's running checks on projects with no actual change from this commit because the branch-vs-origin diff is broader than HEAD~1..HEAD. Fixing inherited drift mid-push burns time and pollutes the commit history. Bypass is honest about what's being pushed.

**How to apply:**
- Trigger: pre-push gate fails on projects the commit doesn't touch
- Sanity check: confirm the failure is in a path the commit doesn't modify (`git show --stat HEAD` vs the failing project path)
- Sanity check: confirm the failure is the kind a CI rerun would catch anyway (fmt, lint, test) — not something that would break deployment silently
- If both: `HUSKY=0 git push` is appropriate; mention the authorization in the response
- If the commit DOES touch the failing path, fix the drift first — that's the gate doing its job

Not for: commits that modify the failing code surface, commits that change deployment-critical state silently, commits where the failure looks like a real regression rather than rustfmt-class drift.

Related: [[memory_in_repo_two_tier]] (.claude/ tracked), [[no_kubectl_from_dev_env]] (operator/agent boundary).
