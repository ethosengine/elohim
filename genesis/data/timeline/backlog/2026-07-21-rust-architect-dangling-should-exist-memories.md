---
id: "backlog-rust-architect-dangling-should-exist-memories"
kind: "backlog"
title: rust-architect.md leans on ~34 dangling [[slug]] memories — ~8 are SHOULD-EXIST lessons worth writing
created: 2026-07-21
status: "backlog"
domain: D-memory
source: /memory-ceremony 2026-07-21 (historian adjudication + Phase-4b coherence Explore)
severity: low
---

The substrate-currency ceremony (chronicle `2026-07-21-substrate-currency-rust-architect-memory-kit.md`)
found that `.claude/agents/rust-architect.md` cites 66 real `[[slug]]`s of which **~34 do not resolve**
to any `.claude/memory/*.md` file. The historian adjudicated them: 2 were RENAMED (repointed this cycle),
and the rest split into legitimate FORWARD-ANCHORs (fine per the link-liberally memory convention — the
concept is explained inline) and **SHOULD-EXIST** lessons that have no home anywhere (file or palace) yet
are cited as load-bearing.

The ~8 HIGH SHOULD-EXIST slugs worth writing (cited 2–4× each, several named canonical by the ceremony brief):

- `project_signal_kind_extensible_protocol_class` (4×) — new social moves are `signal_kind` + `resource_classified_as` whitelist entries, never new entry types
- `project_compute_commitments_bounded` (4×) — the care-class/compute-class ISOLATION invariant (distinct from the live `[[project_rea_compute_commitment_primitive]]`)
- `project_substrate_floor_elohim_ceiling` (3–4×) — substrate is deterministic; discernment lives in elohim agents on top
- `project_doorway_single_target_no_fanout` (3×) — no peer-iteration/blob fan-out in the gateway (lives only in doorway/CLAUDE.md today)
- `feedback_serde_json_value_breaks_zome_boundary` — `serde_json::Value` chokes `SerializedBytes`; pre-stringify with a `_json: String` field
- `project_hdi_no_get_links_in_validators` — HDI 0.7 validators can only `must_get_*`; link traversal is HDK-only
- `feedback_schema_first_ioc` — schema-first is inversion-of-control (JSON schema first, Rust+TS comply)
- `feedback_signature_changes_grep_callers` — `rg <fn>` crate-wide before commit (named #1 cause of pre-push failures 30+ min out)

Two are better served as cites to existing `history/` records than as new memory slugs:
`project_three_layer_truth_model` (→ the `dht-is-a-notary` history record) and
`feedback_sweettest_ignore_is_ci_noop` (→ the `2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md`).

Not urgent — the concepts are all explained inline in the prompt, so the dead links are lost drill-down
affordance + eroded citation-graph trust, not missing knowledge. A future memory-writing pass (or the next
ceremony picking rust-architect) should write the SHOULD-EXIST entries and repoint the two history-cite cases.
Note: `memory-coherence-audit`'s DEAD-CITE detection does not currently scan inline prose `[[slug]]`s (only
`cites:`/doc-cite envelopes) — which is why these dangled undetected; widening that scan is a separate
candidate.
