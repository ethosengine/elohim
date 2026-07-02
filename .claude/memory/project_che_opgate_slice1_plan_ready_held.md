---
name: project_che_opgate_slice1_plan_ready_held
title: Che op-gate Slice-1 — offline spine landed, live legs held
description: Che op-gate offline spine landed (ff31cd170..HEAD, fail-closed); all doorway deploys run DEV_MODE=true — no valid Che-facing enforce node; live-mesh legs held.
metadata:
  node_type: memory
  type: project
  originSessionId: a0632634-fa0a-4b7a-8546-a7e1a5d6f0ab
---

The **Che keyless peer-client Slice-1 op-gate** governance spine — plan
`genesis/docs/superpowers/plans/2026-06-26-che-keyless-peer-client-slice1-governance-spine-plan.md`
(implements spec `2026-06-26-che-keyless-governed-peer-client-design.md` §10) — was **EXECUTED**
2026-06-26 via `subagent-driven-development` after the omnibar Phase-2 hold cleared. The **offline
code spine is committed** on `feat/frontend-eyes-sprint`, range **`ff31cd170..621778123`** (5
contiguous commits; base = omnibar tip), each fresh-implementer + per-task-reviewer Approved:

- **T1** `7a64cfe45` (seeder TS): bounded `seed-delegates-compute` factory → POSTs Matthew→Che
  self-contract to the GATED storage seed endpoint (NOT `/api/v1/commitments`); minimum-bounds guard
  (`SAFE_CEILINGS={commons,community}` allowlist; reject `epr_scope:["*"]` w/o finite rate+ttl).
- **T2** `97438aa3f` (storage): flag-gated dev-seed endpoint (`ALLOW_SEED_DELEGATES_COMPUTE=1`, 403
  default) writes `delegates-compute` into `mishpat_commitments` (synth `dht_anchor_hash`, `dev-seed`
  provenance); revoke variant; KEPT OUT of `build_manifest()`.
- **T3** `8ec0bd207` (storage): `authorize_operation(pool,req,signed_at)` 3-arg + `POST
  /api/v1/authorize-operation` (also off `build_manifest()`); scope-in-SQL lookup, explicit
  `performer==recipient` guard (NOT in shared `bounds_validator` — `[C3]`), reuses `bounds_validator`
  via `EventForValidation`+`ProjectionCommitmentFetcher` (fails closed on NULL anchor).
- **T4** `95629aa27` (doorway): pre-dispatch op-gate on `POST /db/content`+`/db/content/bulk`; 3-mode
  flag (`off`/`observe`/`enforce`, default off, `OpGateMode` in `doorway/doorway-service/src/config.rs`);
  per-request fail-closed; performer from verified JWT `human_id` (never client `X-Agent-Cid`); generic
  client 403 + detail to logs; forwards user Bearer; `CHE_FACING` boot-refusal in `config.rs::validate()`.
- **T7** `621778123` (docs): Che-facing deploy-posture honesty gate in
  `genesis/orchestrator/manifests/doorway/README.md` — env contract + boot-refusal pointer; **flips no
  live manifest**.

**LOAD-BEARING TENSION surfaced (operator/architect decision):** all four doorway deploys
(`alpha/prod/staging/staging-read.yaml`) run `DEV_MODE: "true"` → **none is a valid Che-facing
`enforce` node as-is**. The Che-facing dogfood node is a distinct posture (dedicated deploy, or alpha
sheds `DEV_MODE` → breaks portal-handoff fixtures); flipping it to `enforce` is operator-owned + coupled
to seeding the commitment on that node (held leg). (Incidental: prod/staging `DEV_MODE=true` contradicts
alpha's own "NEVER set on staging/prod" note — pre-existing drift.)

**STILL HELD (NOT executable without a live `hc:start:seed` M/J/J stack + matthew dev credential):**
Phase-0 runtime probes (which JWT claim == seeded recipient `[C8]`; storage `--features p2p`),
**Task 5** (keyless-Che driving loop → `distribute_shards` → card `stewardingCollectives>0`),
**Task 6** (governed-distribution a2o, 2 consecutive green). Resume these when the live mesh is up.

**REVIEW + FINISH (all done 2026-06-26):** whole-branch review (Opus, `ff31cd170..HEAD`) = "With fixes,
NO Critical" — spine confirmed fail-closed, performer unspoofable, both endpoints off `build_manifest()`,
boot-refusal closes the dev_mode forge path. ONE fix wave (Opus, 3 commits `75106cbfc 0fa7ee17c 00f36740b`)
fixed: observe-mode 403'd the no-credential case (D2 violation) → now forwards; dev-seed silently un-revoked
on re-seed → `perform_seed` guard (NOT the shared `upsert_with_anchor` — it's the real DHT-projection path);
seed:delegates:dev wrong host; +3 minors. Re-review = **Ready to merge: Yes**. Doc-nit fixed `8a467c8bd`.
story-harvest → `genesis/a2o/features/resilience/governed-distribution.feature` (@wip, `dd76ec359`).
Final spine = 10 commits `ff31cd170..HEAD`. `finishing-a-development-branch`: branch is a 94-commit SHARED
shift branch → kept as-is; **integrator owns the dev-merge (commit-only; never `git push`)**.
See [[feedback_commit_only_integrator_pushes]], [[project_resilience_weave_sprint_landed_data_starved]],
[[feedback-cleanup-toward-p2p-dataplane-trajectory]].
