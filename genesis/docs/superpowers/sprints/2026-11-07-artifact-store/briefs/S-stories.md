# Brief S — stories first: `@concern:artifact-custody` and the sibling fetch (Codex sol drafts, Sonnet steps, blind-reader review)

Worktree `as-s`, branch `artifact-store/s`. Read `common.md`, plan §7.3 "Stories first" and §7.4 R2/R4 (`2026-10-29-native-dev-tooling-over-peers-plan.md:317-324, 376-381`), and `design.md` §3.e.

**Write-set:** `genesis/a2o/features/devflow/sibling-fetches-the-build.feature` (new, `@concern:sprint-lands-each-closure-once`; every scenario `@wip` until F2 lands), `genesis/a2o/features/storage/artifact-custody.feature` (new, `@wip @concern:artifact-custody`), step stubs under `genesis/a2o/steps/` only where the dry-run needs them.

**Scenarios:**
- Sibling fetch: resolve by key, fetch, verify each chunk and the whole, install, record `Use` against the builder's `Produce`; an unfetchable record is a typed miss and the build proceeds; an interrupted fetch resumes only missing chunks; a trickling holder is abandoned within budget; an oversized manifest is refused before any chunk moves; a swapped manifest CID is refused; an admitted device outside the grant is refused by name; a stranger gets the uniform refusal; one scenario per transport mode, with a mode that cannot deliver reported as a typed gap.
- Artifact custody (R2/R4): private, self and intimate reach create zero DHT heads; Trusted admits an election only under a versioned custody-admission policy; an unknown policy epoch is held with `pending-election` and its reason; `under-custodied` and repair within the measured objective; quarantine (expiring hold) versus refuse (terminal); consent withdrawal retires replication and serving without deleting proof; supersede, withdraw and standing revocation stay distinct; `no-eligible-head` is explicit and `unreachable` is never absence; an older reader that cannot interpret a policy epoch cannot admit an election under it.

**Review floor (granted by the steward):** one considered fresh read per feature file by the context-isolated `blind-reader` agent (profile a2o-story); fix its correctness findings; the orchestrator closes by judgment and records each round (reader verdict and counts) in the hub's STATE.md. A Codex leg never performs or claims this review.

**Gate:** `just gate genesis-a2o` (includes `gherkin-prepush-lint` and the dry-run: 0 undefined, 0 ambiguous).
