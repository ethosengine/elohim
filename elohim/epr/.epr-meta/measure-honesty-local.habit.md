---
epr-habit-version: 1
id: measure-honesty-local
invariant: >
  Every quantity this repo tracks declares its kind (level/rate/ratio), a
  rate cannot forget its period, confidence rides inside the canonical
  dag-cbor bytes (never a detachable sibling), a fold over uncertain
  quantities returns an interval rather than a bare scalar, and the
  governance gate refuses a measure declaration with no kind — proven both
  on the type (elohim/epr) and on this repo's own doc corpus
  (_lib/doc_dynamics.py), not merely asserted in prose.
status: green
active: false
checks:
  - "cargo test (elohim/epr) — measure_ontology.rs (L1/L3/L4) + canonical_bytes.rs confidence_is_inside_the_canonical_bytes (L2, asserted at the CID level via compute_cid, not byte-inequality)"
  - "cargo test (elohim/epr-rea) — uncertainty_closure.rs, fold::with_uncertainty (L5); also covers the branch the plan never tested — two rates with DIFFERENT periods refuse to fold, since Period rides inside MeasureKind's derived PartialEq"
  - "python3 -m pytest .claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py (L6 on BOTH gates — validate_meta for manifest rules and load_policies for registry policies, sharing one MEASURE_KIND_VOCAB — plus Rust/YAML kind-vocabulary parity read from measure.rs, not a copied constant)"
  - "python3 -m pytest .claude/scripts/_lib/__tests__/doc_dynamics_test.py (the index applied to this repo's own corpus — slice 2 upgraded it from a bare ratio to a STOCK: level + inflow + outflow, declared window, in-place-rename discrimination, and the Q12/Q13 zero-absorption guards mirrored from the Rust)"
  - "cargo test (elohim/epr) — measure_dimensional.rs: the dimensional algebra (MeasureKind::divide/combine_additive), Q12's sign check, Q10's multiplier_widen, and interval div/sub incl. the float-overflow case that manufactures +inf from two finite operands"
  - "cargo test (elohim/epr-rea) — stock_dynamics.rs: Stock refuses every incoherent construction, turnover/net-change/emission-absorption/harvest-regeneration, and stock_over_window folded from real FlowEvents"
  - "cargo test (elohim/eprfs) — flow_absorption.rs: the projection now mints the SINK (ReaVerb::Consume) and discriminates a move out of the value chain from an in-place rename against the recipe's stage globs"
  - "python3 -m pytest .claude/scripts/_lib/__tests__/intervenor_retire_when_test.py (retire-when on both gates + the census, incl. the two-implementation hash-exclusion invariant with eprfs-meta/src/canonical.rs)"
  - "cargo test (elohim/epr-rea) — model.rs: a limit declares WHICH SIDE is safe (Sense::{Ceiling,Floor}) with the band edge mirrored around it, and whether its number was claimed at the container or rolled up from the parts (LimitSource::{Declared,Folded{rule}}); Composition::{Min,Harmonic,Sum} refuses an empty part set rather than returning a capacity of zero"
  - "cargo test (elohim/epr-rea) — scope.rs + stock_dynamics.rs: a fold can say WHERE. Scopes::contains is transitive and reflexive over containment the DHT already notarizes, refuses cycles rather than depth-capping, and Within::{Anywhere,Scope,Under} makes the selection the fourth term of a stock's identity instead of a silent 'everywhere'"
gate_honesty: >
  CORRECTION, 2026-08-12 — read this before trusting any green above dated
  earlier. Two of this habit's checks (`uncertainty_closure.rs`,
  `stock_dynamics.rs`) live in `elohim/epr-rea`, which until 2026-08-12 had
  NO build-manifest entry, NO pipeline, and NO pre-push case: an epr-rea-only
  change matched no source glob, so the graph-walker emitted no project and
  no gate ran. Those two legs were hand-run only, and an epr-only change
  never compiled its one consumer. Every green this habit recorded before
  2026-08-12 therefore rests on two checks that CI never executed. The gate
  is wired now (elohim-epr widened to `-p elohim-epr -p elohim-epr-rea`
  across manifest, pre-push, and Jenkinsfile), so greens from here carry
  what earlier ones only claimed. Recorded rather than quietly fixed: this
  is the same eroding-goal channel `best_observed` exists to expose — a
  green that means less than it appears to.
