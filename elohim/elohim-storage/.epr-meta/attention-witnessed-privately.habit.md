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

DELTA 2026-09-24 (A1–A9 landed; status stays red, mesh run pending). The landed tasks are A0
`a29e9c64f`, A1 `033cbf0c8`, A2 `f46f3517c`, A3 `79996e877`, A4 `1c3adb869`, A5 `5a7ae8599`,
A6 `06444bfca`, A7 `064f720d3`, A8 `a2b1b3b9d`, A9 feature+steps `bdb6b1ac3` and backlog atoms
`6bae0fd12`. Check (3) was re-measured green: the grep on content-viewer.component.ts = 0.
Check (2) was re-measured green: `cargo test --lib -- gossip_gate` (CARGO_BUILD_JOBS=1, pool
target) ran `agent_private_kind_yields_no_announcement` ok and
`non_private_kind_yields_announcement` ok, 2 passed, EXIT=0. Check (1) is still red: the
feature exists, and its cucumber dry-run defines all 14 steps, but it has not yet run on the
household mesh (`just mesh start && just mesh prologue`, then `just test mesh-browser
'@concern:attention-witnessed-privately'`), and the habit stays red until that is green twice.
Found while writing the scenario: the view posts only on an in-app leave (backlog
`content-view-observation-lost-on-hard-leave`).

DELTA 2026-09-24 (independent review H1 closed; status stays red, mesh run still pending). The
review found that un-shadowing the observation routes (A2, `f46f3517c`) brought three
cross-observer GETs to life that break this habit's "never joined by an outside observer" clause:
`by-observer?observerCid=<anyone>` served another person's agent-private rows with NO header at
all, `by-subject` listed every observer of a subject, and `diversity` counted across observers.
Closed under ruling R-A9 with negative tests first (red run recorded: 4 of the 5 new privacy
tests failed on the old handlers): `by-observer` now takes the explicit `X-Agent-Cid` and serves
only that observer (401 without it, 403 on mismatch, no `local_sessions` fallback), and
`by-subject` / `diversity` refuse any kind the registry marks `reach: agent-private` — 404 with
the reason whether or not rows exist, so the route is not an existence oracle, and a node with no
registry refuses every cross-observer read rather than guess. `cargo test --test api --
observations`: 35 passed, 0 failed, EXIT=0. Same wave (rulings R-A10/R-A11): `observedAt` bounded
to `0..=now+300`, the POST body capped at 16 KiB, `subjectCid` required to equal the payload's
`ref_cid`, the lifestream's title lookup filtered to commons/public reach, the stream window
pushed into SQL with the out-of-window rows counted there, and the in-memory log reduced to its
hasher, offset and a bounded tail. Check (1) is unchanged: the household mesh run is still owed.
