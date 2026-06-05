# HANDOFF — Household Formation Ceremony, Stage 1: COMMITTED, ready for integrator dev-merge + the CI run that unlocks Task 10

_Last updated: 2026-06-05 (~03:30) · Author: Claude Opus · Branch: `shift/a2o-greenup` · Session mode: **orchestrating** (subagent-driven execution; the 18-commit local diff is the work product — this doc is the integrator's checklist)_

> **Previous handoff (2026-06-04, peer-native OAuth portal-handoff): DELIVERED.** All 8 portal commits verified present in `origin/dev` (`6ada3ad55`, `0e9ef8357`, `6011d00d4` ancestry-checked 2026-06-05). Nothing from it remains open; see git history for its decisions (redemption-callback ratified; Session Transfer Store reuse; client-driven redirect; conductor stays authority).

## Goal

Land the **household formation ceremony** (design session 2026-06-04 → spec → stage-1 plan → execution): a family (matthew/jessica/james) forms a household through real multi-agent zome calls — collective + affirmed memberships + anchored custody reciprocity — driven by the seeder as each persona's own conductor agent, projected to storage, validated by a2o on `household-nodes`. Fork settled: **emergent reciprocity + explicitly-marked interim fixtures** (fixtures retire at Task 10).

Canonical docs (all committed):
- Umbrella seed: `genesis/docs/content/elohim-protocol/architecture/2026-06-04-qahal-epr-household-lattice-design.md`
- Spec: `genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md` (§1 = the 8 settled decisions)
- Plan: `genesis/docs/superpowers/plans/2026-06-04-household-formation-ceremony-stage1.md` (Tasks 1–10)

## Current Progress — VERIFIED against repo state

**Branch `shift/a2o-greenup` is NOT pushed** (no `origin/shift/a2o-greenup` exists); `origin/dev` tip is `04817e6e9`. **Local-only commits = `89d86805c..HEAD` (18):** 2 design docs (`89d86805c` lattice+spec+backlog theses, `fa19ec4ee` plan), 15 stage-1 commits, 1 co-session a2o fix (`1faa8315e`).

Stage-1 tasks 1–9 ALL complete, each through implementer → spec review → quality review (+ fix rounds):

| Task | Commits | Evidence |
|---|---|---|
| 1 Fixture triad pairs + provenance | `a934056dd`, `ceb651a55` | seeder vitest 8/8 |
| 2 `issue_household_invite` + `affirm_membership` (imagodei) | `6462ffbad`, `9cb43955b` | sweettest 2/2 (`qahal_formation_test`); TOCTOU + clock-trust documented; projector dedup verified (UNIQUE(h_app_id,collective_id,human_id) + upsert) |
| 3 Charter→projection (`governance_layer='family'` + slug-alias merge) | `1a86f0a9a`, `e1d5fb24a` | storage lib 1341/0; redelivery idempotency asserted |
| 4 Soft action-gate (custody-blob conductor round-trip, diesel fallback; update_state anchor-gated) | `1e5bd0ab5`, `fca614344` | rea_commitment tests 24/24 |
| 5 DeliveryPeer enrichment patch applied (household_id + provide reaches; patch file retired) | `ec500e1d6` | lib 1350/0 |
| 6 `seed-household-formation.ts` (rung-3 ceremony driver; convergent re-runs via projection probe) | `c322d950e`, `8a902af9a` | vitest 9/9; encodeHashToBase64 for ActionHash |
| 7 Jenkins `Seed Household Formation` stage + artifacts entry + probe env | `a0986fb71`, `bcccb37ef` | stage between Bindings and Upload-M1; STORAGE_URL from INTERNAL_STORAGE_URL |
| 8 `qahal/household` quiltPolicy + qahal codegen quilt-refs gate | `ad70e954e` | manifest:test 32/0, schema:test 83/0, both codegen gates GATE OK |
| 9 a2o spine + retags (household-formation.feature; Susan→Jessica; love-map → live) | `2a4507726` | dry-run 0 undefined; eslint clean |

Final whole-implementation integration review (seams BETWEEN tasks): **coherent, no Criticals.** Charter contract, CID round-trip, provenance wire shape, Jenkins env all verified end-to-end.

## What Worked

- **Subagent-driven execution with two-stage review caught real bugs**: ActionHash returned as raw bytes (would have silently broken all CID scope matching); double-schemed probe URL (`http://https://…` — convergence would never fire); CI re-runs minting a fresh collective nightly (fixed via projection-probe reuse + deterministic salt).
- **Zero new DHT entry types** — the whole feature composes existing primitives (D2 canon held; entity model in spec §2).
- The seed-realism audit before planning: rung-3 identity seeding already existed (`seed-conductor-identities.ts` pattern) — the ceremony driver is its sibling, not an invention.
- Native sweettest env quirks memorialized: `RUSTFLAGS=""` + `BINDGEN_EXTRA_CLANG_ARGS="-I/usr/lib/clang/21/include"` + `just pack` (not `just build`) — see `.claude/memory/project_sweettest_native_build_env.md`.

## What Didn't Work / Don't Repeat

- `GetLinksInputBuilder` isn't this HDK's idiom — use `LinkQuery::try_new` + `get_links(q, GetStrategy::default())`.
- The sweettest crate registers tests via `[[test]]` in `Cargo.toml`, NOT `src/tests/mod.rs`.
- `runProbedSeeder` exports only `CONDUCTOR_URLS` — stage-level `env.X =` assignments are pipeline-GLOBAL in Jenkins; `RESOLVED_DOORWAY_HOST` already carries `https://` (never prefix it).
- Doorway does NOT proxy `/db/*` (it maps `/api/v1/collectives/*` → `/db/collectives/*` internally) — the seeder's DOORWAY_URL probe fallback soft-fails to the create path; only STORAGE_URL actually probes.
- `qahal_collab_t0_test::two_conductor_t0_collab_end_to_end` has a pre-existing peer-exchange-timeout flake — verify YOUR symbols appear in a failure before chasing it.

## Next Steps (integrator, in order)

1. **Push + dev-merge** `shift/a2o-greenup` (18 local commits; repo convention = local fast-forward onto `dev`, no PR). Pre-push hook runs the per-project gates — all were green at commit time. `sweettest-check` fires on a dev-targeted push; budget for the DNA build (and note the `just pack` requirement if running zome tests by hand).
2. **Watch the genesis pipeline run**: the new `Seed Household Formation` stage runs after Agent Bindings, before Upload-M1. Expect the `seed-results-household-formation.json` artifact. First run: probe misses (no stamped CID yet) → creates the collective → affirms jessica + james → stewardship grant → custody layer self-skips (no `M1_BLOB_HASH` yet at that stage position); the triad's custody rows come from the Task-1 FIXTURES this run (marked `fixture: formation-output`). The projector stamps `family-dowell` with the collective CID + `governance_layer='family'`.
3. **TASK 10 — fixture retirement (the one open plan task, precondition-gated).** Trigger: a CI run shows `seed-results-household-formation.json` with `"partial": false` AND `GET /api/v1/commitments?action=custody-blob&state=active` returns triad rows with `metadata.seedGeneration == "ceremony"`. Then execute plan Task 10 (`genesis/docs/superpowers/plans/2026-06-04-household-formation-ceremony-stage1.md`, last task — fully specified, ~30 min): flip the seed-commitments test to expect 2 pairs, remove the 4 fixture pairs from `defaultM1Pairs` (KEEP the M1 matthew↔jessica anti-drift pair + the `fixture?` capability), un-`@wip` the provenance scenario in `household-formation.feature`.
   - NOTE: ceremony custody rows require a run where `M1_BLOB_HASH` is available to the formation seeder — either re-order the stage after Upload-M1 at that point, or run `seed:household` ad-hoc with the env set. Decide at Task-10 time; the seeder reads the env and self-skips gracefully either way.
4. **Verify the a2o spine against the deployed env**: `cd genesis/a2o && npx cucumber-js features/qahal/household-formation.feature` (needs `E2E_DOORWAY_ALPHA` + `E2E_STORAGE_URL`). Scenarios 1/2/4 should pass post-seed; 3 and 5 are `@wip` by design (sponsor surfacing on the participants view; ceremony provenance pre-Task-10).
5. **Stage-2 seams already captured** in `.claude/memory-kit/gap-items/specs__2026-06-04-household-formation-ceremony-design.json` (#13, #14): `ReaCommitmentView.in_scope_of` nulls for conductor-path rows (`views_convert/shefa.rs:140` — fix before any scenario asserts household scope on the wire); `create_stewardship_grant` non-idempotent (timestamped ids → duplicate grants on re-runs; bounded noise, latest-wins).
6. **Future plans (not this branch):** Stage 2 = doorway headless persona auth (`/auth/service-login`) + thin ceremony proxy + `conductor_writes` collective wrappers; Stage 3 = seeder service-agent with `delegates-compute` standing (X-API-Key displacement per admin-key-lifecycle spec). Held design theses in `genesis/data/timeline/backlog/`: dwelling-first-class-entity, household-mobility-seams, capability-arc-stewardship-gradient, witnessed-records-reach-flywheel.

## Housekeeping for the next dev session (not integrator-blocking)

- `/memory-stasis-loop` owed (cleanup gate 141/120 at SessionStart); MAP.md refresh owed (11 seeds changed, incl. the lattice — gap-item `architecture__2026-06-04-qahal-epr-household-lattice-design#1`).
- Uncommitted in-repo memory files from this session (`.claude/memory/feedback_k8s_is_not_the_architecture.md`, `.claude/memory/project_sweettest_native_build_env.md`, refined `project_local_stack_dht_anchor_gap.md`, MEMORY.md index line) — commit with the next memory-hygiene pass (MEMORY.md is co-dirtied by a concurrent session; selective-stage).
- Untracked `genesis/docs/superpowers/held/CLAUDE.md` and modified `genesis/docs/architecture/pillar-bundle-split-runbook.md` predate/parallel this session — not ours; leave for their owners.