best_observed: >
  HIGH-WATER MARK, slice 2 (2026-08-11) — the ratchet Meadows prescribes for
  *Drift to Low Performance*: "let standards be enhanced by the BEST actual
  performances instead of being discouraged by the worst." `evidence:` records
  LAST-observed, which cannot distinguish a genuinely green run from a
  deferred-gate green (the documented PVC-deferral channel). This field
  records the strongest run this habit has ever had, so a later weaker green
  reads as visibly weaker rather than equivalent.
  Best to date: 25 cargo test binaries green across elohim-epr + elohim-epr-rea
  (0 failed), clippy -D warnings exit 0 and fmt --check exit 0 on both crates;
  elohim-epr-cli + eprfs-meta full suites green with clippy 0 / fmt 0; 14
  doc_dynamics + 14 intervenor + 10 epr_meta_measure_ontology python tests
  passed. ALL GATES RUN, NONE DEFERRED — that last clause is the part the
  ratchet exists to preserve.
guard: >
  Regression risk = a future fold or index that special-cases its own
  zero denominator instead of reaching for the Interval::unknown() shape
  (spec Q13's general mechanism: a multiplier-based widening scheme cannot
  produce width from a zero base), or a caller routing around
  Confidence::widen() via a direct struct literal (Q8 — the fields are
  pub, so L4 is enforced by convention, not by the type). Neither is
  caught by a compiler error today.
  Slice 2 additions. (1) Q15 is NEW and open: Level ÷ Rate returns a bare
  Level, so the period a turnover time is denominated in survives only in
  the basis string — "43" is weeks or years depending on a divisor the type
  no longer carries, a weaker version of the exact forgetting Rate{per}
  exists to prevent. (2) The retire-when hash exclusion is a
  TWO-IMPLEMENTATION invariant (_lib/epr_meta.py `_HASH_EXCLUDE_KEYS` and
  eprfs-meta/src/canonical.rs `HASH_EXCLUDE_KEYS`); adding a key on one
  side only makes every backfilled policy fail its pin on the other and
  routes live deny/ask rules to judgment instead of enforcing them — a
  SILENT un-enforcement, caught here only because a live-root test existed.
  (3) The Python doc-corpus instrument and the Rust Stock are parallel
  implementations of one shape, held together by review and by mirrored
  tests, not by codegen.
refs:
  - "spec: genesis/docs/superpowers/specs/2026-08-11-measure-dynamics-confidence-ontology-design.md (six laws L1-L6, each anchored; Q1-Q14 open)"
  - "plan: genesis/docs/superpowers/plans/2026-08-11-measure-ontology-slice1-epr-local-first-plan.md"
  - "NOT covered — the NETWORK slice 2 (spec §4) is still untouched: per-fold anonymity (blocks the network rung), correlation-aware interval arithmetic (Q1), cross-peer determinism at a band edge, and the missing typed measure::canonical_bytes(&Quantity) entry point, so L2 remains serializer-proven but UNWIRED. The systems-discipline slice 2 landed instead, deliberately local."
  - "handoff: genesis/docs/superpowers/plans/2026-08-11-systems-discipline-slice2-handoff.md (items A-F; A/B/C/D/F landed, E — respite/response — is now COMPUTABLE via stock::respite_response but has no live numerator wired)"
  - "survey §6.2 corrected in place 2026-08-11: its overshoot proof cited levels-against-ceilings as rate ratios — the same dimensional error it diagnoses in spatial_capacity.rs. Conclusion survived, proof replaced."

