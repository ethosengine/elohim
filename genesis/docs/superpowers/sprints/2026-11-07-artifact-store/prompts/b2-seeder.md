You are editing TypeScript in the repository checkout at /projects/elohim/.worktrees/as-b2 (your working directory). Do not run git, brit, cargo, pnpm install, or any network command. You may run `cd genesis/seeder && pnpm exec vitest run <file>` or the seeder's existing test command if node_modules exists; if it does not exist, do not install it — say so in your final message.

Task (brief: genesis/docs/superpowers/sprints/2026-11-07-artifact-store/briefs/B2-seeder-deferred-retry.md — read it first):
The storage service sometimes answers a commitment write with HTTP 400 whose body contains the text "REA projection changed during authority read; deferred". That is transient; the seeder currently treats it as fatal and fails a MUST-SUCCEED household prologue leg. Make the seeder retry exactly that case.

1. Find where genesis/seeder/src/seed-commitments.ts (and the client module it uses to POST/PUT commitments) handles non-2xx responses.
2. A 400 whose response body contains the substring `deferred` is retried up to 3 more times with backoff 1 s, 2 s, 4 s (inject the sleep function so tests do not wait). Log each retry on one line naming the attempt number and the reason text. Any other 400, and any other status, keeps today's behaviour exactly. After the last attempt the error message names how many attempts were made.
3. Add unit tests beside the seeder's existing tests (find the existing test file pattern and runner under genesis/seeder): (a) deferred-400 twice then 200 succeeds and made 3 calls; (b) deferred-400 four times fails and the error names 4 attempts; (c) a non-deferred 400 fails on the first call with no retry.
4. Keep the change minimal; do not refactor unrelated code; keep existing exports and signatures backward compatible.

Final message only (never write a receipt into any file): the files changed with a one-line summary each, the test command you ran and its output summary (or that you could not run it and why), and anything ambiguous you decided.
