---
id: f19-household-proof-report-20260929
status: partial
date: 2026-09-29
---

# F19 household proof — delivery verified; widening transition not exercised

The rebuilt leg-3 storage serves the FCT v2 course anonymously with the same earned head and
identical body on Matthew, Jessica and James. This was also true before the rebuild. The differing
`commons` / `public` strings are equivalent under the storage verifier, explicitly accepted by the
operator during this run. This is delivery evidence, **not evidence of a private-to-commons
transition caused by F19**. F19 remains partly verified; do not mark its wider claim complete.

No storage code, reach rule, DNA, conductor binary, fixture reach or peer projection was changed
to manufacture convergence. No push, leg 4 or integrity-tag-gate work was performed.

## Build, ownership and execution

- Starting tree: `594179e99`, nine commits ahead of `origin/dev`, including storage fix
  `600dff89109b24e221eb9f33b151f99a7b78cb67`. Unrelated worktree edits were preserved.
- `epr doctor`: governance floor ready; Codex hooks still unprojected.
- Host inspection found prior background session `5037f04c` idle/blocked. Stopped that named
  session for the handoff. Sandbox process/network isolation initially hid the host mesh; all
  runtime probes below ran against the host loopback services.
- Berth session: `codex-f19-household-20260929`. Cargo claimed for the build; mesh claimed
  `class=verify`, `ttl_s=3600`, `claimed_at=1790686716.184`. The scoped a2o runner subsequently
  released its mesh lease. Final cleanup released remaining resources and unmoored the session.
- Built from `elohim/elohim-storage` with `CARGO_BUILD_JOBS=1`,
  `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev`,
  `RUSTFLAGS='--cfg getrandom_backend="custom"'`, and
  `cargo build --locked --features 'p2p p2p-iroh' --bin elohim-storage`.
  Build finished in 8m14s; tool-session refresh lost the first handle, so a cached repeat confirmed
  **EXIT=0**. Artifact mtime: **2026-09-29T13:05:52.429999945Z**.
- Installed atomically into the pool's `mesh-bin/elohim-storage`; SHA-256:
  `68bdf78c82b0b636de977eea4ebad5b94113ef5d89d5a69b52618af39109f693`.
- `MESH_TRANSPORT_BACKEND=dual just mesh storage-restart matthew jessica james`: **EXIT=0**.
  New PIDs: Matthew `2507365`, Jessica `2507447`, James `2507558`. All three `/proc/<pid>/exe`
  hashes matched the artifact above. Zome-path readiness passed; conductors were not restarted.
- `just mesh preflight`: **EXIT=0**, without any stale-binary or stale-hApp override.
- Steward plan: `pnpm exec tsx scripts/steward-publish.ts foundations-christian-technology
  --dry-run`: unchanged, seed hash matched, **EXIT=0**. This is the steward script's read-only
  planner, not a content-seed dry-run.
- Re-declaration: same command with `--redeclare` instead of `--dry-run`: **EXIT=0**, `declared`.
  Signing agent: `uhCAkVyP1wsmt6wJjShequ0tBAmlkt2iuuzKO6dczOa22wd67eTGM`.
  The command signed a new earned declaration of the existing head; it did not author new bytes.

## Course evidence

Course ID: `foundations-christian-technology`.
Head on every peer, before and after:
`uhCkkln3rUdCOyOYm_wIKC2OBeN8zqllCGwyj7uld0ZFN3aG0ZC81`.

| Peer | Reach before | Reach after | Anonymous content before/after | Live head before/after |
|---|---|---|---|---|
| Matthew :8090 | commons | commons | 200 / 200 | same head; declared=true, earned=true |
| Jessica :8091 | public | public | 200 / 200 | same head; declared=true, earned=true |
| James :8092 | public | public | 200 / 200 | same head; declared=true, earned=true |

Each content row's `dhtAnchorHash` matched `/head?election=live` `headActionHash`.
All six course bodies hashed to
`653ba7a90b47bb0f2285f01d1254cf77edcabfd4b3e694f332f06fbd71c15eb0`.
Every row referenced the same blob:
`sha256-c5dcc24c13dd03354e57ed7a1be9fe8d5272beb9829182f4b340785941f712a8`.

The steward script's `--await-peer` checks literal reach strings. It was not used to require an
alias rewrite after the operator accepted equivalence. No test was edited to conceal a failure.
The equality assertion in the original handoff was superseded by that explicit clarification.

## Refusals and safety checks

- `love-map-matthew-jessica` (intimate): anonymous content, `/head`, `/head-record`, and source
  relationships all returned **403** on all three peers, before and after (12 checks each time).
- Existing private rows `__preflight_test_1790339220708` on Matthew and
  `__preflight_test_1790339239399` on Jessica: the same four surfaces returned **403** after
  restart (8 checks). Read-only SQL selected only IDs/reach; no private bodies were collected.
- **James private coverage is unmeasured:** read-only projection inspection found no `private`
  row there. An initial `bdd-smoke-tests` probe was 403 on Matthew/Jessica but absent on James
  (content/head/head-record 404; relationship query 200). Absence is not proof of private
  authorization. No fixture was created through an undeclared write to fill this gap.
