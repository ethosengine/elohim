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

## Fleet follow-up: FCT v2 and campaign 1.4 (2026-09-29)

The operator clarified the immediate requirement: establish the elected head and prove convergence
between Adam and Matthew and their doorways. Intentional developer/elected views are later work;
they must not explain away today's divergence. Lamad course composition is a separate sprint.

Read-only content probes and browser captures showed alpha serving the five-movement v2 path while
`elohim.host` served the older six-chapter path. The HTTP `/head?election=live` response is not an
independent election-winner witness: `headActionHash` still comes from the storage projection, and
both probes returned `earnedSource: cached` with `stagingCandidateState: unavailable`.

Fleet admin credentials were located in the checked-in alpha and alpha-b doorway manifests and
used without printing their values. The conductor diagnostic used admin/app WebSockets, requesting
probe signing credentials and an app token, then calling only read-only content zome functions.
No content, head, election link, deployment or cluster resource was deliberately changed.

- **Matthew:** `resolve_canonical_election("foundations-christian-technology")` returned winner
  `uhCkkxFL6asdXIQGdVgxMgKwfpmNOKahPVgHp26UWlskOys-5F23_`, `canonical_earned: false`, declaration
  timestamp `1790587408793189`, and link `uhCkk09OUgVZr5SrtBdVwYZRrvESKKlNENpNbXZvreXV9ez9tkuu3`.
  `resolve_content_head_local` independently resolved that same canonical action and v2 content.
  This is an observed staging-tier election winner, not an earned-tier proof.
- **Adam:** admin `listApps` succeeded, but `authorizeSigningCredentials` failed. A captured
  doorway proxy frame reported `Holochain error: Request timeout`; the client subsequently closed
  with a pending request. The first diagnostic used an 18-second budget; the follow-up used 45
  seconds per call. Adam's election winner remains **unmeasured**, not inferred from the SQL head.
- Both listed Lamad DNA `uhC0kZezl4k2nZa5ZyU5O5H-5vH5LYpvwkSwkVx4G1wS_sHB4GTOt`.
  Adam's authoritative running-cell membership included Lamad, while storage's per-role health
  retained `app-disabled` for Lamad and node_registry, with an approximately 16,138-second episode.
  This is a latched failure awaiting successful role-call evidence, not proof the cell is currently
  absent. `elohim.host/health/serving` returned 503; alpha's returned 200.

**Next proof boundary:** obtain Adam's independent local election and resolved-content answers,
then distinguish election disagreement, missing winner bytes and stale projection. Do not substitute
`POST .../canonical-head` for this diagnosis: that endpoint declares a staging election, so forcing
agreement could conceal the campaign 1.4 failure. The existing operator reconcile verb requires an
actual performer JWT and a scoped delegation; the admin key alone does not authorize that verb.
No canonical-head POST, push, restart or redeploy was performed in this follow-up.

Diagnostic artifacts are local: `/tmp/fct-adam-election.json`, `/tmp/fct-{alpha,apex}-diagnostics.json`,
`/tmp/fct-{alpha,apex}-head-now.json`, and browser captures under
`genesis/a2o/reports/look/fct-v2-{public,apex}-before/`. The material election result and failure are
preserved above because those raw files are not durable evidence.


## Conductor candidate and household convergence receipt (19:04 UTC)

The isolated fork candidate `26374c316ea767c85cb54996b6333e7d53afe249` preserves deployed
`c8c17202c` and adds the cap-grant lookup port, private-entry restriction, and corrupt-action
review correction. Its release binary SHA256 is
`c6ffea7d9d834b0643f85a8f6316bd9d0af40075c12c62c99b3784ef545fb841`.
All three household conductors were restarted on that exact binary with their identities
preserved. Storage remained the previously proved leg-3 binary (SHA256
`68bdf78c82b0b636de977eea4ebad5b94113ef5d89d5a69b52618af39109f693`); the supported storage
restart refreshed conductor connection tokens and all three zome readiness probes passed.

Candidate validation: 359 data/state/integration tests passed; changed-library clippy passed;
release build and formatting passed. All-target clippy is **not green**: unchanged capability
WASM test fixtures have needless-borrow lints. This limitation must remain visible in the
rollout decision.

