---
epr-habit-version: 1
id: attention-witnessed-privately
invariant: >
  A person's attention on content is witnessed as an agent-private observation on their own node,
  under a manifest-declared kind. It is never written as a lens preset, never gossiped, and never
  joined by an outside observer. Every view over it is a recipe whose CID prints on the page.
status: red
active: false
checks:
  - "a2o @concern:attention-witnessed-privately (genesis/a2o/features/lms/attention-witnessed-privately.feature — Jessica's dwell and scroll depth appear in her own /me/stream with the recipe CID printed, James's stream has no such entry, and her storage peer counts the cursor as suppressed, never announced; runnable via `just test mesh-browser '@concern:attention-witnessed-privately'`; MEASURED 2026-09-24: absent — the feature file does not exist)"
  - "cargo test --lib -- observation::gossip_gate (elohim/elohim-storage — an agent-private kind yields no cursor announcement on the real write path; MEASURED 2026-09-24: absent — no `gossip_gate` anywhere in elohim/elohim-storage/src; not run, since there is nothing to run)"
  - "test $(grep -c \"trackContentLeave\\|trackContentView\" app/lamad/src/app/components/content-viewer/content-viewer.component.ts) -eq 0 (the content viewer no longer writes dwell as an AttentionTending lens record; MEASURED 2026-09-24: 3 — the viewer still tends on leave and on view)"
refs:
  - "plan: genesis/docs/superpowers/plans/2026-09-25-post-station-4-participant-classifier-search-attention-sprint.md (Lane A, rulings R-A0..R-A5, tasks A0..A10)"
  - "spec: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md (the observation layer this habit makes live; §4 gate answers, §8.2 summary graduation)"
  - "manifest kind: elohim/sdk/domains/lamad/manifest/observation-kinds.json (`lamad:content-viewed` with `dwell_ms` and `scroll_depth_pct`)"
  - "routes today: elohim/elohim-storage/src/http.rs (the observation-sessions arm and `build_manifest()`), elohim/elohim-storage/src/api/observations.rs (GET routes that never serve)"
retire-when: >
  when the attention log is a signed, persisted, agent-private-encrypted iroh log with the
  graduation evaluator — privacy is then a property of the log itself, not a practice under watch.
---
DELTA 2026-09-24 (BORN red, `active: false`; declared by task A0 of the post-station-4 plan).
All three checks measured: the feature file is absent, the gossip-gate test is absent, and the
content viewer still makes 3 tending calls. GROUNDING: the observation routes are shadowed on the
live server. `elohim/elohim-storage/src/http.rs:2531` matches every path starting
`/api/v1/observations` and sends it to the observation-sessions handler (`/begin`,
`/{id}/entries`, `/{id}/report`) before the `/api/v1/` catch-all can reach
`api/observations.rs`, so the existing GET routes have never served. The tending route
`/api/v1/attention/tending` is undeclared at the doorway: `build_manifest()` declares only the
three session routes for observations and nothing for attention, so a browser-path tending write
has no route through the doorway (Tauri-direct only).