deltas:
  - "2026-08-25c: a measure that could not see a shipped regression, recorded rather than quietly fixed. elohim.host: every EPR-card click landed on /epr/manifesto stuck on Loading content... with the node already fetched — ContentViewerComponent (mounted cross-bundle by the shell at /epr/:id) was left implicit-OnPush by the Eager-removal wave, so its subscribe-callback mutations marked no view dirty (proof: ng.getComponent isLoading:false + forced applyChanges rendered instantly; edge of elohim/dev #1670). Three instruments were blind by construction: the app pipeline E2E stage is a Cypress relic (binary missing → catchError → prints ✅ alpha validation passed, publishes no report, #1670); the only scenario touching the card RESOLVED it but never FOLLOWED it, and its feature is @act:i — HELD on the fleet lane (elohim-genesis/dev: every act-i scenario skipped-not-failed, @browser-only excluded from the mesh stage), so the new regression scenario cannot run in CI until an act-i browser lane exists; and TestBed.overrideComponent on the component under test recompiles it CheckAlways (ɵcmp.onPush true→false), so every spec-level OnPush guard that overrides the CUT is a null measure. Cured locally: Eager stamp + a browser-mirroring spec that FAILS without it (Eager host, production def) + a2o @regression Following the card (fails against the deployed alpha, ELOHIM_CAP_OWNED_SUBSTRATE_STATUS=available); residuals filed in backlog-onpush-eager-debt-inventory. DEPLOY EVIDENCE (elohim/dev #1673, fresh trigger): alpha.elohim.host serves the fixed bundle — card click renders the Manifesto at /epr/manifesto, no economic-events 500, no governance 400 (b512fbf5f); elohim.host does NOT — doorway-B refuses the byte-seed with 403 since edge #1380 closed the dev-mode remote seed hole and B binds a different api-key-admin secret than the Jenkins credential (backlog ci-app-apex-seed-403-doorway-b-admin-key). CORRECTED 2026-08-25d: this was recorded as an operator credential decision and is NOT one — every doorway api-key-admin is a plaintext fixture in the manifest's own stringData, applied verbatim, so it was always an in-tree fix; cured below. The strict a2o measure is still failed by the by-design signal/emit 503 the console-cleanliness hook counts (backlog console-noise-signal-emit-503-per-content-view). Also caught in flight: a step-definition edit dispatched edge #1383 — a full fleet redeploy — via the dataplane-validation glob genesis/a2o/steps/** (backlog ci-edge-a2o-steps-glob-redeploys-fleet). Flip still needs a build number on a green check."
retire-when: >
  when the measure ontology is enforced at the type level in every producer (a rate that
  forgot its period cannot be constructed), leaving the governance gate nothing to refuse.
---
GREEN 2026-08-11, verified on the merge tree (not an earlier run):
cargo test (elohim/epr) 19 suites ok / 0 failed, clippy -D warnings and
fmt --check both exit 0; cargo test (elohim/epr-rea) 4 suites ok / 0
failed, clippy and fmt exit 0; all 10 epr_meta_*.py exit 0;
doc_dynamics_test.py 6 passed. Live doc-corpus measure at merge:
generated=63 (28d window, git-witnessed adds under
genesis/docs/superpowers/{specs,plans}), absorbed_counted=0 (zero
delete/rename events in that window) → value=Infinity, claim=estimated,
interval={lo:-Infinity, hi:Infinity} (Interval::unknown()-shaped),
flag `~`. THIS IS THE HABIT'S OWN HONESTY TEST PASSING: an honestly
unbounded ratio, not a manufactured number. The first live run of this
measure produced a ZERO-WIDTH interval ([inf,inf], NaN width) — the
exact false precision the ontology exists to prevent — and that was
caught in review and cured by mirroring Interval::unknown(); the `~`
flag then emerged from the arithmetic with no change to the flag
expression. Absorption in this repo is bursty, which is why the window
matters: 28d → 63/0 (unknown); 90d → 318/98 (≈3.2).
HELD AND STRENGTHENED 2026-08-11 (slice 2): the vocabulary now has a
SPINE. Q12 and Q10 closed on the type (is_unknown gained its sign check,
so [+inf,+inf] no longer reports as honest absence; multiplier_widen
yields absence at a zero base); a dimensional algebra landed
(MeasureKind::divide/combine_additive) that makes the shipped
spatial_capacity.rs error non-silent — level ÷ rate now returns a TIME,
which cannot be read as a utilization; and interval div/sub route every
divide-by-a-band-admitting-zero to Interval::unknown() rather than +inf,
curing at the type what slice 1 patched at one call site. Its own test
found a case nobody had named: 1e300/1e-300 OVERFLOWS two finite operands
to exactly +inf, which now reports as [f64::MAX, +inf) — a bound we have,
not an infinity we claim. Live doc-corpus stock at close (328 live docs):
28d 64/0 → unknown; 90d 320/98 → 3.27 [1.09–3.27] with turnover 43wk
[14–43]; 365d 427/99 → 4.31 [1.44–4.31]. The 90d and 365d intervals lie
ENTIRELY above 1.0 — confirmed overshoot even at 3x the counted
absorption, which is a strictly stronger claim than the survey §6.2 rows
it replaces (those were levels against ceilings, and §6.2 is now
corrected in place).
