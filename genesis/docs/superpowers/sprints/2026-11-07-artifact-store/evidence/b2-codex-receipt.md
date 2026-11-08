Changed:

- [doorway-client.ts](/projects/elohim/.worktrees/as-b2/genesis/seeder/src/doorway-client.ts) — retries deferred HTTP 400 commitment POST/PUT writes four total attempts with 1/2/4-second injectable backoff and attempt-aware errors.
- [doorway-client-deferred-commitment.test.ts](/projects/elohim/.worktrees/as-b2/genesis/seeder/src/__tests__/doorway-client-deferred-commitment.test.ts) — covers recovery, exhaustion, logging/backoff, and non-deferred 400 behavior.

Tests were not run. Both `genesis/seeder/node_modules` and root `node_modules` are absent, and installation was prohibited. Intended command:

`cd genesis/seeder && pnpm exec vitest run src/__tests__/doorway-client-deferred-commitment.test.ts`

Decision: scoped retries centrally to `/api/v1/commitments` POST/PUT requests, using the required case-sensitive `deferred` substring. Other responses remain untouched.