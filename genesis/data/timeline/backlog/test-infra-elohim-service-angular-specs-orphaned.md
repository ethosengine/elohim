---
id: "backlog-test-infra-elohim-service-angular-specs-orphaned"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-service `src/angular/**` specs run under NO runner — library vitest includes only resilience/ and distribution/, jest cannot parse them, the app's vitest is src/** only"
slug: "test-infra-elohim-service-angular-specs-orphaned"
written: "2026-08-25"
author: "epr-card-nav shift (integrator)"
status: "backlog"
priority: "medium"
tags: [test-infra, elohim-library, elohim-service, vitest, jest, orphan-suite, coverage-blindness]
cites:
  - app/elohim-library/vite.config.ts
  - app/elohim-library/projects/elohim-service/src/angular/services/governance-api.service.spec.ts
  - app/elohim-library/package.json
---

Found while adding a `queryChallenges` wire-shape test (2026-08-25): `app/elohim-library/vite.config.ts`
`test.include` lists `projects/elohim-service/src/resilience/**` and `…/distribution/**` only, so every
spec under `projects/elohim-service/src/angular/**` (GovernanceApiService and siblings — they import
from `'vitest'`) is never collected; `pnpm test:service` runs jest, which fails on `import type`
before collecting anything. Verified: `vitest run <that spec path>` → "No test files found", while a
scratch config widening `include` to that file runs it green (33 tests). A green library gate says
nothing about these services. Fix: add `projects/elohim-service/src/angular/**/*.spec.ts` to the
vitest include (the analog Angular plugin is already configured there) and retire the jest
`test:service` script if nothing else uses it; run the suite once and file any real failures separately.
