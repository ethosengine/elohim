---
id: rg1-brief-l2-integrate
title: Brief L2 — stack every landed track onto rg1-integrate and make the tree compile
date: 2026-11-06
status: active
---

# Brief L2 — the integrator (Opus 5.5)

Worktree `/projects/elohim/.worktrees/rg1-integrate` (branch `regenesis/integrate`, `.family=sprint`, submodules `elohim/brit elohim/rakia sophia` initialised — `git submodule update --init` them if preflight refuses). Work ONLY there. brit panics in worktrees: every git command's FIRST line is `# Brit fallback: <reason>`. Commit by pathspec with trailer `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` (UNMEASURED attribution expected). Never `git stash` (WIP commit). KEEP list: brit notes, device keys, `.eprfs/**`, cites — never touched.

**Operator steer in force: WORKING CODE FIRST.** The deliverable is one linear branch on which every crate compiles, every TS workspace typechecks, and each track's own tests pass; full gates and habit verification run once after L3. Do not wait more than ~10 min for the host's single build slot (a PreToolUse hook refuses a second build; never kill another session's build) — record "not run (host slot held)" with the cheaper command's EXIT line.

## Stack order (rebase each track's branch ONTO the growing integrate head; resolve conflicts; one `fixup:` commit per seam if needed)
1. `regenesis/kernel` (head 9f9fef877; == dev once landed) — reset `regenesis/integrate` to it.
2. `regenesis/eprfs` (T3; take the ref as it stands — if T3 pushes more commits later the orchestrator will tell you to re-stack).
3. `regenesis/schemas` (T5: d382a8efe, 010a9e859 on an OLD kernel base ce870481a; rakia submodule commit f9787c9e58).
4. `regenesis/storage` (T2, head bc0edbd64; already contains T5's views/schemas/fixtures byte-identical and the rakia pin — expect identical-both-sides conflicts; keep storage's copy).
5. `regenesis/doorway` (T6, head a1a5d5603; base = storage).
6. `regenesis/app` (T7, head cbe451c01; base = storage).
7. `regenesis/seeder` (T8: e489b0150, 6bca817e0; old kernel base).
8. `regenesis/steward` (T10/T11) when its report lands — the orchestrator will say.
9. `regenesis/shem-evidence` (368aaa3ac: a2o stories, alpha inventory, STATE timings) — merge-friendly docs.
(`regenesis/dna` T1 is L3 — NOT in this stack.)

## Fix-ups to apply while stacking (each is a named seam from the track reports; cite the report in the commit body)
- Packager header: `genesis/a2o/scripts/epr-release-package.ts` sends `X-Blob-Cid` → `X-Content-Address` (ruling 18:22Z: the ONLY content header).
- T5 `element-registry-view.schema.json` still pins `cid` to `^sha256-` while its Rust doc says byte CID → `^bafkr4i[a-z2-7]{52}$`; drop the sha256 fixture in `schema_contract.rs`.
- Regenerate every generated copy on the stacked tree, never hand-edit: `pnpm run schema:codegen:ts` (app, doorway-app, sdk generated-ts, seeder), storage `cargo test export_bindings` → `elohim/sdk/storage-client-ts/src/generated` (`Content.ts`/`ContentWithTags.ts` must say `blobCid`), `pnpm run lamad:codegen` if touched. T7 reverted the doorway-app/sdk/seeder copies on purpose — regenerate them here.
- T8 → T9: `substrate-verify.sh` state keys `contentBlobCid`/`content_blob_cid`/`probeBlobCid`; the a2o TS port `scripts/substrate-verify.selftest.ts` + `steps/delivery.steps.ts:50-59` (`^bafkr4i` on `X-Content-Address`), `steps/mesh/{household-chaos,death-witness}.steps.ts`, and the 7 features' placeholder CIDs (plan §1.2 a2o row) → byte CIDs minted from real bytes or the kernel vectors; `gherkin-prepush-lint` green.
- `elohim-render` `BundleSource` trait: `resolve_blob_hash`/`resolve_server_blob_hash` → `resolve_blob_cid`/`resolve_server_blob_cid` (T6 could not; doorway + storage implement it).
- Doorway `/api/blob/verify` fields are now `expected_cid`/`outcome`/`computed_cid`; check nothing in app/lamad still sends `expected_hash` (T7 removed the doorway verification tier — confirm by grep).
- eprfs_tier: the 3 `elohim-storage` `eprfs_tier::blob_store_tier` tests fail until eprfs-core's byte-profile verify (T3) is on the same tree — after stacking they must pass; if not, the seam is eprfs-core `BlobCid::verifies` vs storage's expectation — fix in storage's tier adapter, not by weakening eprfs-core.
- `app/elohim-app/scripts/hc-mesh.sh:5485` comment flagged by the address lint (mesh scripts) — fix the comment.
- `genesis/scripts/seed-landing-bundle.mjs` updated by T8 but unrunnable (jszip resolution) — make it run (`node --check` + a dry invocation).
- Lockfiles: every cargo workspace that `--locked` gates touch must be relocked onto elohim-epr 0.2.0 + elohim-address + fastcdc (bridges/*, eprfs, steward/node already; check doorway, elohim, crates/*, elohim/holochain tools); `pnpm-lock.yaml` once from the root (`CI=true pnpm install --frozen-lockfile` must pass afterwards; T7's `@noble/hashes ^1.5.0` churned it).
- Managed gospel docs are NOT edited by you (`elohim/elohim-storage/CLAUDE.md`, `doorway/CLAUDE.md`, `app/elohim-app/CLAUDE.md` blob sections) — list every stale sentence with file:line in your report; the orchestrator edits them through `epr flow cites`.
- Codemod receipt: append rows for fixtures re-minted by T2-B (11) and T6 (fixture sweep) if their tools wrote none.

## Compile ladder (cheapest first; log every command to `genesis/local-dev/regenesis-1/l2-<step>.log` with a trailing `EXIT=` line)
1. `cd elohim && cargo check --workspace` (pool slot from `cargo-pool key`).
2. `cd elohim/elohim-storage && cargo check --all-targets --features p2p-iroh` (slot; `CARGO_BUILD_JOBS=1`).
3. `cd doorway/doorway-service && RUSTFLAGS="" cargo check --all-targets`.
4. `cd elohim/eprfs && cargo check --workspace`; `cd crates && cargo check --workspace` (consent-grant, elohim-views, elohim-address, seam-contracts…); bridges/* `cargo check --locked`.
5. `CI=true pnpm install --frozen-lockfile` (root); `pnpm run schema:codegen:ts`; `cd app/elohim-app && pnpm exec tsc --noEmit -p tsconfig.app.json`; same for `app/lamad`; `cd genesis/a2o && pnpm exec tsc --noEmit`; `cd genesis/seeder && pnpm exec tsc --noEmit`; `cd elohim/sdk/epr-ts && pnpm test`.
6. Tests, one crate at a time when the slot is free: storage `cargo test --lib` (the 3 eprfs_tier tests are the acceptance), doorway `cargo test --lib`, eprfs `cargo test --workspace`, app + lamad vitest.
7. `scripts/ci/address-profile-lint.sh` (T8) over the tree: report the count; it need not be 0 yet, but every remaining hit must be classified (cite/oci/record/prose/legacy-left-for-<track>).

## Hand-off
Report `genesis/docs/superpowers/plans/task-rg1-integrate-report.md` (`status: claimed-not-verified`, `landed_commit`, `verified_by:` blank, `concerns:`, `actor`): the final stack (sha per track), every conflict and how it was resolved, the compile-ladder EXIT table, the stale-gospel list, the lint census, and what is still red. Hand back concisely with evidence. Do not push.
