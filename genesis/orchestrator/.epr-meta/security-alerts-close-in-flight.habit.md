---
epr-habit-version: 1
id: security-alerts-close-in-flight
invariant: >
  Dependency edits expose open security package clusters to agents. Pushed
  dependency scopes introduce no new patchable vulnerable resolutions and pay
  down at least one existing patchable alert; the next harvest measures closure.
status: red
active: false
checks:
  - "python3 .claude/scripts/dependabot-paydown.py --base origin/dev --head HEAD (SECURITY_PAYDOWN=warn by default; strict enforces the ratchet)"
  - "python3 .claude/scripts/dependabot-harvest.py --clusters (next harvest measures closure)"
  - "python3 .claude/scripts/_lib/__tests__/dependabot_paydown_test.py"
  - "python3 .claude/scripts/_lib/__tests__/dependabot_inflight_test.py"
guard: >
  Unpatchable alerts never block paydown. Missing evidence is unmeasured, not
  clean. A fix commit or passing fixture does not establish live closure.
retire-when: >
  Open patchable Dependabot alerts on dev stay at 0 critical and 0 high for
  30 consecutive days.
---

Steward ruling, 2026-11-12: "we need to have an aggressive loop on closing
security issues in-flight so every agent fights forward on that universal
responsibility."

Baseline measured 2026-11-12 from the GitHub API by the orchestrator:
478 open (16 critical, 190 high, 213 medium, 59 low; 10 unpatchable).
Fixed by month: July 510 / Aug 16 / Sep 15 / Oct 0 / Nov 0.
