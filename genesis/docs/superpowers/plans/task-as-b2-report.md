---
id: task-as-b2-report
status: landed
landed_commit: 3b266e41d
verified_by: "orchestrator (Claude Opus 5.5): diff review + call-path trace + vitest 3/3 new, 742/742 suite EXIT=0 (evidence/b2-seeder-vitest.txt)"
cites: []
concerns: []
actor: "Codex gpt-5.6-sol (write leg, receipt evidence/b2-codex-receipt.md); reviewed and committed by agent:orchestrator@claude-opus-5-5"
---

# B2 — the seeder retries a "deferred" 400

**Change.** `genesis/seeder/src/doorway-client.ts`: `sendRetryingDeferredCommitmentWrite` re-issues a commitment write only when storage answers HTTP 400 with a body containing `deferred` (the storage text is `REA projection changed during authority read; deferred`, `elohim-storage/src/services/rea_commitment_service.rs:842`). Up to four attempts in all, backoff 1/2/4 s (an injectable `sleep` in `DoorwayClientConfig`), one log line per retry, and a final error naming the attempt count. Any other status or 400 body is returned unchanged to the caller's existing handling. Applied in `DoorwayClient.fetch` for `POST|PUT /api/v1/commitments`.

**Call path verified.** The failing household write (the elohim-host-landing projection commitment) goes through `ProjectionClient.createCommitment` (`seed-projections.ts:693`) and `CommitmentClient` (`seed-commitments.ts:514`); both extend `DoorwayClient` and call `this.fetch('/api/v1/commitments', POST)`, so they take the retry.

**Tests.** `src/__tests__/doorway-client-deferred-commitment.test.ts`: recovery after two deferrals, exhaustion naming 4 attempts, non-deferred 400 with no retry: 3/3. Whole seeder suite: 50 files, 742 passed, 9 skipped, EXIT=0.

**Gate.** `just gate genesis` runs at the integration closure (L2), not on this branch alone.
