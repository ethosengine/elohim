---
name: project_regenesis1_landing_traps_receipt_and_epr_closure
title: "Re-genesis 1 landing traps: serving receipt and the epr closure-CID flip"
description: "2026-11-06 overnight: two pre-push legs that a whole-repo crossing trips — the serving-receipt rule (deliverability story must pass on the final tree) and gate-source-state demanding a bafkr4i closureCid from an epr binary built pre-crossing; cures and the device re-ingest path."
metadata:
  node_type: memory
  type: project
  originSessionId: 184beb01-43bc-4c74-9212-9f7189301998
  modified: 2026-11-06T11:26:29.964Z
---

Two legs of `.husky/pre-push.bash` bit the re-genesis 1 crossing after every project gate was green:

1. **Serving receipt** (`genesis/orchestrator/scripts/serving-receipt.mjs`): editing `features/dataplane/{epr-app-deliverability,served-shell-boots}.feature` or their steps makes the hook demand a household receipt — a brit ValidationAttestation signed by THIS workspace, or a `sprint-report-household-*.json` whose SUT parts (a2o, elohim/sdk, scripts/ci, app/scripts, pnpm-lock, hc-mesh.sh, elohim-storage, doorway-service, elohim-render, conductor pin, elohim/holochain/dna, DNA hash, fixture) equal the tree — with every deliverability station passed. It is refuse-class, not warn. On a crossing the story can only pass on the FINAL tree (client + server + DNA together), so the a2o closure and the DNA closure land as one push after the household run; a report produced on another device with equal SUT parts is admissible here.

2. **epr closure CID** (`genesis/orchestrator/gate-source-state.mjs`): after the crossing it demands a `bafkr4i` closureCid, but the device's landed `epr` (bin-store, pre-crossing) still prints `bafkrei`, so `gate-runner.test` reds "source closure dirty" on every device. Cure: `just gate eprfs` builds + ingests epr from the crossed source; run gates with `EPR_BIN=<slot>/debug/epr`; the hook lands the candidate itself after a dev push (`epr bin land … --commit <sha>`, unpublished commits defer to the next session arrival). A candidate ingested while the "before" reader was the old epr is marked dirty-tree (the two readers disagree) — re-run `_eprfs-ingest-around build` once the new debug/epr exists.

**How to apply:** on a big crossing, schedule the household run + epr re-ingest BEFORE the last closure's push, not as a Phase 5 item; run `just gate eprfs` first on every device so `EPR_BIN` can point at the crossed epr. Related: [[project_ci_rust_style_method_bootstrap_lockout]], [[feedback_big_crossing_working_code_first_verify_after_dust_settles]].

3. **`cargo test --lib` is not the storage gate.** An implementer handed back R1 with `cargo test --lib` 4930/0 as evidence; the pre-push elohim-storage gate (2597 s) then failed one INTEGRATION test under `tests/blob/` that `--lib` never compiles (a producer-shape change, `GossipPublish` → `AnnounceCustody`). A storage change is verified by `just gate elohim-storage` or at least `cargo test --test <binary>` for the `tests/` binaries that touch the changed seam; the implementer brief must say so, and the orchestrator reads `verified_by` for which targets ran, not just the count.

**2026-11-11, two more K8 landing traps.** Each cost a full push cycle.
(1) A new Cargo path dependency must also go into the watch list of every gate manifest whose closure contains the dependent crate, not only the owning one. `genesis/orchestrator/build-inputs-closure.test.mjs` refuses otherwise. Run it locally before landing.
(2) Every new markdown doc under the corpus needs an `id:` slug. The `doc-frontmatter-at-birth` hook only advises, but eprfs's `flow_cites::live_corpus_migrate_reports_zero_pending` fails the gate. Run `epr flow cites migrate --apply` before committing a new doc.

**2026-11-12: artifact extraction and recording need a live mesh receipt before landing.** Unit tests and two reviews passed while three defects reached dev, each found only by `EPR_BIN=<new epr> just mesh build doorway|storage`:
- V8 builtins' inline jump tables;
- a build script's OUT_DIR file pulled in through `include_str!`;
- a nonexistent `.git/HEAD` listed as a `rerun-if-changed` path, plus K8's unmapped `AVX_IFMA` on storage.
Build the epr from the branch, copy it outside the pool, record both mesh binaries, and require both to record before landing any K8/K11-class change.
