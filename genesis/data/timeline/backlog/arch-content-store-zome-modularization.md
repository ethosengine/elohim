---
id: "backlog-arch-content-store-zome-modularization"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "content_store coordinator zome modularization — decompose the ~13.4k-line god-file into domain modules (coordinator-only, hot-swappable)"
slug: "arch-content-store-zome-modularization"
written: "2026-07-11"
author: "rust-architect (source-file-loc-ceiling architecture finding b1775267b042); adopts prior stashed draft dated 2026-07-10"
status: "backlog"
priority: "medium"
tags: [architecture, refactor, holochain, coordinator-zome, content-store, loc-ceiling, mod-decomposition]
relatedNodeIds:
  - arch-dataplane-refactor-backlog
  - backlog-doorway-http-rs-modularization
cites:
  - elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  - .claude/epr-meta/policies.yaml
shift_objective: |
  Decompose elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs
  (13,352 lines, 173 #[hdk_extern] fns) into domain modules under
  content_store/src/, one bounded wave at a time. This is a COORDINATOR zome
  (crate content_store; entry types live in the sibling content_store_integrity
  crate — verified: no hdk_entry_helper/hdk_entry_types/hdk_link_types in this
  file) — the decomposition does NOT touch integrity, so the DNA hash does not
  move and the change ships via the update_coordinators hot-swap path (cheap, no
  re-key, no DHT partition). Per wave: (1) git-mv one domain's #[hdk_extern] fns +
  private helpers + IO types into a new module file; (2) `pub use module::*;` from
  lib.rs so the wasm extern surface is byte-identical; (3) gate with
  RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release, cargo fmt
  --check, cargo clippy -- -D warnings, cargo test (nextest is NOT installed in
  this container); (4) run the DNA sweettest (RUN_SWEETTEST=1) to confirm the
  extern surface and behavior are unchanged. Do NOT merge waves — one domain per
  commit so a regression bisects cleanly. Ratchet the loc-hard ceiling DOWN in
  .claude/epr-meta/policies.yaml as lib.rs drains; never up.
---

# content_store coordinator zome modularization

## Finding

`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs` is **13,352 lines**
with **173 `#[hdk_extern]` functions** — roughly 1.9× the `source-file-loc-ceiling`
policy hard ceiling of 7,000 lines (`.claude/epr-meta/policies.yaml`,
`source-file-loc-ceiling@1`, `loc-soft: 3000` / `loc-hard: 7000`). The policy
classifies a first-party file past the hard ceiling as a god-file: line-number
invariant anchors drift every edit, two-location sync traps grow ~1,000 lines
apart, and the file outgrows what one context window can review. The policy's
prescribed response is exactly this artifact — **canonicalize a modularization
plan into the timeline backlog and drive it as bounded work; never refactor
mid-edit.** This entry is `status: backlog` (no code touched); it will not be
picked up by `/shift` until it reaches `status: refined` per timeline CONVENTIONS.

## Refactor-safety class (READ FIRST)

**This is a COORDINATOR zome, not an integrity zome.** Crate `content_store`
(`crate-type = ["cdylib", "rlib"]`) holds coordinator functions only; all entry
types live in the sibling `content_store_integrity` crate. Verified 2026-07-11:
`grep -E 'hdk_entry_helper|hdk_entry_types|entry_defs|hdk_link_types' lib.rs`
returns **nothing** — no integrity type definitions live in this file. That places
the whole decomposition in the **lowest-risk refactor class** the policy names:

| Class | Applies here? | Consequence |
|---|---|---|
| Integrity-zome change | **No** — no integrity edits | Would move the DNA hash → reinstall/partition trap; ride a deliberate DNA-lineage event only |
| Coordinator-zome change | **Yes** | DNA hash unchanged; ships via `happ_manager::sync_coordinators` → conductor `update_coordinators` hot-swap (no re-key, no DHT churn), gated by `ALLOW_COORDINATOR_UPDATE` |
| Plain native Rust | **N/A** (WASM build) | — |

Moving `#[hdk_extern]` fns between modules **within the coordinator crate** does
not change the DNA hash (the hash covers integrity zomes + modifiers only). The
mechanical invariant to preserve is the **wasm extern surface**: every
`#[hdk_extern]` symbol must remain exported. The safe pattern is to move the fn
(with its `#[hdk_extern]` attribute) into the new module and re-export with
`pub use <module>::*;` from `lib.rs`. Verify byte-identity of the extern set per
wave; a sweettest that exercises the moved calls is the behavioral backstop.

**Gates (WASM workspace — keep plain cargo, exempt from CARGO_TARGET_DIR rule; do
NOT override the target dir via CARGO_TARGET_DIR — `hc dna pack` canonicalizes
`./target`):**
```
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
cargo fmt --check && cargo clippy -- -D warnings && cargo test    # nextest NOT installed in this container
RUN_SWEETTEST=1 <DNA sweettest>   # extern-surface + behavior unchanged
cargo test export_bindings         # only if any ts-rs #[derive(TS)] type moves crate
```

## Prior extraction (the pattern already exists)

lib.rs already `pub mod`-declares 12 extracted sibling modules —
`bootstrap_steward`, `manifest`, `attestation`, `governance_action`,
`feedback_signal`, `attention_tending`, `republish_epr`, `migration`,
`healing_impl`, `healing_integration`, `providers`, `gate`. This plan
**continues that established pattern** for the ~13.4k lines of `#[hdk_extern]`
blocks still living inline in lib.rs. No new mechanism is invented; the
decomposition just drains the remaining domains out of the root. Sibling
precedent for this whole loc-ceiling-decomposition class:
`backlog-doorway-http-rs-modularization` (doorway `server/http.rs`) and
`p2p-mod-loc-ceiling-decomposition`.

## Proposed module decomposition

Grouped by domain, following the file's own `// ====` section banners. Line
ranges are approximate (they drift with every edit — the file has grown ~450
lines since the 2026-07-10 draft; do NOT treat ranges as invariants; **re-derive
per wave from the section banners**, which are stable at
`grep -nE '// ={4,}' lib.rs`). Suggested waves are ordered low-risk →
higher-coupling.

### Wave 1 — leaf domains (self-contained, low cross-talk)

| Proposed module | Domain / fns | Approx. lines |
|---|---|---|
| `blobs.rs` | Blob operations — `get_blobs_by_content_id`, `verify_blob_integrity`, `get_blob_variants`, `get_blob_captions` | ~3848–4002 |
| `shards.rs` | Shard generation/storage/watermark — `generate_shards`, `store_shard`, `verify_shard`, `get_shard`, `get_shard_metadata_only`, `should_verify_shard_sample`, watermark helpers, `calculate_sha256`, shard-manifest mgmt, DID-based storage discovery | ~9049–9306, ~10094–10261, ~11549–12058 |
| `knowledge_map.rs` | KnowledgeMap CRUD — `create_knowledge_map`, `get_knowledge_map_by_id`, `query_knowledge_maps` | ~8418–8609 |
| `path_extension.rs` | PathExtension CRUD — `create_path_extension`, `get_path_extension_by_id`, `query_path_extensions` | ~8609–8799 |
| `export_migration.rs` | Migration exports — `export_schema_version`, `export_all_content`, `export_all_paths_with_steps`, `export_all_mastery`, `export_all_progress`, `export_for_migration` | ~8252–8418 |

### Wave 2 — economy (shefa / REA / hREA)

| Proposed module | Domain / fns | Approx. lines |
|---|---|---|
| `points.rs` | hREA point system — `earn_points`, `get_my_point_balance`, `get_my_point_history`, `get_contributor_dashboard`, `get_my_contributor_dashboard`, + point/impact helpers | ~6728–7528 |
| `steward_economy.rs` | Steward economy — `create_steward_credential`, `get_steward_credential`, `get_credentials_for_human`, `create_premium_gate`, `get_premium_gate`, `get_gates_for_resource`, `grant_access`, `check_access`, `get_my_access_grants`, `get_steward_revenue_summary`, `create_steward_revenue` | ~7528–8252 |
| `rea.rs` | REA coordinators — Agreement / Commitment / EconomicEvent / intent-driven event creation | ~12192–12841 |
| `shefa_insurance.rs` | Insurance mutual — risk profiles, coverage policies, claims, adjustment reasoning | ~10261–10529 |
| `shefa_requests_offers.rs` | Requests & Offers — `create/get_service_request`, `service_offer`, `service_match` | ~10529–10755 |
| `shefa_flow.rs` | Flow planning CRUD — `create_flow_plan/budget/goal/milestone/scenario/projection`, `create_recurring_pattern`, `get_flow_plan`, `get_plans_for_steward` | ~11293–11414 |

### Wave 3 — learner journey

| Proposed module | Domain / fns | Approx. lines |
|---|---|---|
| `mastery.rs` | Content mastery — `get_my_mastery`, `get_my_all_mastery`, `upsert_mastery`, `get_mastery_level_index` | ~864–1026 |
| `progress.rs` | Progress tracking — `start_path_progress`, `complete_step`, `complete_path`, `get_my_path_progress`, `get_my_all_progress`, `get_progress_by_status`, `get_my_progress_summaries` | ~4881–5358 |
| `attestation_gating.rs` | Assessment history & attestation gating — `grant_attestation`, `check_attestation_access`, `get_assessment_history`, `check_attestation_eligibility`, `grant_attestation_with_mastery_check`, `check_step_access`, `check_path_step_access` | ~5358–5857 |
| `practice_mastery.rs` | Practice pool & mastery challenge — `get_or_create_practice_pool`, `refresh_practice_pool`, `add_path_to_pool`, `get_pool_recommendations`, `check_challenge_cooldown`, `start_mastery_challenge`, `submit_mastery_challenge`, `get_challenge_history` | ~5857–6728 |

### Wave 4 — stewardship & recovery

| Proposed module | Domain / fns | Approx. lines |
|---|---|---|
| `custodian.rs` | CustodianCommitment CRUD — `create_custodian_commitment`, `accept_custodian_commitment`, `query_custodian_commitments`, `batch_accept_commitments`, `batch_update_commitments`, relationship→commitment auto-create hook | ~8799–9049, ~9306–9476, ~10125–10252 |
| `emergency.rs` | Emergency protocol — `activate_emergency_manual`, `activate_emergency_trusted_party`, `submit_consensus_vote`, `check_consensus_status`, `reconstruct_content_from_shards`, `hash_passphrase`, `notify_emergency_contacts` | ~9476–9861 |
| `category_override.rs` | Category overrides — `create_category_override`, `query_category_overrides`, `validate_category_access`, `revoke_category_override` | ~9861–10094 |

### Wave 5 — content core (highest coupling — do last)

| Proposed module | Domain / fns | Approx. lines |
|---|---|---|
| `content_crud.rs` | Content CRUD + queries — `create_content`, `update_content`, `get_content`, `get_content_by_id`, `check_content_ids_exist`, `batch_get_content_by_ids`, `get_content_by_type[_paginated]`, `get_content_by_tag[_paginated]`, `get_my_content`, `get_content_stats`, existence/link helpers | ~2352–2547, ~3408–3848, ~4805–4964 |
| `content_head.rs` | Content HEAD authority (notary Leg 1) — `resolve_content_head`, `declare_content_head`, `build_content_head_output`, `gather_content_chain`, `resolve_root_author` | ~2548–2851 |
| `import_batch.rs` | Import batch processing — `queue_import`, `process_import_chunk`, `get_import_status`, `list_import_batches`, `bulk_create_content`, `create_import_batch_index_links` | ~2851–3408 |
| `relationships.rs` | Relationships & graph — `create_relationship`, `get_relationships`, `query_related_content`, `get_content_graph`, `get_relationship`, `on_relationship_updated` | ~4595–4801, ~9306–9476 |
| `paths_deprecated.rs` | **DEPRECATED** path/chapter/step ops — `create_path`, `add_path_step`, `batch_add_path_steps`, `get_all_paths`, `delete_path`, `get_path_with_steps`, `get_path_overview`, chapter ops, path/step update ops | ~4008–4595 |

### Cross-cutting (extract alongside or keep thin in lib.rs)

| Proposed module | Domain | Approx. lines |
|---|---|---|
| `wire.rs` | Integrity→wire conversion helpers (`*_to_wire`) — content, path, relationship, agreement, commitment, economic_event, steward, premium_gate, access_grant, revenue, custodian, insurance, service | ~206–847 |
| `io_types.rs` | Input/Output type defs (Content, Blobs, Relationships, Humans, Agent, Progress, Attestations, Paths, Shards, Emergency, Category, Mastery) — **or** colocate each block into its domain module (preferred — keeps types next to their fns) | ~1784–2352 |
| `doorway.rs` | Doorway integration — `__doorway_cache_rules`, `__doorway_import_config`, `warm_cache`, post-commit signal projection | ~1200–1784, ~10755–11293, ~11414–11520 |
| `cross_dna.rs` | Cross-DNA bridge — `issue_attestation_via_imagodei`, `query_effective_revocation_for_key`, `query_effective_identity_freeze_for_human` | ~864–1117, ~3577–3725 |
| `init.rs` | DNA init — `init`, `init_flexible_orchestrator` | ~1117–1200 |

`post_commit` and any dispatch tables it holds should stay in lib.rs (or a
dedicated `post_commit.rs`) since it fans out across many domains — extract last
and carefully, as it is the one true cross-domain hub. Note there are also
inline `#[cfg(test)]` modules in lib.rs (e.g. `canonical_head_selector_tests` at
~2854) — move each test module together with the domain fns it exercises.

## Readiness notes

- **Ready now.** No blockers. Coordinator-only, pattern already in-repo, gates known.
- **Sequencing.** Waves are independent — pick any, but run **one domain per commit**
  so a regression bisects to a single move. Content core (Wave 5) is highest-coupling
  (shared link/existence helpers, `post_commit` fan-out) — do it last.
- **Extern-surface invariant.** Before/after each wave, diff the set of exported
  `#[hdk_extern]` symbols (e.g. `grep -h '#\[hdk_extern\]' -A1` across the crate,
  sorted) — it must be byte-identical (baseline: 173 externs). The
  `pub use module::*;` re-export from lib.rs is what preserves it.
- **ts-rs caveat.** If any `#[derive(TS)]` type moves crate (it should NOT — types
  live in `content_store_integrity`/`shefa-types`, only fns move here), re-run
  `cargo test export_bindings` and sha256-diff the generated TS. Intra-crate module
  moves do not affect ts-rs import paths.
- **Ceiling ratchet.** As lib.rs drains below each threshold, ratchet `loc-hard`
  (and eventually `loc-soft`) **down** in `.claude/epr-meta/policies.yaml`
  (`source-file-loc-ceiling`) — never up. This is the policy's own instruction.
- **Deprecated cluster.** `paths_deprecated.rs` groups three sections the source
  already marks `(DEPRECATED)` (Learning Path / Chapter / Path Update ops). Extract
  them together so a later removal-decision is a single-file delete, not a scatter-gather.
- **Managed-surface note.** `cites:` includes `lib.rs` (a moving target) — the file
  will keep growing until this work lands; that is expected and does not invalidate
  the plan. Re-derive line ranges from banners, not from this doc.

## Definition of done

Every wave: green `cargo build --release` (WASM flags), `cargo fmt --check`,
`cargo clippy -- -D warnings`, `cargo test`, and a `RUN_SWEETTEST=1` DNA
sweettest proving the extern surface + behavior are unchanged. Whole objective:
lib.rs under the (ratcheted) `loc-hard` ceiling, extern-symbol set byte-identical
to the pre-refactor baseline (173), and the ceiling lowered in policy to lock the
gain.
