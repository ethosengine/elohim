---
id: "backlog-app-css-budgets-fontawesome-eyes-sprint-residuals"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "App UNSTABLE flags after the eyes-sprint merge: 6 components over CSS budget + fontawesome stylesheet path missing"
slug: "app-css-budgets-fontawesome-eyes-sprint-residuals"
written: "2026-06-11"
author: "agentic-developer (EPR durability shift, complementary capture)"
status: "backlog"
priority: "low"
ci_status: red
jobs: [elohim]
tags: [app, angular, budgets, css, webfonts, eyes-sprint]
cites:
  - app/elohim-app/angular.json
---

# Eyes-sprint UNSTABLE residuals (app #1531)

Build structurally green (tests, alpha deploy, E2E health all passed);
UNSTABLE from Angular budget warnings after the frontend-eyes-sprint merge
(8dbcc146a): elohim-navigator 13.06kB, markdown-renderer 10.46kB,
content-viewer 28.81kB (+18.8 over), policy-console 10.53kB,
doorway-dashboard 10.31kB, alert-banner 11.70kB — all over the 10kB
per-component CSS budget. Also: fontawesome stylesheet not located at
/assets/fonts/fontawesome/all.min.css (webfont asset move likely renamed
the path). Owner-discipline: frontend (angular-architect / eyes-sprint
follow-up) — either trim the styles, split them, or consciously raise the
budgets with rationale; fix the fontawesome asset path or remove the dead
reference.

shift_objective: |
  Clear the app pipeline's budget warnings honestly (trim/split/raise with
  rationale per component) and fix the fontawesome stylesheet path so app
  builds return to plain SUCCESS.
