# SPRINT-KICKOFF — 2026-06-14 (Master Integration: P2P Dataplane + Federation + Vision Spine)

*The single morning-ready controller. Synthesizes the three integration docs (dataplane,
federation) + VISION-ALIGNMENT + VISION-GAP-PLANS into one dispatch surface. Working draft —
NOT cite-sealed. Subagents commit-only; the integrator pushes/merges; nothing is `kubectl`'d.*

---

## 1. ONE-SCREEN SUMMARY

**The night's output:** 16 implementation plans + 5 vision-gap scoping stubs + 2 contract ledgers
+ 3 integration docs + 2 vision docs.
- **7 dataplane plans** (P-ACTUATION, P-RECONCILE, P-DEFENSE, P-PROOFS, P-ARC, P-TRANSPORT, P-DIAGNOSTIC) — close the loop on a self-healing control plane already ~80% built.
- **4 federation plans** (F-COHERENCE, F-BOOTSTRAP, F-DEPLOY, F-EDGE) — kill the singular-doorway / A-B islanding anti-pattern over the matthew↔adam genesis pair.
- **5 vision-gap stubs** (S-CARE/O2, S-LIMIT/O3, S-AGENCY/O5, S-AI/O4, S-SPINE/O1↔O7) — the human-facing half; ZERO new DHT entry types, all `action` discriminators on existing `Mishpat::Commitment`.

**The throughline:** the plan set is **O7/O9-strong** (cybernetic discipline + capture-resistant
stasis) and **O2/O3/O4/O5-thin** (the felt, human-facing half). This sprint pulls the human-facing
spine back onto the plate: the **grandma-vertical first slice** (`feltStatus` on
`household_resilience.rs::compute` + `<elohim-memory-safety>` "Family Vault" component +
`grandma-photos-survive-node-loss.feature` scenarios 1-2) is **satisfiable TODAY** on P-PROOFS
chaos infra + the additive projection — coupling O7's resilience proof to O1/O5's felt safety while
the read-model is being lit, not after. Its gating **task-0 is the `CommitmentCommitted`
signal-decode subscriber fix** (holo_hash byte-arrays silently dropped — every human-facing
projection rides it).

**Cohesion verdict: COHESIVE-WITH-GATES.** No hard file collision across all 21 plans. All 18
must-fixes verified real; all 14 operator decisions confirmed crisp. Three cross-layer hand-offs
must be RECORDED as pre-dispatch gates (GATE-A/B/C, §2).

**The 5 first moves for the morning** (vision-leverage × readiness, deps gating):
1. **D-ACTUATION** (S13 dead-arm fix — makes the whole arc actuator stop being dead code; root, gates D-ARC)
2. **F-BOOTSTRAP** (shared MongoK2Store — the genesis-pair islanding root cause; highest *new* vision-leverage)
3. **D-RECONCILE** (SweepRegistry — dep-gated land-first for every `p2p/mod.rs` toucher)
4. **D-TRANSPORT** (ConnectionLimits — the one structural no-overwhelm hole)
5. **D-ARC** (unit work + GREENLIGHT the corpus-off-DHT spike — only floor-preserving cure for O1/O8)

…**task-0 (MF14/MF17 signal-decode) is VERIFIED DONE** — the class is CLOSED, all four subscriber families on byte-array-tolerant `HoloHashB64`.

