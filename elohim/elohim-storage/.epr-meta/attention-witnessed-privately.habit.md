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
  - "a2o @concern:attention-witnessed-privately (genesis/a2o/features/lms/attention-witnessed-privately.feature — Jessica's dwell and scroll depth appear in her own /me/stream with the recipe CID printed, James's stream has no such entry, and her storage peer counts the cursor as suppressed, never announced; runnable via `just test mesh-browser '@concern:attention-witnessed-privately'`; MEASURED 2026-09-25: RED twice at the write-ack step — the a2o harness's injected session does not survive the shell's restoreSession, so the bearer-carrying emitter has no one to write as (receipts sprint-report-household-20260925T131704Z / T131924Z))"
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

DELTA 2026-09-25 (first household mesh run of check (1); status STAYS red — the scenario is RED on a
named doorway gap, not on the scenario). Receipt
`genesis/a2o/reports/sprint-report-household-20260925T014102Z-5c2b52f1.{json,md}`
(`just test mesh-browser '@concern:attention-witnessed-privately'`, household-dowell, transport dual,
3 peers, processControl true, sut sha256:1c42aeb8f1a6c599): 1 scenario, 14 steps — 8 passed, 1 FAILED,
5 skipped. The failing step is `And Jessica waits until her storage peer replies to her app that it
has kept her note on the "manifesto" page` (steps/lamad/attention-witnessed.steps.ts:352):
`POST /api/v1/observations answered 500: Error: Authentication error: POST /api/v1/observations
requires the X-Agent-Cid header: an observation is written only by the person it belongs to`.
CAUSE, read at the doorway: the browser path has no identity injection for this route —
`doorway/doorway-service/src` mentions only `/api/v1/observations/{}/entries`
(server/http.rs:9354, the session route) and carries NO route entry for `POST /api/v1/observations`,
so the doorway forwards the write without resolving the bearer into `X-Agent-Cid` (the injection
apparatus exists and is tested for the routes that declare it — routes/identity.rs:482-621) and
storage correctly refuses a write whose observer it cannot attribute. This is the same seam this
habit's BORN delta named for `/api/v1/attention/tending`: the A-lane landed the storage handler
(R-A6) and the emitter (R-A8/R-A11), and the doorway declaration for the browser write path did not
land with them. Everything before the ack passed: Jessica signed in, opened the manifesto in the
shell viewer, the page grew taller than the window, she dwelled >3 s, the WINDOW scrolled (scrollY>0,
so the viewer's own listener could witness depth), and the in-app leave routed to `/`. So the
observation is emitted and refused at the doorway, not lost in the page.
BYPASS, stated so the receipt is honest: the run set `MESH_ALLOW_NO_PROLOGUE=1`, which skips the
lane's shared lamad zome-call readiness rail. That rail was refusing on a peer this scenario does not
measure — `jessica:8091` answers `/db/content/elohim-host-landing/head-record` with 404
`head-record-empty` (she holds no head she authored; the prologue's `propagate-landing-to-B` legs are
DECLARE_ONLY and point at matthew's action, which her cell cannot retrieve) and later times out past
30 s under sustained SQLite lock contention (`apply_delta: database is locked`,
`Update anchor failed: database is locked`). The doorway this scenario measures is alpha, backed by
matthew, which served the same head-record in 1.6 s; the cast was seeded green twice by
`just mesh prologue` this session. The readiness rail is a launch precondition against opaque 401s,
not part of the assertion — the 16 fixture sign-ins all succeeded.
Checks (2) and (3) are unchanged and green, as re-measured in the 2026-09-24 A1-A9 delta and quoted
here verbatim: (2) "`cargo test --lib -- gossip_gate` (CARGO_BUILD_JOBS=1, pool target) ran
`agent_private_kind_yields_no_announcement` ok and `non_private_kind_yields_announcement` ok,
2 passed, EXIT=0"; (3) "the grep on content-viewer.component.ts = 0". Check (1) stays red until the
doorway declares the observation write route and the scenario is green twice.
Substrate conditions recorded while measuring, each a separate concern from this habit: the
household's conductors carry a MUTUAL per-DNA BlockSpan on `infrastructure`
(`uhC0kYVFIpz1CIpaXG_YotfH3n4l-DALhSa2EGUB7He_uBWTvoNga`; matthew 85 incoming blocked from one peer,
jessica 38 incoming / 18 outgoing) while `lamad` is unblocked; jessica additionally held a
`CellDisabled` role that did not self-heal across ~90 min after the conductor roll; and the cure for
`propagate-landing-to-B` after a roll is the CARRIED RECORD — a hash-only declare is refused
`declare_canonical_head: target action … is not retrievable`, while the same declare with the
5120-b64-char record fetched from doorway A's `/head-record` answered
`✓ canonical head propagated` on attempt 1.

DELTA 2026-09-25 (household re-proof after the bearer fixes; status STAYS red — check (1) is red twice
on a named a2o-HARNESS cause, not on the fixes and not on the scenario's text). Two runs of
`just test mesh-browser '@concern:attention-witnessed-privately'` on household-dowell (transport dual,
3 peers, processControl true, commit a2846c534, sut sha256:0a86588763f52241 / sha256:ae9cd29b07d22ec3;
fix commits bab140f95 f45fb527e 9f7529be5 4a661191e under measure; the lamad and shell development
dists rebuilt at f45fb527e's source, checked for `stream-signed-out`, `stream-provenance` and a
`keepalive` fetch carrying `Authorization`, re-staged via doorway A and propagated to B with the carried
record). Receipts `genesis/a2o/reports/sprint-report-household-20260925T131704Z-a2846c53.{json,md}` and
`…T131924Z-a2846c53.{json,md}`, identical: 1 scenario, 14 steps — 8 passed, 1 FAILED, 5 skipped. Failing
step: `And Jessica waits until her storage peer replies to her app that it has kept her note on the
"manifesto" page` (steps/lamad/attention-witnessed.steps.ts:352): `no POST /api/v1/observations was
answered within 15 s of Jessica leaving "manifesto" — the view was never witnessed`. The write no longer
reaches storage anonymous (the old 500 is gone); it is not sent at all. CAUSE, proven with a read-only
browser probe through the harness's own `PlaywrightDevice.login`: `injectAuth`
(genesis/a2o/src/framework/devices/playwright-device.ts) writes only `elohim-auth-token`,
`elohim-auth-agent-pub-key` and `elohim-auth-human-id`, while the shell's `AuthService.restoreSession()`
needs `elohim-auth-provider` and a live `elohim-auth-expiry` — a missing expiry reads as expired, so the
token store CLEARS the session at boot (every `elohim-auth-*` key present after injection, none after
`/resource/manifesto` loads). `AuthService.token()` is therefore null, and the R-A12 emitter correctly
posts nothing for a reader it cannot name. The same gap made the first proof's write anonymous (so its
500 was two causes stacked; R-A12's fix was right and necessary). The cure is the harness: write the
provider and the expiry the sign-in returns, as a real sign-in does; no product source changes. NO
BYPASS: the readiness rail passed on all three peers. Checks (2) and (3) are unchanged and green, quoted
verbatim from the 2026-09-24 A1-A9 delta: (2) "`cargo test --lib -- gossip_gate` (CARGO_BUILD_JOBS=1,
pool target) ran `agent_private_kind_yields_no_announcement` ok and `non_private_kind_yields_announcement`
ok, 2 passed, EXIT=0"; (3) "the grep on content-viewer.component.ts = 0". Environment: the household
recast on conductor fork e0bfc6c7a (the pin target, 81e77bcae) by session 9adf9f01 at 12:22Z, every
role `zomePath: live`, preflight ok on every line.
