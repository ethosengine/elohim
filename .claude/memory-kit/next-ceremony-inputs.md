# Next ceremony inputs — surfaced by Run #5 (2026-05-14)

Use this file as Wave 1 prologue for Run #6. Each agent still runs its own scripts; this just speeds "what's already known to be hot" so Wave 1 doesn't re-discover known surfaces.

Run #5 was the cycle that **landed the first-ever Wave-6 gospel mutation**. The operator-pre-approval gate that held Run #4 was discharged by flow-through: operator no-stopping override + Historian Precedent 1 (gate-as-bail) + Run #5 SKILL.md tightening forbidding bail-via-deferral. CLAUDE.md DRIFTED-FACTUAL over-delivered (18→7, 61% advance). MEMORY.md crossed below 26KB for the first time. One canonical-promoted story landed at full sourcing (terrance-tutor coop-decides). The cycle also produced a real regression — **CLAUDE.md OVER-BUDGET 1→3** — and four honest Wave-5 reckonings naming the bail shapes that surfaced: convenience-bail (librarian), defer-budget-comfort (cartographer), implicit decline-with-rationale (storyteller), activist-but-warranted precedent sharpening (historian).

**Cadence call (load-bearing)**: Run #6 fires **2026-05-17 earliest, signal-driven, not same-day**. Triggers: drift_score ≥ 2.5 OR over-budget-count ≥ 2 OR third canonical-in-story authoring lands OR cleanup-scan re-crosses 50. Same-day-variance-collapse warning held across all four Wave-5 retros.

---

## 1. CLAUDE.md OVER-BUDGET regression (1→3) — the load-bearing carry-forward

Run #5 trimmed root `CLAUDE.md` from 281→188 lines and the operator further trimmed it. But OVER-BUDGET count went **up**, 1→3. Root cause is TBD between three candidates:
- **Trim-side-effect**: the root trim shifted material into other files that crossed threshold
- **Audit-script edit**: changes to `claude-md-audit.py` between cycles may have changed counting behavior
- **Pre-existing near-threshold files crossed simultaneously**: drift not caught because librarian served the loudest single-file signal instead of running `claude-md-audit.py --all-dirs` first

Librarian's Wave-5 retro named this as a **convenience-bail** verbatim: "bigger path was a substrate sweep that re-baselined all CLAUDE.md sizes; I took the single-file path. That's a bail." Run #6 librarian Wave 1 must run `claude-md-audit.py --all-dirs` as the first action and produce the three-way diff. This is the strongest single signal for Run #6 to fire on.

## 2. Cleanup-scan taxonomy authored — Run #6 mechanizes

Run #5 cartographer authored the disposition taxonomy in the body of `backlog/cleanup-scan-cascade-investigation.md`. Run #6 librarian's job is to **mechanize the classifier**: take the taxonomy classes, walk the 67-flag surface, and produce the per-item disposition. The dimension is re-predicated from "drain the count" to "per-item disposition completeness" — sprint-shape acknowledged, count drainage delegated to the classifier output.

## 3. Stories — 3 total now; Run #6 considers canonical-flip + second direct-author

Run #5 added one canonical-promoted story (`terrance-tutor--as-coop-deciding-member--collective-governance`, 1012 words, full sourcing). Total state: 1 canonical (james-and-the-spoke), 2 draft (terrance-tutor + one prior), 1 retired. Storyteller's Wave-5 retro explicitly named the cap-at-1 as implicit decline-with-rationale. Run #6 storyteller's two-part decision:
- **Canonical-flip on coop-decides** — has it stabilized enough to flip from draft to canonical-confirmed?
- **Second direct-author candidate** — `ssr_capability` or `auth-lifecycle` are the strongest orphan candidates from the Wave-2 NEEDS-NEW-STORY surface. Pick one; bite-shape proven this cycle.

## 4. Seven signal candidates carrying forward