**BUILD UNDERWAY (2026-06-14):** graduation landed (`419571e2e` — 5 cite-sealed specs + `ORACLE.md` index); **increment-1** — the canonical `trait Governor` + `Refusal`/`LimitOwner` contract in `elohim-compute::actuation` — landed, 6 tests green (`5ba4dcb02`); **increment-2** — `arc_actuator` implements `Governor` (the lift's first instance, proving it generalizes) — building. Next: `CoverageRollup` → `hello-household` felt-status.

---

## 2. PRE-DISPATCH GATES (consolidated must-fix table)

These run BEFORE or WITHIN their wave. Type legend: **plan-author** (a plan is absent), **plan-edit**
(text correction to an authored plan), **ledger-patch** (correct the ownership map), **verify-only**
(integrator confirms, no edit), **diagnose-then-fix** (systematic-debugging: instrument first, fix is a named follow-on), **integration-action** (a feat→dev merge, not new code).

| # | Fix | Owner | Wave it gates | Type |
|---|-----|-------|---------------|------|
| **MF1** | ✅ **RESOLVED** — both WAVE-1 root PLANS (P-ACTUATION, P-RECONCILE) were re-authored overnight after their first-run API-500 failures; both on disk + interface-verified by the 7/7 re-cohesion audit. The morning's Wave-1 work is *implementing* them (the Rust), which IS the dispatch — not a pre-gate. | done overnight | — | plan-author (DONE) |
| **MF2** | `p2p/mod.rs` = FOUR mutators (P-RECONCILE primary; P-TRANSPORT event-arm **+ 3 P2PConfig limit fields at mod.rs:374, NOT config.rs**; P-DIAGNOSTIC 2 bools) — all sequence behind P-RECONCILE | P-RECONCILE primary; T/D sequenced | DP Wave 2→3 | ledger-patch (G0-B) |
| **MF3** | RESOLUTION-C 3-way merge applies to **node** `config.rs` ONLY (storage limits live in mod.rs) | P-TRANSPORT | DP Wave 2 | ledger-patch |
| **MF4** | Repoint `system_metrics.rs` → `services/system_metrics.rs` (P-ARC) | P-ARC | DP Wave 2 | plan-edit (G0-D, done in SEAM-DELTA) |
| **MF5** | Repoint P2P-sim Jenkinsfile `dna/` → edge `elohim/holochain/Jenkinsfile:1375` (P-PROOFS) | P-PROOFS | DP Wave 1 | plan-edit (G0-D, done) |
| **MF6** | `chaos_dataplane.rs` `with_codec` 0.54.1 compile-check BEFORE the soak extends it | P-PROOFS Task-4 pre-step | DP Wave 1→2 | diagnose-then-fix (G0-E) |
| **MF7** | `http.rs:2764` two disjoint-line mutators (P-ACTUATION handler repoint ⨯ P-ARC `local_authored_bytes` input swap) — coexist | both; integrator verifies | DP Wave 1/2 | verify-only (RESOLUTION-F) |
| **MF8** | StaggerGate call-wiring + S13 real-bounds both RECEIVED inside P-ACTUATION | P-ACTUATION | DP Wave 1 | plan-author (G0-C) |
| **MF9** | `build_id` → RUNTIME-ENV path (`DEPLOY_VERSION` env + new `config.rs deploy_version` clap arg); rewrite F-COHERENCE Task 2 to read `state.args.deploy_version` | F-COHERENCE | **all** Fed Wave F1 | plan-edit (Fed GATE-A) |
| **MF10** | Add HARD F-COHERENCE→F-EDGE field-population edge (build_id ← F-EDGE `DEPLOY_VERSION` env, lands first) | F-EDGE env first | Fed Wave F1 ordering | ledger-patch (DAG amend) |
| **MF11** | Fix citation: `build_info.rs` is elohim-compute (dataplane), not doorway-local; delete BuildInfo ref | F-COHERENCE | Fed Wave F1 | plan-edit (Fed GATE-B) |
| **MF12** | `main.rs:1058` + `epr_router.rs` additive accessor are disjoint-region, in neither ledger map | F-COHERENCE | Fed Wave F1 | integrator note (Fed GATE-C) |
| **MF13** | **Doorway residual wedge**: warm_stream cold-start re-projection firehose × untimed Mongo awaits → all workers park; wrap the 2 Mongo awaits in tokio timeout | P-DEFENSE (T7 diagnose) + F-EDGE | DP Wave 1 | diagnose-then-fix (UNCONFIRMED / deploy-gated) |
| **MF14** | ✅ **RESOLVED** (verified 2026-06-14) — signal-decode class CLOSED: all four families (infra `d33b0e1f5`, mishpat `73b665122`, REA+content `2571fe642` = "last two dark subscribers") decode via byte-array-tolerant `HoloHashB64`; 42 inline tests; memory file records CLASS CLOSED. The only already-done "task-0". | done | — | was diagnose-then-fix |
| **GATE-A** (X-layer) | `mishpat_projection.rs`: P-ACTUATION (S13 `sets-authority-arc`) lands the structural arm; S-AGENCY/S-CARE/S-LIMIT sibling arms sequence behind it. In NEITHER ledger map | P-ACTUATION structural-first | VISION arms after DP W1 | cross-layer record |
| **GATE-B** (X-layer) | `commitments.rs` becomes a 5-way additive coordinator-action surface (P-ACTUATION `scope_vocab` + 4 vision actions); P-ACTUATION owner must be in the loop on validator-arm ordering | P-ACTUATION + vision stubs | VISION expansion | cross-layer record |
| **GATE-C** (X-layer) | ✅ **RESOLVED** — = MF14; the signal-decode class is CLOSED (verified 2026-06-14). Human-facing projections are no longer blocked on it. | done | — | cross-layer record |

**Grounding-MISSED must-fixes (surfaced from the integration docs' own seam sections):**
- **MF15** — *iteration-3 feat→dev integration* (`89bc208a8` `zome_caller.rs is_transport_error`): on `feat`, ABSENT on `dev`. INTEGRATION-ACTION (integrator's feat→dev merge), not new code; does NOT cure the render wedge. Owner: integrator, DP integration pass.
- **MF16** — *`distribute_shards` `i % len` concentration bug*: peers<shards on one household → single-loss catastrophic. P-PROOFS records it as a RED `#[ignore]` naming the fix — **highest-priority post-campaign durability `src/` fix**, owner P-RECONCILE/storage, deferred.
- **MF17** — **H0 signal-decode is in the SYNTHESIS doc, NOT in the grandma stub itself.** Verified: the stub never names signal-decode / CommitmentCommitted. RECORD it as the stub's task-0 before S-SPINE expands, or it gets dropped (= GATE-C / MF14).

---

## 3. UNIFIED DISPATCH WAVES (all three layers interleaved)

Ordered by **vision-leverage × readiness within dependency constraints**. Within a wave, picks are
vision-ranked; the technical DAG roots are NOT reordered by vision. Each plan dispatches in an
isolated worktree (subagent-driven, commit-only); the integrator merges each wave to the integration
branch so the next wave's file-sequenced sub-tasks rebase onto the updated shared files.

### WAVE 1 — ROOTS (zero inbound HARD edges; dispatch in parallel)
**Dataplane:** P-ACTUATION · P-RECONCILE (PRIMARY `p2p/mod.rs` owner — lands first) · P-DEFENSE · P-PROOFS-core (Tasks 1/2/5/6)
**Federation:** F-BOOTSTRAP (shared MongoK2Store — islanding root) · F-COHERENCE (cross-edge divergence DETECTOR — producing root) · F-EDGE/F-DEPLOY independent legs (DEPLOY_VERSION env, ALLOW_COORDINATOR_UPDATE env, gatePairCoherence guard — land FIRST inside F1)

**What this wave unblocks (the gate to Wave 2):**
- P-ACTUATION's `Actuation`/`ScopeId`/`ActuationRefusal` + the **S13 projection arm** landed & merged (the single highest-leverage fix — without it the arc actuate path is dead even though it shipped).
- P-RECONCILE's `p2p/mod.rs` structural `run()`/`P2PCommand`/snapshot rewrite landed & merged.
- P-DEFENSE's `jittered` re-exported from `elohim-compute` (clears X-COH-DEF / X-BOOT-DEF shims) AND the `immutable` Cache-Control on the storage-proxy miss path (clears the F-EDGE CDN gate X-EDGE-DEF EARLY).
- F-COHERENCE Wave-F1 merged → supplies `CoherenceView` + the `/api/v1/federation/coherence` endpoint for Federation Wave F2.

**Vision-rank note:** dependency-rank puts P-ACTUATION/P-RECONCILE first (everyone consumes them).
Vision-rank agrees on P-ACTUATION (#1 — S13 is the dead-code cure + REA canonicalization) but
ranks **D-RECONCILE as EN-class plumbing** — it leads here ONLY on deps (every `mod.rs` edit rebases
onto its SweepRegistry), not vision. **F-BOOTSTRAP is the highest *new* vision-leverage move** (O1
directly; the museum-named partition root) and competes for *authoring* attention, not build-slot
ordering.

### WAVE 2 — FIRST-ORDER CONSUMERS (parallel; non-mod.rs work may pre-start in W1 worktrees)
**Dataplane:** P-ARC (L-term wiring + node-local StaggerGate + corpus-off-DHT spike; arc-as-instance ABSORBED by P-ACTUATION) · P-TRANSPORT (connection_limits behaviour + event-arm + 3 additive P2PConfig fields, sequenced behind P-RECONCILE) · P-PROOFS-cont (Tasks 3/4)
**Federation F2:** F-EDGE (consume `CoherenceView`; `/p2p-peers` honest count) · F-DEPLOY (`deployGenesisPairAtomic` barrier; alpha-b fail-loud posture) — needs only F-COHERENCE Wave-F1, NO dataplane gate

**What this wave unblocks (gate to Wave 3):** P-TRANSPORT's additive `P2PConfig`/event-arm touches to
`p2p/mod.rs` merged → the only remaining `mod.rs` edit is P-DIAGNOSTIC's 2 bools (mechanical append).

**Vision-rank note:** within W2, **D-TRANSPORT outranks D-ARC on readiness** (unit-shaped, no operator
gate; closes the one structural no-overwhelm hole — O7/O8/O9). D-ARC's *unit* portion is W2-ready; its
*cure* portion (corpus-off-DHT) waits on Decision D1/D1b.

### WAVE 3 — TERMINAL CONSUMERS (file-sequenced behind all prior `p2p/mod.rs` mutators)
**Dataplane:** P-DIAGNOSTIC (`self_cid_present`/`provide_loop_enabled` + populate; `anchor` block on doorway `self_healing.rs`; schema bumps + TS regen + schema_contract). Non-mod.rs Tasks 2-5/7 may run in W2; only Task 1 + Task 6 wait.
**Federation F3 (cross-layer, behind dataplane W3):** F-COHERENCE Task 5 (`coherence` block into `SelfHealingView` via the `self_healing.rs` `// FOLLOW-ON` seam — GATE: P-DIAGNOSTIC `anchor` landed) · F-COHERENCE Task 6 (`self_cid_present` enrich, SOFT) · F-DEPLOY head-leg flip to `exit 1` (X-DEPLOY-DIAG) · F-EDGE `/blob/*` CDN enable (X-EDGE-DEF — already cleared by P-DEFENSE in W1, so it can land as early as Federation F2).

### THE GRANDMA-VERTICAL SPINE SLICE (H1) — earliest wave it is satisfiable = NOW
**Couple it DURING Wave 3 authoring** (Decision D13 / Vision Decision F), as a plan-edit to
P-DIAGNOSTIC + P-PROOFS plus one new `.feature`. It is satisfiable TODAY on P-PROOFS chaos infra +
the additive projection.
- **task-0 (H0/MF14/GATE-C):** fix the `CommitmentCommitted` signal-decode subscriber FIRST.
- **piece 1:** `feltStatus` Cat-C sub-block on `household_resilience.rs::compute` (consumes D-DIAGNOSTIC read-model + D-PROOFS placement) — turns `at-risk` into "Aunt Ruth's copy went offline — 2 households still hold these photos."
- **piece 2:** `<elohim-memory-safety>` "Family Vault" felt component (renders names, not nines; eyes-first via `pnpm look`).
- **piece 3:** `grandma-photos-survive-node-loss.feature` scenarios 1-2 (satisfiable now); scenarios 3-4 land as `@wip` attach-points for S-AGENCY / S-CARE.

**THE EXPLICIT VISION↔DEPENDENCY DIVERGENCE:** dependency-rank puts the spine in Wave 3 (it *reads*
D-DIAGNOSTIC's read-model + D-PROOFS' chaos infra, both of which must land). Vision-rank weights it as
the **highest-leverage move in the whole set** (O1+O7-felt — the only test O9 actually sets: stasis the
grandmother can feel). The honest resolution: the spine *couples during* D-DIAGNOSTIC authoring as a
plan-edit, so the read-model lights for **household-eyes, not admin-eyes** — it does NOT precede the
technical landing, and it does NOT reorder the technical DAG roots. Vision-leverage orders only the
*human-facing* picks within their gate: **H1 → H2 → H3 → H4 → H5** (spine → data-agency revoke →
self-limit governor → native care emitter → AI covenant last).

### Human-facing sequence (pinned to the technical plan each consumes — expand only on operator blessing)
| Order | Move | Must land first | Gate |
|---|---|---|---|
| **H0** | signal-decode subscriber fix | — (bug fix) | task-0 of whichever stub expands first |
| **H1** | S-SPINE first slice | D-DIAGNOSTIC (W3) + D-PROOFS a,b (W2) | couple *while* D-DIAGNOSTIC is authored |
| **H2** | S-AGENCY MVP (`withdraws-provide`) | D-ACTUATION (W1) `Actuation`/`RefusalCode` + S13 arm | speaks the canonical refusal contract |
| **H3** | S-LIMIT first slice (`respects-self-limit`) | D-ACTUATION (W1) + D-RECONCILE (W1) cadence | governor is a near-clone of the actuation spine |
| **H4** | S-CARE (Observation→EconomicEvent emitter) | H0 + Observation read side (CONSUME) | only stub needing a new storage *service* |
| **H5** | S-AI (agent covenant, Framing A only) | H2 + D-ACTUATION refusal legibility | most values-gated; no pre-built standing economy |

---

## 4. THE INTEGRATION PASS (runs AFTER the parallel plans land)

Run on the integration branch after all waves merge. Closes every `// FOLLOW-ON` seam, verifies the
shared contracts MATCH (one owner, identical signature), and runs the full proof suite as the
acceptance gate.

**4.1 Resolve the `// FOLLOW-ON` seams (DO-NOW vs DEFER-with-named-owner):**
- DO-NOW: arc-as-instance refactor landed in `services/actuation/arc.rs` (RESOLUTION-A); StaggerGate call-wiring received inside P-ACTUATION (G0-C, must not be dead code); live actuate-path end-to-end (commitment→projection→derive→gate→restart, now that S13 is real); `http.rs:2764` two-line coexistence verified; iteration-3 feat→dev merge (MF15); `pooled_client_config()` extraction if T6 needed it; `render_queue_full_total()` counter surfaces in render-stats.
- DEFER (named owner): cross-mesh StaggerCoordinator (P-RECONCILE v2); fractional arc (REJECTED, gauge-only); corpus-off-DHT impl (operator-gated, leak-discriminator-gated); `distribute_shards` diversity fix (MF16, RED `#[ignore]`, highest post-campaign durability item); render-channel fix (pending the confirming load trace — systematic-debugging, no blind fix); blob-plane cutover (L-sized, owner+date blank); Angular stability-lens anchor render (eyes-first frontend sibling); runtime-tunable connection limits.
- DECIDE-in-pass: `provide_loop_enabled` semantics → precondition-only (D3); AtomicBool liveness is the named follow-on.

**4.2 Verify shared contracts MATCH (grep-verified single owner):** S1 `ActuationRefusal`/`RefusalCode`
ONLY in `elohim_compute::actuation` (no reconcile/arc shim survives); S2 `trait Actuation` matches the
ledger §3 canonical row; S3 `ScopeId` one definition, literal `"conductor.target_arc_factor"` GONE from
zome + actuator; S5 sweep types one definition (P-DIAGNOSTIC did NOT redefine); S7 `jittered` one
definition, both consumers import it; S9 `P2PStatusInfo` four additive contributions in order (every
literal incl. `for_testing` carries all new fields); S13 projection arm returns real bounds for `active`;
S14 `connection_limits` on both swarm behaviours + feature flipped in both `Cargo.toml`; `elohim-compute/src/lib.rs`
three append-only re-export blocks; node `config.rs` 3-way additive merge (storage limits NOT here).

**4.3 The full proof suite = the ACCEPTANCE GATE** (per-crate RUSTFLAGS discipline load-bearing:
`--cfg getrandom_backend="custom"` ONLY for elohim-storage; `""` for compute/doorway/node/render;
`RUSTC_WRAPPER=""`; `/tmp` target dirs; plain `cargo test`, no nextest; `2>&1 | tail -N`, never `&&`-pipe):
`rs_reconstruct_property` · `placement_diversity_invariant` · `arc_coverage_multinode` ·
`no_overwhelm_soak --ignored` (a HANG = the floor-hole documented, not a CI red; must complete bounded
once `connection_limits` is present) · `schema_contract` · `--lib`/`clippy -D warnings`/`fmt --check`
across every touched crate · `bash -n simulate.sh` · doorway `verify-pair-coherence.sh --selftest`
(3 cases) · both doorway manifests parse as YAML.

**4.4 Live acceptance (operator-run, post-deploy — NOT tonight, NOT from dev):**
- **A/B-coherence live check:** `curl` `/admin/bootstrap-coherence`, `/api/v1/federation/p2p-peers`, and `/api/v1/federation/coherence` on BOTH `doorway-alpha.elohim.host` and `elohim.host` → equal `digest` + equal `buildId` = coherent; divergent digest = content skew; divergent buildId only = the operator's actual `e0352a7`/`8a2c65e` deploy-version-skew symptom, now NAMED distinctly. The next pair deploy runs `deployGenesisPairAtomic` → a divergent pair is FAILURE (not UNSTABLE).
- **Grandma a2o scene:** `grandma-photos-survive-node-loss.feature` scenarios 1-2 pass on the chaos infra; `pnpm look` confirms the `<elohim-memory-safety>` Family Vault surface renders names.

---

## 5. OPERATOR DECISIONS (the ratify-over-coffee list — 14 + 2 surfaced)

| ID | Decision | RECOMMENDATION (defended) | What it gates | Greenlight? |
|----|----------|---------------------------|---------------|:-----------:|
| **D1** | ARC topology | **Accept (i){0,1}+structure-bound + (iii) corpus-off-DHT; REJECT (ii) fractional** — (ii) spike-verified infeasible on kitsune2 0.3.2/0.4.1 (clamps {0,1}); (iii) is the only RAM∝C/N lever keeping lean devices full participants. GREENLIGHT the (iii) spike. | P-ARC Tasks 2-5 (DP W2) | ☐ |
| **D1b** | (iii) sub-gate: leak-vs-bounded | **Run the RSS-split discriminator FIRST** (conductor-child vs storage-parent, or `arc_factor:0` ablation) — sizes (iii)'s payoff before spending blast radius | P-ARC Task 5 only | ☐ |
| **D2** | iroh endgame | **Dated blob-first cutover, NOT freeze** — complementarity spec makes dual-stack permanent for 7/9 planes, so freeze can't stop the parity tax; only blob-canonical cutover removes a transport. Operator fills owner+date | nothing in-campaign; L follow-on | ☐ |
| **D3** | provide_loop_enabled semantics | **Precondition-only** — storage-visible necessary condition; minimal blast radius; doc-comment states the weaker meaning. AtomicBool liveness = named follow-on | P-DIAGNOSTIC integration pass | ☐ |
| **D4** | rate_limit_rpm | **DELETE** — zero enforcing consumers (config-theater); if ever wanted, it's an `Actuation` instance, never a bespoke counter | P-DEFENSE W1 | ☐ |
| **D5** | Coherence-locus | **Substrate (F-BOOTSTRAP+F-DEPLOY) is the real fix; edge is detect-only** — doorway surfaces divergence + alarms, never authors cross-edge truth. Either way the DETECTOR (F-COHERENCE) ships first | F-COHERENCE; F-DEPLOY posture | ☐ |
| **D6** | Real LB + apex failover | **NO auto-failover** — A=matthew/B=adam pinning is load-bearing; auto-failover would let apex silently serve a DIVERGENT matthew head (worse than a clean 503). Detection from F-COHERENCE | F-DEPLOY Task 5; F-EDGE open-decision | ☐ |
| **D7** | Atomic A+B deploy gate | **FAILURE** (not UNSTABLE) on pair divergence; single-human flap stays UNSTABLE. Enforce genesis-pair DNA-flag coherence via `gatePairCoherence` | F-DEPLOY Task 4 | ☐ |
| **D8** | CDN for immutable-CID | **YES, `/blob/*` ONLY** (CID=cache key); never EPR-head/view routes. Prereq: P-DEFENSE immutable Cache-Control on miss path. Ships commented DRAFT | F-EDGE Wave F3 enable | ☐ |
| **D9** | S-CARE primitive | **Instantiation of EconomicEvent/Commitment family (0 new entries)** — `commits-care` action; substrate already holds the EconomicEvent entry + emit spine | S-CARE expansion | ☐ |
| **D10** | S-LIMIT notarize | **A — notarize the self-limit** — citable in refusals + gossiped accountability; the line a person draws should be answerable, not private-only | S-LIMIT expansion | ☐ |
| **D11** | S-AGENCY withdrawal | **Succeeds-with-residual-report** — a refusal re-imports the operator-veto smell O5 exists to kill; honest residual-held read is separate | S-AGENCY expansion | ☐ |
| **D12** | S-AI standing | **Parallel-but-subordinate** — bound power under a revocable covenant, not peer standing (`confession.md:93`); ship Framing A only, no standing economy | S-AI expansion | ☐ |
| **D13** | S-SPINE couple now | **Couple into THIS sprint** — light the felt surface (feltStatus + `<elohim-memory-safety>` + scenarios 1-2) WHILE D-DIAGNOSTIC lands, for household-eyes. Satisfiable today on P-PROOFS chaos infra + additive projection | S-SPINE first slice | ☐ |
| **D14** | Shared refusal vocab (cross-cutting) | **Settle ONCE** — `limit_owner: self\|commitment\|operator` + `RefusalCode::SelfLimitConflict` in P-ACTUATION's enum; S-LIMIT/S-AGENCY/S-SPINE all inherit. Bless it in the S-LIMIT decision | all three vision stubs | ☐ |
| **D15** (surfaced) | `gatePairCoherence` reads adam's LITERAL flag? | **DEFER** — v1 asserts the *intended* formula agrees; a true guard parses adam's manifest env literal. Not dispatch-blocking | F-DEPLOY hardening | ☐ |
| **D16** (surfaced) | Apply edge Jenkinsfile + manifests | **Operator applies post-dev-merge; subagents commit-only, never kubectl** — the repo is the cleanup surface. Nothing applies tonight | the whole federation delivery | ☐ |

---

## 6. THE VISION LEDGER (O1-O9 × plan coverage)

Legend: ● direct · ◐ indirect/enabling · ○ none.

| Obj | What it means | Coverage this sprint | Plans |
|-----|---------------|:---------------------|-------|
| **O1** | grandma-friendly P2P substrate, hub-optional floor | ● (floor + felt spine pulled onto plate) | D-ARC, F-COHERENCE, F-BOOTSTRAP, **S-SPINE** |
| **O2** | trust-economy of intimate, observed care | ◐ → ● *if S-CARE blessed* | S-CARE (stub), enabled by D-ACTUATION REA spine |
| **O3** | respect-own-limits, pro-social feedback loops | ◐ → ● *if S-LIMIT blessed* | S-LIMIT (stub), D-DIAGNOSTIC |
| **O4** | a home for AI | ○ → **scoping-only** (honestly flagged) | S-AI (stub; Framing A only, deliberately deferred — zero momentum, named so it doesn't drift to permanent zero) |
| **O5** | agency back to one's data | ◐ → ● *if S-AGENCY blessed* | S-AGENCY (stub), D-DIAGNOSTIC read-model |
| **O6** | responsibility for creation of mutual value | ● | D-ACTUATION (REA `delegates-compute` spine) |
| **O7** | systems thinking + cybernetic discipline | ●●● (the set's center of gravity) | D-ACTUATION, D-RECONCILE, D-TRANSPORT, D-PROOFS, D-DEFENSE, D-DIAGNOSTIC, F-COHERENCE |
| **O8** | broadly shared capability (anti-capture) | ● | D-ARC, D-TRANSPORT, F-COHERENCE, F-BOOTSTRAP |
| **O9** | attractor / stasis / capture-resistance | ●●● | D-ARC, D-TRANSPORT, D-PROOFS, D-DEFENSE, F-COHERENCE |

**Reading:** the sprint is **O6/O7/O8/O9-strong** (the cybernetic floor) and **O1-felt newly served**
via the grandma spine. **O2/O3/O5 are scoping-blessed-and-ready** (zero new DHT entries, all blocked
only on operator decisions D9/D10/D11/D14). **O4 (home for AI) is honestly flagged as scoping-only** —
no implementation this sprint; named so it does not drift to permanent zero. The single move that puts
the sprint's stasis where the grandmother can feel it is **D13 (couple S-SPINE now)** — the only test
O9 actually sets.