`just test mesh features/dataplane/federation-version-convergence.feature` first failed during
fixture declaration with `Stamp declared head failed: database is locked` (report
`sprint-report-household-20260929T185058Z-d2733929`). Classified **legitimate caller refused**,
not a security side door closed. The warm retry passed **1 scenario / 15 steps**, including
forged declaration rejection, in 10m36.985s (report
`sprint-report-household-20260929T185355Z-d2733929`). Its organic receipt recorded 612,956 ms
waiting for convergence, five setup writes, and **zero declaration calls during the cure**.
Matthew's earned head `uhCkk9axmHr5LdL7ojz_aljwB9ZvSfDQqXG9EtAMCUJGGoYqKlUOe` displaced
Jessica's later staging head `uhCkkNeUwwickAYV5Ij0TdSFZnvKYC_UOvxclVUYLo56bnQmM99S_` and
both household doorways agreed. The peer-carried counter was absent before and after: this
proves organic convergence, not that the carried-election arm caused it. This run-owned
page is the campaign 1.4 fixture; it is not a claim that the public FCT page has recovered.

**At this checkpoint the rollout candidate was conductor-only.** No fork push, superproject pin move,
or public fleet rollout had occurred. The public acceptance boundary remains Adam's independent
election answer plus both doorways serving the same v2 head and content.

### Separate storage findings preserved for follow-up

The first failure exposed a deferred SQLite transaction that reads before acquiring the writer
slot. A narrow top-level `BEGIN IMMEDIATE` fix, retaining nested savepoints, passed an independent
review and a two-connection WAL regression. Its control reproduced the old database-lock failure.
It was not running during the successful household proof.

Review also identified a courier-path gap: a row with a locally canonical staging election may
never compare a peer's stronger earned declaration; eventual DHT visibility can still converge,
as this run demonstrated. A draft comparison patch retains own-conductor link verification,
record proof, byte availability and the monotonic stamp. It needs its own scenario and review:
first-advertiser inventory selection can hide a later differing peer, and repeated losing evidence
can consume sweep/conductor budget. It is not part of the immediate rollout candidate.

Both storage drafts are preserved, unapplied, at
`genesis/a2o/reports/recovery/fct-convergence-20260929/storage-followup.patch`, SHA256
`4010851314f99131bb283faddb84f1d599cf3509e5905f70561023163f0f3c34`.
The three shared storage source files were restored to their pre-draft bytes. Raw proof artifacts
are in the same recovery directory; the organic receipt is
`genesis/a2o/reports/dataplane/carried-election-organic-receipt-2026-09-29T19-04-25-763Z.json`.

### Pipeline rehearsal: publication remains gated on household proof

The operator clarified that publication must follow local or hybrid proof of the complete
delivery path, to avoid spending fleet pipeline wall time finding locally reproducible failures.
The isolated integration candidate pins conductor `26374c316`; no remote push has occurred.
The selected local gates passed (schema-DNA, sweettest compilation, DNA extern lints, storage,
steward-node and Cargo coverage). Those checks do not substitute for delivery proof.

The broader `epr-app-deliverability.feature` run on candidate `60ca98d88` failed: **3/5 scenarios
passed**, 73 steps passed, 2 failed and 27 skipped, in 12m10.966s. Report:
`sprint-report-household-20260929T195832Z-60ca98d8`. The running-renderer upgrade left the
elohim.host doorway serving the old browser pointer past 75 seconds; Jessica and James still
held that old storage head afterward. Jessica's recovery scenario then timed out on a two-second
health read, with generic cleanup-hook timeouts. All three storage peers subsequently answered
health 200. This run overlapped build gates and a CPU-heavy sweettest; recovery timing needs a
quiet-host repeat, but the persisted pointer divergence is not explained away by that overlap.

The responder assembled torn evidence during successive browser/server declarations: James
received a record for `ak3kt` with an election for `igIH`, then a record for `igIH` with the final
election for `b4Zz`. Its own conductor correctly refused both mismatches. The responder also
waited on two sequential five-second calls within a ten-second requester deadline. The narrow
repair under test runs those reads concurrently and omits mismatched election evidence; it
preserves own-conductor verification, refusal memoization and matched election-only supply.
It does not incorporate either separate storage draft above.

Story-graph seam: app deliverability / between successive author declarations -> receiving peer
adopts verified head / missing node: head-record supply carries a matching record/election pair
within one responder budget, probed by concurrent slow-call and torn-pair regressions / current
state: reproduced locally, repair pending executable and household proof.

Two deterministic strict-CI harness failures were also reproduced locally and fixed in
`461d4ed39`: escape the literal slash in the household step expression, and declare the absent
native doorway TLS capability so its opt-in scenario is held. The exact CI selector dry-run
resolves all 976 steps across 132 scenarios. Declaring TLS available without supplying its
endpoint still fails; no TLS runtime proof is claimed.
