---
id: f19-household-proof-handoff-20260929
status: handoff
date: 2026-09-29
---

# Codex handoff: prove F19 leg 3 on the household mesh, then prepare the push

A portable brief for a standalone Codex session in `/projects/elohim` (branch `dev`). Recheck every
fact below before relying on it. It was written at 12:55 UTC on 2026-09-29 from the state described
here.

## Request

1. **Prove** on the household mesh that F19 leg 3 (`600dff891`) works: a steward's authenticated
   **earned** head carries its reach to the other peers. The FCT v2 course path, left `public` on
   jessica and james and `commons` on matthew, must converge. The four disclosure holes must stay
   closed.
2. **Record** the result: the F19 row, the security backlog item, and a habit delta if a habit's
   check was exercised.
3. **Stop before pushing.** Report back with the evidence. The operator decides the push, because
   pushing `dev` dispatches an edge build that rolls the alpha fleet.

Out of scope: leg 4 (device delegation), the integrity-layer earned-tag gate (it moves the DNA
hash), and the reach design session (`genesis/data/timeline/backlog/reach-delivery-subsystem-solutioning-bootstrap.md`,
a separate operator-led pass).

## State of `dev` (8 commits ahead of `origin/dev`, none pushed)

| Commit | What |
|---|---|
| `4687e4111` | fix: authenticate earned election standing (coordinator-only; the earned-link author must be the id's immutable first root author, a valid in-scope delegate, or the progenitor) |
| `1e9368935` | fix: close earned election review bypasses |
| `251c851b5` | backlog: reach defect ledger (docs only) |
| `600dff891` | **feat(storage): re-land earned reach widening (F19 leg 3)**; the subject of this proof |
| `1e2774b01`, `c0629ab12`, `b2466cd5d`, `10b551c46` | backlog: reach item updates (docs only) |

**`600dff891` in brief** (files: `elohim/elohim-storage/src/db/content_diesel.rs`,
`services/head_adoption.rs`, `services/courier_obey.rs`, `tests/api/content_read_reach_gate.rs`, and
two backlog rows). Widening via a `HealCanonical` stamp happens only when all of these hold:
- the adopted version is complete and verified, carrying its own non-empty body or blob;
- an absent incoming `blob_cid` would not retain an old blob pointer (`content_diesel.rs:~2488-2529`);
- identity fields are present, and title and description are now carried in adoption patches.

It also:
- refuses the whole stamp as `ReachNarrowing` when the incoming reach is narrower
  (`content_diesel.rs:~2309`);
- keeps the authenticated election-ordering floor when an author `Declare` moves the row, in both
  the stamp path and the production `upsert_with_anchor` path (`~1476`, `~2444`);
- widens only the edges the head restates in `patch.relationships`, and none when that is absent.

Verification: `just gate elohim-storage` `EXIT=0` (4,238 library tests plus integration suites). The
real-server test `tests/api/content_read_reach_gate.rs:~581` shows anonymous `200` after a
legitimate earned widen. No DNA or integrity change.

The previous leg's working notes (the report, the adversarial review `leg3-review.md` and the run
log) are in `/tmp/claude-0/-projects-elohim/2041165d-7a5c-48ab-9a3e-7bb078885c99/scratchpad/`.
They may have been cleaned; the table above carries what matters.

## The household mesh as found

- **Running:** fixture `genesis/local-dev/household-dowell`, peers matthew, jessica and james.
  - Conductors run the pinned fork `hc-fork-c8c17202c40d`, up since about 2026-09-28 05:15 UTC.
  - Storage runs on ports 8090, 8091 and 8092.
  - Doorways are on 8888 and 8889.
  - The portal is on 8081.