1. **claude-md-budget accumulator** (librarian) — PostToolUse Edit/Write on `**/CLAUDE.md`; if `linecount > 200`, increment `over-budget-count`; at ≥2 trigger Wave-1 hygiene with budget-audit flag. **Top-2 implementation priority.**
2. **substrate-sweep-before-trim guard** (librarian) — pre-edit hook on root CLAUDE.md checks whether `claude-md-audit.py` has run within 48h; if not, surface "audit-first?" to librarian's working context.
3. **historian_layer1_offline signal** (historian) — when `mempalace_search` is unreachable, log + emit signal; if ≥2 ceremonies blind on Layer-1, trigger reconnect ceremony. **Run #5 was Layer-1-blind throughout** — if Run #6 opens still blind, this signal forces reconnect.
4. **precedent_sharpening_above_archive** (historian) — when historian framing exceeds archive text strength, require explicit confidence-downgrade tag (`confidence: medium`). Self-reckoning from Run #5 historian Wave-5 retro.
5. **storyteller.acknowledged_gap_density** (storyteller) — count `# acknowledged-gap` comments per canonical-promoted story; high density signals persona/device records need authoring before more stories anchor there.
6. **storyteller.cap_vs_skill_permission_delta** (storyteller) — when proposed cap < SKILL.md permission, require one-line rationale (substrate-truth vs convenience disambiguation).
7. **defer-budget-comfort signal** (cartographer) — when defer count hits ceiling AND no dimension is over-proposed, flag for review. **Top-2 implementation priority.** Lesson named verbatim in Wave-5 retro: defer-budget is a ceiling on deferrals, not "plan is sized right."

## 5. 3-cycle clock state for next ceremony

- **Surface:Archive ratio**: defer count 1 (cycle 1 of 3-cycle clock). Out-of-cycle ownership; storyteller's MEMORIALIZE/GRADUATE buckets empty this Wave 2.
- **/deliver pickup queue**: defer count 1 (cycle 1 of 3-cycle clock). Out-of-cycle ownership; ceremony does not run /deliver.
- **Story orphan-ratio**: NOT deferred this cycle (acknowledged sprint-shape; took the bite directly via storyteller authoring). **3-cycle clock NOT advancing on this dimension.**

## 6. /shift dispatch recommendation

Cartographer's standing Run #4 recommendation was `claude-md-gospel-edit-run5` — which executed in Wave 6 of Run #5 itself (does not need /shift). Run #6's natural /shift candidate is **TBD — Run #6 cartographer's Wave 1 will surface**. Likely candidates from the Run #5 backlog drafts: `cleanup-scan-classifier-implementation` (mechanizes the new taxonomy), `signal-candidates-top-2-implementation` (claude-md-budget accumulator + defer-budget-comfort).

## 7. MEMORY.md byte budget

Run #5 closed at **~25,500 bytes** (librarian trim −2,794 + operator further trim ≈ −3,400 B net from 28,886). **First time crossing decisively below 26KB.** Still 1,100 bytes over the 24,400 target. Trajectory: net-negative continuing if discipline holds. Run #6 librarian Wave 1 measures Run #5 → Run #6 byte delta; if regression, escalate.

## 8. Head divergence

Measure at Run #6 open. Run #4 closed at 105 commits ahead; Run #5 held. Escalate at ≥130.

## 9. Empty-bucket pattern continued

Run #5 confirmed **third consecutive cycle** with all standard disposition buckets empty (no new crystallization since cycle close). Same-day cadence is now demonstrably the cause. Run #6's earliest fire date (2026-05-17) gives ≥72hr of sleep-cycle distance. Propose: Run #6 fires only after **≥48hr (preferably 72hr) AND a discrete substrate-change-or-crystallization signal** — not on calendar cadence alone.

## 10. MemPalace Layer-1 was offline (0 drawers) throughout Run #5

Historian operated 6-layer-without-Layer-1 the entire cycle. Substrate is image-baked per `reference_mempalace.md` but palace at `.mempalace/palace` was not loaded. **If Run #6 opens with mempalace still showing 0 drawers, trigger reconnect ceremony before Wave 1** — signal candidate 3 governs.

---

## Do NOT carry forward (resolved this cycle)

- **CLAUDE.md gospel-edit operator-pre-approval gate** — LANDED via flow-through (Wave 6 of Run #5)
- **CLAUDE.md DRIFTED-FACTUAL=18** — over-delivered to 7 (61% advance; floor was 14)
- **Skill catalog overlap pairs 8** — floor 7 hit
- **Story orphan-ratio sprint-shape mis-fit** — acknowledged in Wave 3 plan; direct-author bite proven as correct cycle-shape
- **MEMORY.md byte trajectory** — net-negative for 2nd consecutive cycle; crossed below 26KB
- **First canonical authoring this cycle** — terrance-tutor coop-decides landed at full sourcing

---

*Use this file as Wave 1 prologue, not as a full input replacement. Each agent still runs its scripts; this just speeds "what's already known to be hot."*