- `just test mesh features/dataplane/reach-enforced-http.feature`: **EXIT=0**, **3 scenarios /
  24 steps passed**, zero findings. Unverifiable bearer and self-asserted identity headers gained
  no extra listing access. The feature's WIP byte-route case was not run.
  Report: `genesis/a2o/reports/sprint-report-household-20260929T130939Z-594179e9.md`
  (JSON beside it). Composite source fingerprint `sha256:8c254f9491605970` is distinct from the
  executable hash above. The report records `networkStage: unknown`, unknown `sut.dnaHash`, and
  the pre-existing dirty conductor submodule. It does not attest a sealed simulator world.
- Focused F19 regression re-run: `cargo test --locked --features 'p2p p2p-iroh' --lib f19_`
  with the build environment above and `RUST_TEST_THREADS=1`: **EXIT=0, 8 passed**,
  4,385 filtered out. Compilation took 6m08s; tests took 1.44s.

The crafted hole probes are library regressions, not forged-head experiments on the household:
retained body, retained private blob, older election after author narrowing, unrestated edge,
narrower incoming version, non-earned head and unknown reach. Title/description replacement and
the real-server positive adoption remain covered by leg 3's recorded full storage gate (4,238
library tests plus integration suites); that full gate was not rerun for this documentation pass.

## Interpretation and remaining proof

`recognized_reach_level_index` (`src/epr_service.rs`) maps `public` and `commons` to level 0.
`widen_to_adopted_earned_reach` (`src/db/content_diesel.rs`) changes only strictly wider levels.
The unchanged aliases are therefore neither a newly closed side door nor a legitimate caller
refused. Both callers already had anonymous access. Treating the spelling difference as an
authorization red would recreate the oscillation this session is meant to prevent.

A separate static review found an unexercised integration concern: `adopt_local` constructs a
same-head verified patch but supplies `blob_cid` only for a move/fill or pointer repair. When the
stored blob already matches, F19's retained-blob guard can refuse a genuine widening because the
patch omits that evidence. This is a candidate **LEGITIMATE-REFUSED** case requiring a dedicated
authorized transition fixture, not a demonstrated failure of the already-public course. Preserve
the completeness guard; carry the verified evidence if that case is confirmed. No fix was made.

Story-graph seam for the next proof:

- chain: commons-path-steward-publish / F19
- between: authenticated earned head adopted -> newly authorized reader receives its bytes
- missing node: a previously restricted, same-head, matched-blob projection receives the complete
  verified version and applies the newly authorized reach; probe row/head/body plus anonymous
  refusal-before / permit-after, and unchanged unrelated edge reach
- current state: unmeasured on household; the course fixture never crossed this boundary

The existing feature remains WIP; no new runnable scenario is claimed. The reach habit remains
red. The storage fix's local safety tests and this delivery check do not close integrity-layer
authority, confidentiality, world isolation or revocation ordering.

## Operator clarification and brainstorm grounding

Recorded in `genesis/data/timeline/backlog/reach-delivery-subsystem-solutioning-bootstrap.md`:
Matthew is the operator of record; the development agents act under his authorization; the
developer effort and its demonstrations have earned public reach. Public visibility of simulacra
does not confer real-network standing on fictional memberships or grants. World, stage, audience
and authorizing holon must remain distinguishable. This is an accepted design input, not an
assertion that a grant chain already exists in the runtime.

Read-only GPT-6-sol reviews grounded authority/integrity, world/stage, and verdict/freshness seams.
Existing entry types and verdict witnesses provide reuse points; holon-authorized tuple writes,
world sealing and version/fact-freshness/key-epoch binding remain design or implementation gaps.
The operator's claim/human-standing/Mishpat-affirmation trace is now explanatory context in
`.epr-meta/manifest.md`, with no invented network affirmation or executable rule. The existing
reach backlog now makes construction, discovery and evaluation of that trace foundational (§6b),
adds solutioning question 13, and requires acceptance traces in DoD (h). No separate spec was minted.
Root manifest validation returned no issues; evaluation tests passed 42 checks and resolver tests
passed 36 assertions. `just gate genesis` passed (schema validation, typecheck, 689 tests passed /
9 skipped). The habit projection was regenerated and its freshness check passed without changing
status. The operator's gopher analogy is included as bounded orientation from any location, with
progressive authority traversal and an acceptance test in the same backlog.

## Disposition

**Do not push this as “F19 fully proved.”** Course delivery and active HTTP checks are green;
the wider transition still needs a declared fixture, and the design session must preserve the
operator's authority while making simulated standing explicit. Stop before push as instructed.

Local raw artifacts: `/tmp/f19-before.json`, `/tmp/f19-after.json`,
`/tmp/f19-negatives-before.json`, `/tmp/f19-negatives-after.json`,
`/tmp/f19-storage-build.log`, `/tmp/f19-storage-build-confirm.log`,
`/tmp/f19-storage-restart.log`, `/tmp/f19-preflight.log`, `/tmp/f19-redeclare.log`,
`/tmp/f19-reach-lane.log`, `/tmp/f19-focused-tests.log`. The committed report preserves the
material results; the temporary files are not durable authority.