- **The storage binary is stale.** `…/elohim__elohim-storage/mesh-bin/elohim-storage` was built
  2026-09-28 01:19, before `600dff891`. Rebuild it in the pool slot with
  `cargo build --bin elohim-storage` (with the `CARGO_TARGET_DIR` and `RUSTFLAGS` the gate uses;
  elohim-storage keeps the custom getrandom flag). Put it where `mesh-bin` expects, then
  `just mesh storage-restart matthew jessica james`. `just mesh preflight` refuses a stale pool
  binary; don't override that with `MESH_ALLOW_STALE_BINARY`, since the new code is the thing
  being proven.
- **Claim the berth first** (`BERTH_CLASS=verify BERTH_TTL=…`; see CLAUDE.md). Release it after.
  `CARGO_BUILD_JOBS=1` for storage; the RAM guard sheds larger builds.
- **The DNA is unchanged by leg 3.** `c8fb553a8` (the lamad init cure, `[build:dna]`) is already on
  `origin/dev`. If `just mesh` refuses a stale coordinator, that's a separate finding: report it and
  don't paper over it.
- After a storage restart, reads can answer `503 catching-up` for a while. Wait for anchored heads,
  not just TCP (backlog `household-mesh-harness-honest-readiness`).

## What the proof should show

There is no dedicated F19 scenario yet. `genesis/a2o/features/lms/commons-path-steward-publish.feature`
is the closest, and it is `@wip`. Story-first is the house rule (root `CLAUDE.md` §Story-First): add
or un-wip a scenario whose steps assert the following, or at minimum capture the same assertions as
recorded probes.

1. **Convergence.** Find the FCT v2 course path id in the genesis content. After the steward
   (matthew) publishes or re-declares the earned head, every peer's `GET /db/content/{path-id}` and
   its `/head` report the steward's head and reach `commons`. Anonymous reads on jessica (8091) and
   james (8092) return `200`.
2. **No regression for private content.** A `private` or `intimate` atom stays refused anonymously
   on every peer.
3. **The holes stay closed**, where the mesh can exercise them:
   - an older earned head does not re-open a row the author narrowed;
   - a head without its own body or blob does not widen;
   - an edge the head doesn't restate stays narrow.

Classify every red before changing anything. Is it a **side door closed** (a caller or fixture
relying on an undeclared path: fix the caller) or a **legitimate caller refused** (fix the
verifier)? Never loosen a gate to make a red pass. This is P9 in the reach backlog item.

## Recording

- F19 row in `genesis/data/timeline/backlog/lamad-teacher-authoring-backlog.md`: status plus the
  sprint-report path.
- `genesis/data/timeline/backlog/security-earned-election-tier-unauthenticated.md`: "Current
  decision".
- If `dataplane-convergence` or `reach-enforced-everywhere` checks ran, add a one-line DELTA in the
  habit atom (`elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`,
  `.epr-meta/reach-enforced-everywhere.habit.md`) and re-project with
  `.claude/scripts/habits-project.py`. A flip needs evidence, never intention.

## House rules

- Never run `kubectl`.
- Never push, force-push, rebase or amend.
- Stage by path only (`git add -- <path>`; never `-A`). The tree is shared, and other sessions leave
  unrelated modified files in it (`.claude/data/*`, `.eprfs/status/*`, memory files, the `sophia`
  and `holochain-conductor` gitlinks). Don't touch those.
- Commit messages end with `Co-Authored-By: Codex <noreply@openai.com>`.
- Echo `EXIT=$?` on its own line after cargo, gate or mesh commands. Never judge a run from piped
  output.
- **One driver.** A background Claude session (`5037f04c`, "Lamad foundations course visibility")
  orchestrated legs 1–3 and is still alive. The operator should `claude stop 5037f04c` before this
  session starts, so two agents don't drive the mesh.

## Report back

Include:
- the storage binary's sha and build time;
- the berth claim;
- the per-peer reach and head for the course path, before and after;
- the anonymous read results;
- each hole probe's result;
- the classification of any red;
- the sprint-report path;
- the files and commit(s) created.

Recommend push or no-push, with the reason.
