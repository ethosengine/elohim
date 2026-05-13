---
name: schema codegen produces oscillating Prettier diff
description: schema:codegen:ts oscillates 18 generated TS files between single-line and multi-line union types; the codegen script has no idempotent fixed point
type: feedback
originSessionId: bdf209e4-03e4-4d83-813b-19ac535d11dc
---
`pnpm run schema:codegen:ts` regenerates 18 files in `app/elohim-app/src/app/generated/`, `app/elohim-library/projects/elohim-service/src/generated/`, `elohim/sdk/schemas/generated-ts/`, and `genesis/seeder/src/generated/` with a Prettier line-wrap oscillation on union types like `Reach` and `ContentFormat`. Whichever form is committed, the next codegen run flips it.

**Why:** The codegen script and the local Prettier config disagree on whether to wrap union types past a width threshold. Different commits have committed different forms. Each codegen run produces a diff against whatever was committed previously. Affects only Reach and ContentFormat enum union exports — no field names, interface shapes, or schema semantics ever change.

**How to apply:** Before treating a `schema:codegen:ts` diff as a real schema drift, scan the diff. If it's purely whitespace/line-wrapping in `Reach` or `ContentFormat` union exports, it's the oscillation — not real drift. EPR Phase 3.5 T21 (commit landing 2026-05-01) skipped the codegen freshness gate because of this, on grounds that the drift is cosmetic. The proper fix is to pin Prettier behavior in the codegen script so it has a stable fixed point. Until then, treat single-file-pattern diffs in those 18 files as non-blocking for merges; surface as pre-existing if the pre-push hook flags them on push.
