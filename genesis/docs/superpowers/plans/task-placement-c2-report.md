---
id: placement-c2-report
title: "C2 report — elohim-compute's runtime split, then elohim/placement/core"
status: DONE_WITH_CONCERNS
class: architecture
gap: plans__2026-10-20-placement-contract-architecture-plan#25
actor: agent:implementer@claude-opus-5-5
commits: []
habits: [materialize-what-is-read]
cites:
  - "placement-c2-brief | C2 | sha256:a73a1bfc9440c996 | path: genesis/docs/superpowers/plans/task-placement-c2-brief.md"
  - "placement-contract-interfaces-design | The placement contract's interfaces | sha256:de0208f9b6e426db | path: genesis/docs/superpowers/specs/2026-10-20-placement-contract-interfaces-design.md"
  - "placement-contract-architecture-plan | The placement contract's architecture | sha256:5417b3363dffdc44 | path: genesis/docs/superpowers/plans/2026-10-20-placement-contract-architecture-plan.md"
---

Item C2 (gap `#25`). Built in the working tree, uncommitted (architecture before landing: no
staging, commits or pushes). Write-set held: `elohim/elohim-compute/**`, `elohim/placement/**`,
`elohim/Cargo.toml`, `elohim/Cargo.lock`.

**Gate evidence:** `just gate elohim-placement` → `EXIT=0` (steps: `cargo fmt --check` EXIT=0;
`cargo clippy -p elohim-placement-core --all-targets --all-features -- -D warnings` EXIT=0;
`cargo test -p elohim-placement-core --all-features` EXIT=0, which is 51 unit + 9 ceiling + 19 floor + 3 schema + 7 doctests;
`cargo test -p elohim-compute --no-default-features` EXIT=0, 20 tests; `cargo clippy -p elohim-compute --no-default-features --all-targets -- -D warnings` EXIT=0).
`just gate elohim-compute` → `EXIT=0` (49 tests, default features). `just gate elohim-ark` (a
consumer of elohim-compute) → `EXIT=0`. `cargo metadata --locked` in `elohim/elohim-storage` →
`EXIT=0` and in `doorway/doorway-service` → `EXIT=0` (their lockfiles need no change). Cargo berth
claimed with `berth claim cargo --class verify --ttl 3600` (EXIT=0) and released (EXIT=0).

## 1. elohim-compute's `runtime` feature

- `elohim/elohim-compute/Cargo.toml`: `[features] default = ["runtime"]`, `runtime = [dep:serde_json, dep:dashmap, dep:chrono, dep:parking_lot]`. The four dependencies are optional. `serde_json` is also a dev-dependency, because actuation's wire-form test needs it with default features off.
- `elohim/elohim-compute/src/lib.rs`: `counters`, `health`, `peers`, `report` and `resources`, with their re-exports, are behind `#[cfg(feature = "runtime")]`. `actuation`, `build_info` and `detail_level` need only serde and are not gated.
- Consumers are unchanged: ark-core, elohim-storage and doorway-service take the default. The lane did not run elohim-storage's or doorway's full gate. Their compiled code is identical under the default, and a locked metadata resolve passes for both.

## 2. `elohim/placement/core` (new; package `elohim-placement-core`, lib `placement_core`)

A member of `elohim/Cargo.toml` (`"placement/core"`). Its dependencies are R-P1's exact set. `serde_json` and seam-contracts' `harness` are dev-dependencies only.

| File | What it holds |
|---|---|
| `Cargo.toml` | R-P1's set. Feature `testing` (turns on seam-contracts `harness`). `[[test]]` entries for floor and ceiling require `testing` |
| `src/lib.rs` | module map, re-exports, `boundary` rails (R-P1 `exact_dependency_set`, R-P2 `no_clock`, `no_io_or_async`, R-P3 `no_floats_in_ranking`, R-P7 `every_ranking_runs_the_contract`, `scan_covers_every_module`) |
| `src/kind.rs` | `KindId`, `GrantScope`, `required_scope`, `MishpatAction`, `Charge`, `PlacementKind`, `KindRegistry`, `MethodRegistry::builtin`. Under `testing`: the `BytesKind` double |
| `src/need.rs` | `NeedKey` (derived Serialize, hand-written Deserialize), `Demand`, `Requirements`, `Isolation`, `AgentRefKey`, `Expresser`, `NeedClass`, `NeedTerms`, `Need::id`, `Depth`, `PrivateNeed`, `NeedSource` |
| `src/offer.rs` | `Posture`, `OfferTerms`, `Capacity`, `DeviceFacts` (not Serialize) and its `posture`, `PersonChoices`, `Candidate`, `FailureDomain`, `Consent`, `EprScope`, `CidString`. R-P4 `compile_fail` doctest plus a compiling counterpart |
| `src/eligibility.rs` | `Eligible` (private fields), `Ineligible`, `IneligibleReason` and `IneligibleKind` (a ReasonLabel), `refusal()`, `unanswered()`, `admit`, the `Eligibility` Governor face whose `decide` runs the same stages, `Candidacy`. R-P5 `compile_fail` doctest plus a compiling counterpart |
| `src/rank.rs` | `MethodId::named` (a raw sha2-256 CID through `elohim_epr::cid::compute_raw_cid`), `is_method_name`, `Factor`, `Weights` (integer basis points), `RankKey`, `RankInput`, `Ranking`, `Scratch`, `XorDistance`, `DiversityAware`, `NearestBytes`, `WeightedCapacity`, `diversify`, `contract::assert_ranking_contract` (built on `check_ordering`) |
| `src/adopt.rs` | `Lease`, `Period`, `RecordPlane::for_need`, `CommitmentDraft`, `commitment_id` (byte-identical to `deterministic_custody_id` for CustodyBlob), `Carrying`, `Release`, `ReleaseReason`, `FulfilmentDraft`, `RealEvidence::from_readings` (Real only), `settle`. `compile_fail` doctest on the `RealEvidence` literal |
| `src/decide.rs` | `LossDomain`, `Charter::resolve`, `ForwardingRule`, `Snapshot::empty`, `Plan`, `decide` (fill, move within the cap, drain, observe, scarcity, NeedWithdrawn), `chosen`, `PrivateSnapshot`, `DeviceParams`, `PrivatePlan`, `decide_private` |
| `src/floor.rs` | `Trigger`, `affected`, `PortError`, `Ports` (with a defaulted `evidence`), `Floor::on`, `PassOutcome` |
| `src/explain.rs` | `Explanation`, `Outcome`, `NotCarriedReason` and `NotCarriedKind` (a ReasonLabel), `NotCarried`, `explain` |
| `src/params.rs` | `ParameterSet` (`cid`, `ranking_cid`), `HeadDeclaration`, `RatificationRef`, `RatificationKind`, `RatificationRule`, `InForce` (`builtin`, `from_head`, which verifies BuiltIn and Charter in core), `builtin_set`, `ParamRefusal` and `ParamRefusalKind`, `ParamFallback`, `SetSource`, `select_in_force` (verify first, then elect with `select_arbitrated_winner`). R-P6 `compile_fail` doctests: no `From<Draft>`, and no literal |
| `src/ceiling.rs` | `PressureKind`, `Pressure` (private fields, `count`, `own_need`), `Draft`, `within_band`, `ParameterHead`, `RatificationCheck`, `TuningSink` |
| `src/manifest.rs` | `ManifestNeeds`, `WorksOffline`, `ComputeNeed`, `ComputeBounds`, `ManifestContext`, `ManifestNeedsOut`, `GrantRequest`, `needs_from_manifest` |
| `src/cid_serde.rs` | a private copy of epr-rea's helper, pinned by a parity test (disagreement 1) |
| `tests/floor_contract.rs` | D contracts 1–16 and the worked examples (19 tests) |
| `tests/ceiling_contract.rs` | E contracts 1–7 and the pressure rule (9 tests) |
| `tests/schema_contract.rs` | reads lane S's enum schemas and `$defs.ManifestNeeds` with `include_str!` and checks ordered equality (3 tests) |

Governance: `elohim/placement/.epr-meta` is a file, modeled on `elohim/ark/.epr-meta`. It sets
`covers: subtree` and one `inject` rule, `decisions-live-only-in-core`, and its body names the
structural rails. `elohim/placement/build-manifest.json` declares gate project `elohim-placement`
(kind `just`, workspace `elohim`, `rustflags: ""`), and the step's inputs include the SDK enums and
the app-manifest schema the schema contract reads. `elohim/placement/justfile` holds the `gate`
recipe, and each step echoes its own `EXIT=` line.

### Tests added (by contract)

- Rails. R-P1, R-P2, R-P3, R-P7, the no-I/O/async scan and module coverage are in `lib.rs::boundary`. R-P4, R-P5, R-P6 and I9's private field are `compile_fail` doctests. Each was confirmed to fail for the intended reason by compiling it as a scratch example, deleted afterwards. The errors were E0277 `DeviceFacts: Serialize`, E0451 on `Eligible`'s private fields, E0277 `InForce: From<Draft>`, E0451 on `InForce`'s private fields, and E0451 on `RealEvidence.readings`.
- Ranking. `assert_ranking_contract` runs over all four built-ins (720 permutations each, three keys). `matches_storage_strategy_vectors` copies storage's two strategies verbatim as the oracle and runs every `placement.rs` test vector, including both orders of the conflicting-duplicate regression. Also added: the D15 multi-pass vector, the claim that NearestBytes ranks input holder over base holder over neither whatever the XOR order, the method-name grammar, and `MethodId` matching `raw_cid_from_sha256_hex`.
- Eligibility. The three Answer states for live, admitted and consent. Reach compared by openness. `CrossHouseholdBelowPublic`. `IsReceiver`. `NotNamed`. `OverCapacity`. The consent window and `epr_scope`. Both faces give the same first refusal. R7's three parts. Every LimitOwner.
- Floor (D 1–16). Agreement under input order, and the union of slices equals `chosen`. Self only. `check_quiescence` replay. A pending commitment is not re-drafted. Make-before-break within `max_moves_per_event`, settling to the new set's placement. Revert to the built-in set's placement. Hysteresis, with the carve-out where a domain gap fills and then drains. Every need explained, with the digest matching the commitment. No refusal reachable through five generated sets. R2 `HouseholdLocal { household: "A" }` and the intimate worked example (`DomainShort { 1, 2 }`, `posture-changed`, hh:B `CrossHouseholdBelowPublic`, identical explanations). The public worked example fills the domain gap. Convergence under built-ins only. Idle is free through `Floor` with a `Ports` double: a quiet pass, a skipped `CarrierLost`, and `PeriodClosed` settling only. The legacy contracts: incumbents and a first pass that moves nothing, committed but not honored, a transport-id row counted as me. A dead carrier refills and is never released on `Unreachable`. Capacity charged once across two scopes.
- Ceiling (E 1–7). A lenient check cannot admit BuiltIn or Charter. A set for another scope or kind is a Mismatch. An unratified newer declaration never shadows an older one, and the refusal is reported with `UseHeld`. No declarations fall back to the last verified set or the built-in. An unfetchable set is Unverifiable. A revert is a newer declaration. Same declarations in either order give the same set. `within_band` refuses an unknown factor, removed keys, a zero weight, a move outside the range, and any weight under XorDistance. Republishing at another reach keeps `ranking_cid`. A pressure names only the drafter's own need, and only at a reach no narrower than the draft's.
- G1 vectors. JSON → `Need` → DAG-CBOR gives the same `Need::id`. `NeedKey` and `Need` round-trip DAG-CBOR. `NeedKey` decodes with its fields in either order. A negative test pins why `NeedTerms` holds a `CidString` (disagreement 3). The `cid_serde` parity test compares against epr-rea's `ModelPin`.

## 3. Places the code and the interfaces spec disagree (for the spec seat to absorb)

1. **`cid_serde` is private in epr-rea.** §1.4 says placement-core must not keep its own copy. The `pub` re-export is a follow-on outside this write-set. `src/cid_serde.rs` is therefore a temporary copy, held byte-identical by `matches_epr_rea_encoding_in_both_formats`, which compares epr-rea's `ModelPin` and a mirror struct in JSON and DAG-CBOR. Delete the copy when epr-rea exports its helper.
2. **`NeedKey`'s derived adjacently tagged `Deserialize` cannot read DAG-CBOR.** Canonical key order puts `id` before `form`, so serde buffers `id` and loses the tag-42 link. The test failed with `invalid type: byte array, expected a valid CID … newtype struct`. The wire form stays as the spec gives it, with Serialize derived. Deserialize is now written by hand, reads either field order, and decodes the link through cid's private newtype the way `cid 0.11 serde.rs` does.
3. **`NeedTerms::Task.input_root` is a `CidString`, not a `Cid` under `cid_serde`.** The spec puts a CID inside an internally tagged enum, which G1's own rule forbids. `a_cid_inside_an_internally_tagged_enum_does_not_decode_from_dag_cbor` pins the failure. JSON is unchanged, but in DAG-CBOR the field is now a string rather than tag 42, so it is part of `Need::id`'s canonical bytes. `Need::input_root()` parses it.
4. **`admit` gains `receiver_domain: Option<&FailureDomain>`, and `Candidacy` gains the same field.** `CrossHouseholdBelowPublic` (D13) and own-household-only offers (D11) cannot be decided from the candidate alone. `decide` resolves the domain from the receiver's own candidate. When a below-public need's receiver household is unknown, the refusal is `Unknown { input: "household" }`, never a default. Open question: a receiver with no posture of its own has no domain, so the `Snapshot` may need a receiver-domain map.
5. **A `household_only` offer meeting another household's need is refused `Kind { label: "household-only" }`.** No reason variant names it. Propose `IneligibleReason::HouseholdOnly`.
6. **`OfferTerms.reach_ceiling` is enforced in the reach step**, as `OutsideReach { admitted: ceiling }`. The spec names the field but no check.
7. **`admitted` = `Absent` gives `OutsideReach { admitted: Commons }`**, because the variant requires a reach and Absent has none. A missing map entry for consent or admitted reads as `Unreachable`, which is `Unknown`.
8. **`Ports::evidence(&Carrying, Period) -> Option<(u64, RealEvidence)>` is added, defaulted to `None`.** `PeriodClosed` settles, settling needs `RealEvidence` (I9), and no snapshot field holds readings. `decide` therefore never fills `Plan.settle`; `Floor::on(PeriodClosed)` does.
9. **Additive public surface:** `chosen()` (the global assignment, for contract 1), `NotCarriedKind` and `ParamRefusalKind` as the ReasonLabel faces §2.0 requires, `kind()` on each reason, `KindId::ALL`, `GrantScope::ALL`, `Factor::ALL`, `NeedClass::BUILTIN_ORDER`, `label()`s, `Snapshot::empty`, `builtin_set`, `Weights::{new, with, get, iter}`, `FailureDomain::{as_str, household}`, `AgentRefKey::{of, as_str}`, `Capacity::of_offer`, `Pressure::{kind, key, n}`, `Floor::ports`, and `Need::input_root`. Extra derives (Clone/PartialEq/Eq on `PlaceError`, Copy on `Eligible`, and others) are additive too.
10. **Capacity is one pool per carrier.** It is the component-wise widest of that carrier's per-kind offers, charged once across kinds and scopes in serve order, and only for new placements (an honored holder is not charged again). The spec does not say how per-kind offers share one device's capacity.
11. **`Factor::AlwaysOn` reads as `!offer.deferred`.** `Posture` carries no always-on fact. `Requirements.always_on` is left to the kind's `fits`, because the universal checks do not name it.
12. **NearestBytes ranks lexicographically:** input-root holders, then base-root holders, then the receiver's domain, then distance, then XOR. Its weights scale only within a tier, so they do not reorder across tiers, and the contract's "weights reorder" is vacuous for it. WeightedCapacity is a basis-point sum, then XOR. Its default weights (Distance 5000, SpareCapacity 3000, AlwaysOn 2000) and NearestBytes's (NearBytes 10000, Distance 1) are placeholders; §6 leaves them open.
13. **Placeholder numbers:** `BUILTIN_MAX_MOVES_PER_EVENT = 1`, with the built-in exempt from the charter's move range. `FulfilmentDraft.unit` is `byte-period` for custody, `task` for task, `period` otherwise. Both are open in §6.
14. **`within_band` cannot apply the revert exemption.** It is given no history of the space's verified sets, so the caller routes a revert past it; `from_head` still verifies the revert's ratification.
15. **`select_in_force`'s second value** is the refusal of the newest declaration newer than the elected one, or of the newest declaration when the fallback answered. `meanwhile` is `UseHeld` when a verified or last-verified set answered, else `BuiltInInForce`. `LastVerified` is never reported by `explain`, because `Snapshot.params` holds `InForce` without where it came from.
16. **`decide` clamps `need.demand` to `charter.holding_floor`, with domains ≤ holders**, applying `Charter::resolve`'s rule to needs that arrive already resolved. One drain per need per pass. The move cap is counted per (scope, kind) per pass. `Plan.unserved` lists every unserved need, not only `me`'s. `NeedWithdrawn` releases my commitment for a registered kind whose need is absent from the snapshot, which relies on `Ports::snapshot` loading every scope's needs.
17. **`explain` returns `None` when the snapshot no longer holds the need.** The three fields a commitment carries are not in the snapshot, so `InputsGone` is not produced from it. `Carried` means honored holders meet holders and domains. `Held` means the chosen carriers meet demand but honoring has not caught up.
18. **LimitOwner mapping.** `OutsideReach` is carried as `Operator`, with the space named in `elevate`, until `LimitOwner::Charter` lands. `Unknown`, `IsReceiver`, `CrossHouseholdBelowPublic` and `Kind` map to `Operator`. `OverCapacity` and `NoPosture` map to `SelfLimit`. `NoConsent`, `ConsentExpired` and `NotNamed` map to `Commitment`.
19. **`required_scope` moved as `required_scope(project: Option<&str>) -> GrantScope`.** It reads the envelope project prefix, because the method descriptor lives in eprfs-core, which R-P1 keeps out.
20. **`#![allow(clippy::result_large_err)]`.** R7's by-value `Unanswered<R>` in the spec's `Err` types trips the lint, and boxing it would change the signatures.

## 4. Concerns

- The schema contract `include_str!`s lane S's `elohim/sdk/schemas/v1/enums/*.schema.json` and `manifest/app-manifest.schema.json`. They are uncommitted working-tree files from another lane, present and equal in order today, so this crate's tests cannot compile without them. They land together, or lane S lands first.
- The **brand-vocabulary advisory** flagged `MishpatAction`. It is the spec's own name, kept as written. The spec seat decides.
- **Not done here (outside this write-set):**
  - the root `justfile`'s `_gate-elohim-compute` does not test `--no-default-features`; `just gate elohim-placement` covers it;
  - epr-rea's `pub cid_serde`;
  - `LimitOwner::Charter`;
  - R-P8 and the storage `PlacementStrategy` shim (elohim-storage);
  - ark-core's `task.rs`, which would implement `TaskKind` and map `Isolation`/`Depth`;
  - the habit atom delta for `materialize-what-is-read`. Its atom lives under lane E's `elohim/eprfs/.epr-meta/` and elohim-storage's, and the delta belongs to the seat that lands the first adapter.
- The cargo berth was held through session ancestry ("renewed by" this session) and released at the end. A parent lane that still needs it must re-claim it.

## Fix round 1

The nine review findings were checked against the code. Findings 2 and 5 are the same defect, and so are 4 and 7. Each finding is listed below with what was done.

**Gate evidence:** `just gate elohim-placement` → `EXIT=0`. Steps: `cargo fmt --check` EXIT=0; `cargo clippy -p elohim-placement-core --all-targets --all-features -- -D warnings` EXIT=0; `cargo test -p elohim-placement-core --all-features` EXIT=0 (52 unit, 11 ceiling, 22 floor, 3 schema and 7 doctests); `cargo test -p elohim-compute --no-default-features` EXIT=0 (20 tests); `cargo clippy -p elohim-compute --no-default-features --all-targets -- -D warnings` EXIT=0. The first run failed clippy, on an unused import and `too_many_arguments` on `select_in_force`; both were fixed before the run above. The cargo berth was claimed with `berth claim cargo --class verify --ttl 3600` (EXIT=0). The gate waited for another session's `cargo test --workspace` in `elohim/eprfs` to finish. `berth release cargo` → EXIT=0, reporting "was not held by" this session, so the lease had already lapsed or been taken over by then.

### Fixed

- **F1, re-expression churns a commitment (decide.rs).** Carryings now match a need by the identity `commitment_id` hashes, `(kind, key, receiver)` (`same_commitment`), not by `Need::id`. A re-expressed need with a wider reach or another demand therefore counts the old carryings as its own: honored, live, and incumbent by `under_params`. `NeedWithdrawn` fires only when no need in the snapshot has that identity. If an old and a re-expressed need are both still held, `decide` drafts and releases each commitment id only once. Test `c4_a_re_expressed_need_keeps_its_commitments`: the same key is widened from Intimate to Public. No plan both adopts and releases one id, nothing is `NeedWithdrawn`, and the holders stay honored. When the need is really gone, each holder releases.
- **F2 = F5, several successors name one predecessor (decide.rs).** The move loop keeps a `claimed` set, so each superseded commitment is named by one successor only. Test `c4_a_cap_above_one_supersedes_distinct_holders` uses `max_moves_per_event = 2` and two stale holders, and asserts two moves with distinct `replaces`.
- **F3, a ratification is not bound to its head (ceiling.rs, params.rs).** The signature is revised to `RatificationCheck::verify(&self, head: &HeadDeclaration, charter)`, which hands over the whole head. The adapter must prove that the act names exactly this head's `scope`, `sets` and `ratified_at`. That makes the election clock a proven fact rather than a declarer's field. Test `e9_a_ratification_names_its_head_exactly`: an act for set X attached to a head for set Y is `Refused`. A republished copy with an inflated `ratified_at` is refused, and the newest proven declaration wins. The ceiling doubles (`Acts`) now record the heads their acts name.
- **F4 = F7, the band is unchecked on the floor path (params.rs, ceiling.rs).** A new `structural_band` holds the band checks that need no history. Every weight must be > 0. When this peer runs the method, the weight keys must be exactly `method.factors()` (`UnknownFactor`, or `Band { every-factor-weighted }`). `max_moves_per_event` must lie inside `charter.moves_per_event`, and an inverted `lo > hi` range admits nothing (`Band { move-range }`). `InForce::from_head` runs it for `Charter` and `Ratified` sets, and `within_band` now calls it too. The signature is revised: `from_head` and `select_in_force` take `methods: &MethodRegistry` (between `check` and `charter`). Test `e6_an_off_band_set_is_never_in_force_whatever_the_check` covers negative, zero, foreign-factor, missing-factor and `u16::MAX` sets, each under a lenient check and as charter-named, plus an inverted charter range.
- **F6, `NotNeeded` bypasses the grant (eligibility.rs).** The code already held when this round began. `stage_consent` accepts `NotNeeded` only when the carrier is the need's receiver (§F5's "requester's own envelope") and otherwise refuses `NoConsent`. Only the test was missing. Added `not_needed_stands_only_for_the_receivers_own_carrier`: another carrier is `NoConsent`, the receiver passes, and a kind that needs no grant ignores the map.
- **F8, contract 8 is not true as written (floor_contract.rs).** Ruling (a) is taken. Contract 8 is restated per need, given the capacity the pass has left: no reason other than `OverCapacity` changes with the parameter set, and an `OverCapacity` that was not there at the start of the pass comes only from serve-order charging, meaning the carrier was chosen for a need served earlier in the same pass. c8 is now a generated property over 49 sets (the built-in weights plus 48 from an xorshift generator, with no new dependency). It uses two needs that share a capacity-limited carrier, `narrow` with 15 bytes for two 10-byte needs. It asserts that the charging channel was exercised, which fails if it was not.
- **F9, a need sets its own demand (decide.rs).** The fix adds `Charter::ceiling()`, the widest demand the charter declares for any loss domain and never below `holding_floor`, and `Charter::bound(asked)`, which raises an ask to the floor, caps it at the ceiling, and keeps `domains <= holders`. `decide` places at `charter.bound(need.demand)`. Test `a_need_cannot_raise_its_own_demand_past_the_charter`: an ask of `{500, 9}` is placed at the ceiling `{3, 2}`. The floor fixture's charter now declares `Household {2,1}` and `Region {3,2}`.

### Refused, in part

- **F9's "name the over-ask".** The ask is clamped, not reported by name. A new `NotCarriedReason` variant, or an `Explanation` field, would change `views/placement-explanation-view.schema.json` (`additionalProperties: false`), which is outside this write-set. `Need` also names no `LossDomain`, so the cap is the charter's widest declared demand rather than one loss domain's. Adding `loss` to `Need` would move `Need::id` and lane S's schemas.
- **F7's "refuse `UnknownMethod` when the method is absent".** Refused. A peer that does not run a method only observes needs under it (D7 `MethodNotRun`). If `from_head` refused the set, that peer would fall back to the built-in and place under a different set from the peers that run the method, which breaks E5 agreement. With no method present, the checks that need no method (weights > 0, move range) still apply, and the factor check is skipped.

### Spec revisions for the spec seat (adds to §3)

21. `RatificationCheck::verify(&self, head: &HeadDeclaration, charter: &Charter)`. The act must name the head's scope, sets and `ratified_at`.
22. `InForce::from_head(scope, kind, set, head, check, methods, charter)` and `select_in_force(scope, kind, decls, sets, check, methods, charter, last_verified)` both take `methods`, and `from_head` applies the band checks that need no history to `Charter` and `Ratified` sets. `select_in_force` carries `#[allow(clippy::too_many_arguments)]`.
23. Contract D8 is restated per need, given the remaining capacity, as above.
24. `Charter::ceiling` and `Charter::bound` are added, and `decide` places at `bound(need.demand)`. This replaces entry 16's "clamps to `holding_floor`". Open question: whether `Need` should name its `LossDomain`, so the cap becomes `resolve(loss)`.
25. A carrying belongs to a need by `(kind, key, receiver)`, not by `Need::id`. This revises D10's reading of `Carrying.need`, which stays as a record of the id the commitment was adopted under. `commitment_id` ignores `in_scope_of`, so two scopes holding one identity share one commitment. That limitation predates this round.

Concern (new): a `Ports::snapshot` adapter must declare charter demand. A charter with an empty `demand` map, which is every household today, places every need at `holding_floor`. No adapter exists yet, so running behaviour is unchanged.

## Fix round 2

I checked each of the eight review findings against the code. Six hold and are fixed. One is
refused because it falls outside this lane, and one is fixed only in part. Nothing is staged or
committed (architecture before landing).

**Gate evidence:** `just gate elohim-placement` → `EXIT=0`. Steps: `cargo fmt --check` EXIT=0;
`cargo clippy -p elohim-placement-core --all-targets --all-features -- -D warnings` EXIT=0;
`cargo test -p elohim-placement-core --all-features` EXIT=0 (56 unit, 13 ceiling, 23 floor,
3 schema and 9 doctests); `cargo test -p elohim-compute --no-default-features` EXIT=0 (21);
`cargo clippy -p elohim-compute --no-default-features --all-targets -- -D warnings` EXIT=0.
`just gate elohim-compute` → `EXIT=0` (50 tests). `just gate elohim-ark` → `EXIT=0`. The earlier runs in
this round failed on fmt, then on clippy (`useless_conversion` on a `&'static str` reason, and
`result_large_err` on a test closure). Both were fixed before the passing run. The cargo berth was claimed
with `berth claim cargo --class verify --ttl 3600` (EXIT=0) and released (EXIT=0).

### Fixed

- **R2-1: a forged BuiltIn or Charter head wins the election (params.rs).** The election is now
  tiered: `election_key` ranks `Ratified` (2) above `Charter` (1) above `BuiltIn` (0). The clock is
  `ratified_at` only for `Ratified`, the one kind whose clock `check.verify` proves. Everything else
  gets 0. A verified `BuiltIn` head is not a candidate at all. It means "no tuning chosen", so it never
  displaces `last_verified`. `from_head` now refuses a `BuiltIn` head unless `ratification.id ==
  set_cid.to_string()`, `rule` is `None` and `touches` is empty, and it returns the canonical
  `InForce::builtin(kind)` rather than the head's copy. A `Charter` head must also carry no `rule` and
  no `touches`. `newer_refusal` compares by the same election key. Tests:
  `e4_an_unproven_clock_on_a_builtin_or_charter_head_never_wins` covers forged BuiltIn and Charter heads at
  `ratified_at = u64::MAX`, in three orders, against one ratified tuning. It also checks that Charter
  beats BuiltIn and that a BuiltIn head does not displace `last_verified`.
  `e1_a_builtin_or_charter_head_carries_no_unproven_provenance` covers a wrong id, a rule and touches on
  BuiltIn, and touches on Charter. The e1 fixture's BuiltIn head now carries the set's CID as its id.
- **R2-3: the device budget is never enforced (decide.rs, manifest.rs).** A `PrivateNeed` with
  `budget_bytes: None` is now refused by name as `Unknown { input: "need-size" }`. It is no longer kept
  as free. The published custody `spare_bytes` is now the budget left after the device's own keeps.
  `ManifestContext` gains `closure_bytes: Option<u64>`, which the caller reads from the same tree view,
  and `needs_from_manifest` passes it through as the need's size. Tests in `decide::private_tests`:
  `an_unsized_need_is_refused_by_name_never_kept_free` (budget 0) and
  `the_published_spare_is_the_budget_after_own_keeps` (1000 − 300 − 200 = 500).
- **R2-4: `render` yields an `Eligible` without consent (eligibility.rs).** The `Eligibility` face's
  `Effect` is now `Charge`, not `Eligible`, so only `admit` builds an `Eligible`. The overridden `decide`
  is removed. The default spine (authorize → gate → render) runs the stages in `admit`'s order, and
  the existing two-faces test still shows the same first refusal on both faces. A new `compile_fail`
  doctest shows that `Eligibility::new(kind).render(..)` cannot return an `Eligible`, with a
  compiling counterpart that returns the `Charge` to prove the imports are right. The module doc no
  longer claims that the face builds an `Eligible`.
- **R2-5: `MethodRegistry::with` replaces built-ins (kind.rs, rank.rs).** The signature is now
  `with(self, name: &str, r) -> Result<Self, PlaceError>`. It refuses a malformed name, an id that is
  not `MethodId::named(name)`, any `elohim:` name (the namespace is reserved for built-ins), and an id
  already registered. `builtin()` inserts through the private map. The `Ranking` trait's docs now
  state that an implementor runs `contract::assert_ranking_contract` in its own tests. Test:
  `a_registered_method_never_replaces_or_impersonates_another`.
- **R2-6: R-P1's parser misses dotted and target tables (lib.rs).** The new
  `parse_dependencies(&str) -> Result<..>` reads `[dependencies.<name>]` as the dependency `<name>`.
  It refuses any other non-dev table that declares dependencies: `[target.*.dependencies]`, their
  dotted forms, `[build-dependencies]` and `[build-dependencies.*]`. It skips `[dev-dependencies]` and
  its dotted form. Test: `the_dependency_parser_sees_every_form_that_adds_a_dependency`, over fixture TOML.
  ark-core's `no_runtime_or_io_deps` has the same blind spot. Its needles `"\n{pkg} "` and
  `"\n{pkg}="` never match a `[dependencies.tokio]` header. That crate is outside this write-set, so
  the fix is filed under concerns below.
- **R2-7: a charter's membrane is labelled an operator veto (actuation.rs, eligibility.rs).**
  `LimitOwner::Charter` (serde `"charter"`) is landed in elohim-compute. A repo-wide grep found no
  exhaustive `match` on `LimitOwner`: ark-core, elohim-storage's `arc_actuator` and placement only
  construct or compare it. `#[non_exhaustive]` was not added, because that changes the public API for
  every consumer and is the spec seat's decision. `OutsideReach` now maps to `Charter`.
  `CrossHouseholdBelowPublic` stays `Operator`, and that half of the finding is refused: the spec
  holds it "until per-space cells exist" (D13), so it is a runtime limit, not a charter's rule.
  Tests: `a_charter_refusal_is_neither_self_sovereign_nor_an_operator_veto` and the serde form in
  actuation, plus `refusals_name_whose_line_they_honored` for `OutsideReach`.
- **R2-8: a failed need read drains every carrying (need.rs, decide.rs, adopt.rs).**
  `NeedSource::needs` now returns `Answer<Vec<Need>>`. `Snapshot` gains
  `needs_read: BTreeMap<CidString, Answer<()>>`. A carrying is released `NeedWithdrawn` only when its
  scope reads `Present`. A scope with no entry was not read, and nothing in it is withdrawn.
  `Carrying` gains `in_scope_of: Cid` (from `CommitmentDraft.in_scope_of`) so that the scope is
  known. A need whose `id()` fails to encode is no longer dropped. It still counts as expressed, so its
  carryings are held, and it is named in the new `Plan.unencodable: Vec<(KindId, NeedKey, AgentRef)>`.
  No reachable input makes `Need::id` fail today, so that branch has no test. Test:
  `a_failed_need_read_never_drains_a_scope`. An `Unreachable` scope and an unread scope release
  nothing, and a scope that reads `Present` and is genuinely empty releases every holder. The floor
  fixture marks each need's scope `Present`.

### Refused

- **R2-2: placement builds only against uncommitted seam-contracts work.** The finding is accurate
  but needs no code change here. `crates/seam-contracts/**` is lane C1's write-set
  (`task-placement-c1-brief.md`: "Write-set for this lane: `crates/seam-contracts/**`"). The brief
  authorized it, and C1's report lists `unanswered.rs`, `probe.rs`, `harness/status.rs` and the module
  exports. This lane cannot commit, and the files are not in its write-set. **The landing order is
  binding: C1's seam-contracts change lands in the same commit set as placement, or before it.** The
  `elohim-seam-contracts → serde` edge in `elohim/Cargo.lock` lands with that change. The placement
  manifest already lists `crates/seam-contracts/**` among its sources.
- **R2-7, in part:** `CrossHouseholdBelowPublic` stays `Operator`, for the reason given above.
- **R2-5, in part:** R-P8, a consumer rail that checks an adapter's `assert_ranking_contract` call,
  belongs to the storage shim, which is outside this write-set. The obligation is stated on the trait.

### Spec revisions for the spec seat (adds to §3)

26. The election key is `(tier: Ratified 2 > Charter 1 > BuiltIn 0, clock: ratified_at for Ratified
    only, tiebreak: head CID)`. This replaces E4's single tier. A verified BuiltIn head is not elected.
    `RatificationRef` on a BuiltIn head is exactly `{id: set CID, rule: None, touches: []}`. On a
    Charter head, `rule` is None and `touches` is empty.
27. `Governor for Eligibility` has `type Effect = Charge`, not `Eligible<'a>`, and keeps the default
    `decide`.
28. `MethodRegistry::with(self, name: &str, r: Box<dyn Ranking>) -> Result<Self, PlaceError>`.
29. `NeedSource::needs(&self, scope) -> Answer<Vec<Need>>`, `Snapshot.needs_read`,
    `Carrying.in_scope_of` and `Plan.unencodable`. These revise D10 and D18's withdrawal rule.
30. `ManifestContext.closure_bytes`. An unsized `PrivateNeed` is refused as `Unknown { input: "need-size" }`,
    and the published `spare_bytes` is net of the device's own keeps.
31. `LimitOwner::Charter` is landed. This closes entry 18's "until `LimitOwner::Charter` lands".

### Concerns (new)

- ark-core's `no_runtime_or_io_deps` (`elohim/ark/core/src/lib.rs:71`) cannot see `[dependencies.<pkg>]`
  or `[target.*.dependencies]` tables. It belongs to a follow-on for the ark seat, outside this write-set.
- elohim-storage's and doorway's full gates were not run for `LimitOwner::Charter`. The grep shows no
  exhaustive match in either. `just gate elohim-ark` and `just gate elohim-compute` pass.
- The covenant floor was not needed. This round changed code only inside the brief's write-set,
  apart from appending to this report.

## Fix round 1 (third review)

This review raised twelve findings. I checked each one against the code. Findings 2 and 11 describe the same defect, and so do 3 and 8. Nine distinct defects are fixed. One fix is partial, because the view schema is outside this lane's write-set. Finding 12 is refused, with the reason given below. Nothing is staged or committed.

**Gate evidence:** `just gate elohim-placement` → `EXIT=0`. Steps: `cargo fmt --check -p elohim-placement-core` EXIT=0; `cargo clippy -p elohim-placement-core --all-targets --all-features -- -D warnings` EXIT=0; `cargo test -p elohim-placement-core --all-features` EXIT=0 (61 unit, 14 ceiling, 24 floor, 3 schema, 9 doctests); `cargo test -p elohim-compute --no-default-features` EXIT=0 (21); `cargo clippy -p elohim-compute --no-default-features --all-targets -- -D warnings` EXIT=0. The first two runs this round failed clippy: `nonminimal_bool` on the isolation check, then `clone_on_copy` in c14. Both were fixed before the passing run. elohim-compute was not edited this round. I checked that the tests can fail. With the liveness filter reverted and the per-kind offer replaced by the carrier's widest offer, `c14_a_dead_carrier_does_not_hold_a_need` and `c15_a_need_draws_only_on_its_own_kinds_offer_and_one_device_is_charged_once` both fail (floor_contract EXIT=101, 22 passed, 2 failed). The source was restored byte-identical (`cmp`) before the final gate. The cargo berth was claimed with `berth claim cargo --class verify --ttl 3600` (EXIT=0, "renewed by" this session). `berth release cargo` returned EXIT=0 with "was not held by" this session: the gate runner's own claim and release had already ended the lease.

### Fixed

- **T1: a dead carrier holds its need (decide.rs).** A carrying now counts as honored only when its claim reads `Present(true)` *and* its carrier's candidate reads `live == Present(true)`. `run_pass` builds the set of live carriers. A silent carrier is not counted, so the need re-fills, but it is not released either: `Unreachable` still does not mean not-honored, and it drains only after it comes back live and the need is over-held. c14 now has two legs. In the first, the claim reads `Present(true)` and only liveness is `Unreachable`. In the second, both are `Unreachable`. Each leg asserts a single re-fill by another carrier, an `Unknown { liveness }` refusal, an outcome that is not `Carried`, and no release. Each then applies the plans and asserts quiescence and `Carried`. The mutation check above shows c14 fails without the fix.
- **T2 = T11: the election puts a tuning above its charter (params.rs, decide.rs).** The election has one tier again. `Charter` gains `adopted_at: EpochSecs`, taken from the adoption record the snapshot builder holds (0 when there is no charter). `election_key(decl, charter, cid)` sets the clock to `charter.adopted_at` for a `Charter` head, to `ratified_at` for a `Ratified` head (proven by `check.verify`, E9), and to 0 for a `BuiltIn` head, which is never elected anyway. The clock is a proven fact for every candidate, so fix round 2's forged-clock defect stays fixed and authority is not reordered. The head's own `ratified_at` on a `Charter` head is never read. A `Charter` head verifies only against the charter in force, so a superseded charter's head does not verify, whatever its clock. Test `e4_a_charter_amendment_displaces_an_older_tuning_and_a_later_tuning_displaces_it`: an amendment adopted at 20 displaces a tuning ratified at 10, in either order. A tuning ratified at 30 then displaces the amendment. The superseded charter's head stays out at `u64::MAX`. The existing e4 test still passes, with the fixture charter adopted at 5.
- **T3 = T8: capacity is the carrier's widest offer, and the floor has no capacity check (decide.rs, eligibility.rs, offer.rs, kind.rs).** `Capacity::widest` is removed. `decide` now keeps `charged: carrier → Charge`, summed across kinds and scopes (`Charge::plus`). It admits a need against `remaining_for(candidate, need.kind, charged)`, which is `Capacity::of_offer(the offer for this kind)` net of that sum. No kind can draw on capacity another kind offered, and one device is still charged only once. A new universal stage, `stage_capacity`, runs in both faces after scope and isolation and before `kind.fits`. It charges every component the need requires (bytes, memory, cpu, pids). It refuses `OverCapacity` past what is left, and refuses `Unknown { input: "capacity" }` when the need requires a component the offer does not state (spec §7.2), so a missing figure is never read as unbounded. `fits` now only narrows further. Tests: `capacity_is_the_floors_refusal_before_the_kinds` (replacing `over_capacity_is_the_kinds_refusal`) uses a kind that admits anything, and covers over-capacity, unknown capacity, and pids both over and unstated. `c15_a_need_draws_only_on_its_own_kinds_offer_and_one_device_is_charged_once` has two offers on one carrier: 15 bytes of custody-root and 1000 of custody-blob. A 100-byte root need is `OverCapacity` on that carrier. A 990-byte blob placed there leaves nothing for a 10-byte root, which goes to the other carrier, while the same root need alone fits.
- **T4: scopes and isolation are never checked (eligibility.rs).** `stage_gate` now refuses `Kind { label: "scope" }` when `kind.scope(need)` is `Some(s)` and `offer.scopes` lacks `s`. It refuses `Kind { label: "isolation" }` when `need.requires.isolation` is `Some(floor)` and the offer's isolation is unknown or weaker. Both checks run before capacity and `fits`. The labels are the ones §F gave TaskKind, so a TaskKind that repeats the checks agrees with core. Test `an_offer_must_spend_the_scope_and_meet_the_isolation` uses a kind that admits anything, and covers no scopes, another scope, the right scope, no isolation, weaker isolation, and equal or stronger isolation.
- **T5: the required scope comes from the requester's own bytes (kind.rs, eligibility.rs).** `NeedTerms::Task.scope` stays on the wire, because lane S's `placement-need.schema.json` declares it. It is now a declaration that core checks against the kind's derivation. `stage_consent` refuses `Kind { label: "scope" }` for every carrier when `terms.scope != kind.scope(need)`. The `BytesKind` double now derives `scope()` and `action()` from its own field, never from the need. A new contract, `kind::testing::assert_scope_is_derived(kind, need)`, rewrites the need's declared scope to every `GrantScope` and asserts that neither `scope()` nor `action()` moves. `PlacementKind::scope`'s doc requires the derivation (through `required_scope` from the TaskSpec's project) and the contract. Tests: `a_need_declaring_another_scope_than_its_kind_derives_is_refused` covers a requester holding only a sweettest-feedback grant who writes that scope on a measure-stage need. The need is refused, and the honest need is `NoConsent` under the same grant. `a_kind_derives_its_scope_and_never_reads_the_requesters` runs the contract over both doubles and shows that a kind reading the need's field panics.
- **T6: `ManifestContext` fields are never read (manifest.rs).** `closure_key` and `closure_bytes` become `closure: Option<ClosureKey { key, root, paths, bytes }>`. `needs_from_manifest` refuses a key whose `root` is not `release_root`, or whose `paths` are not the declared closure as a set. `GrantRequest` now carries `requester` (from `receiver`), `in_scope_of` (from `scope`), `reach` and `manifest`. Every remaining context field reaches the output. `class` is removed: no output of `needs_from_manifest` is a `Need`, and a Task need takes its class from `TaskSpec::as_need`. Tests: `a_closure_key_is_bound_to_the_release_root_and_the_declared_paths` (either order passes; fewer paths, another root, or a moved release root is refused) and `compute_becomes_standing_grant_requests_naming_who_where_and_whence`, in which changing each field changes the output.
- **T7: any real reading settles any commitment, more than once (adopt.rs, floor.rs).** `RealEvidence::from_readings(subject, readings)` refuses any reading whose `subject` is another one, and `RealEvidence` keeps the subject. `settle` now returns a `Result` and refuses evidence whose subject is not `carrying.key.as_id_string()`. `FulfilmentDraft` gains `id = fulfilment_id(commitment, period)`, which is `"fulfilment-"` + hex16(sha256(commitment | period.start)). `Ports::settle` is now documented as idempotent by that id. `Floor::on(PeriodClosed)` also drafts each id only once per call, and skips evidence that `settle` refuses. Tests: `evidence_settles_only_its_own_subject_once_per_period` (mixed subjects refused; another subject's reading refused; the same id across quantities; a new id for the next period), and c11 now re-delivers `PeriodClosed` and asserts the same fulfilment id.
- **T9: NearestBytes's weights cannot reorder (rank.rs, params.rs).** NearestBytes now reads no weights. `factors()` returns `&[]`, as XorDistance's does, so no tuning is in band under it. Its key is the lexicographic `(input holder 2 / base holder 1 / neither 0, receiver's domain, nearer by XOR)` that §F's F9 orders it by. `DEFAULT_WEIGHTS` is gone, and the built-in Task set has empty weights. Its `cid` and `ranking_cid` therefore change, and no record holds the old ones yet. `assert_ranking_contract` now requires any method that declares factors to show, across its fixtures, that making a different factor dominant gives a different order. WeightedCapacity meets this requirement.
- **T10, in part: a person's household-only limit is reported as an operator veto (eligibility.rs).** `Kind { label }` refusals whose label is in the new `OWN_OFFER_LABELS` (`household-only`, `scope`, `isolation`) now map to `LimitOwner::SelfLimit`, with come-back `posture-changed`. These are the carrier's own posture, so none of them is an operator's line. Any other kind label stays `Operator`. Constants `HOUSEHOLD_ONLY`, `SCOPE` and `ISOLATION` name the labels. Test: `refusals_name_whose_line_they_honored` checks the three labels, an end-to-end household-only refusal, and that another label still maps to `Operator`.

### Refused

- **T10's new `IneligibleReason::HouseholdOnly` variant.** `views/placement-explanation-view.schema.json` lists every `IneligibleReason` as a closed `oneOf`, and every `Explanation.own_refusals` entry serializes one. A `household-only` variant would therefore emit refusals the view schema rejects. That schema is lane S's, outside this write-set. The capture-resistance defect is fixed through the owner mapping above. The variant is a follow-on that must land with its `oneOf` entry. Until then the label rides `Kind`.
- **T12: write the revisions into spec §6.** The spec file is outside this lane's write-set (`elohim/elohim-compute/**`, `elohim/placement/**`, `elohim/Cargo.toml`, `elohim/Cargo.lock`). The brief directs the revisions to this report: "record the revision in your report so the spec can absorb it". Every change the finding lists is already recorded in this report:

  | Change | Report entry |
  |---|---|
  | `admit` gains `receiver_domain` | §3 entry 4 |
  | Governor `Effect = Charge` | entry 27 |
  | `MethodRegistry::with -> Result` | entry 28 |
  | `from_head` and `select_in_force` take `methods` | entry 22 |
  | `RatificationCheck::verify(head, ..)` | entry 21 |
  | `input_root: CidString` | entry 3 |
  | `NeedSource::needs -> Answer`, `Snapshot.needs_read`, `Plan.unencodable`, `Carrying.in_scope_of` | entry 29 |
  | `Ports::evidence` | entry 8 |
  | `chosen()` | entry 9 |
  | `ManifestContext.closure_bytes` | entry 30, superseded by 36 below |
  | the local `cid_serde` | entry 1 |
  | the election tiers | entry 26, withdrawn by 32 below |

  Folding them into §6 and the §2.D/§2.E code blocks is the spec seat's job.

### Spec revisions for the spec seat (adds to §3)

32. **Withdraws entry 26.** The election has one tier with a proven clock: `ratified_at` for `Ratified`, `Charter.adopted_at` for `Charter`, and 0 for `BuiltIn`, which is never elected. E4's charter-amendment sentence holds as the spec states it. `Charter` gains `adopted_at: EpochSecs`.
33. A carrying holds its need only when its carrier is live (D3, D14). `Carrying.honored` alone does not hold it. A dead carrier is neither counted nor released.
34. Capacity: each offer is its own cap. A need is admitted against its own kind's offer, net of everything charged to the carrier in the pass, summed across kinds (`Charge::plus`). This replaces entry 10's widest-offer pool. A universal capacity stage before `fits` charges every required component, and refuses an unstated one as `Unknown { input: "capacity" }`. `fits` only narrows.
35. F6 is checked in core: `offer.scopes ∋ kind.scope(need)` and `offer.isolation >= need.requires.isolation` (unknown isolation fails), refused as `Kind { "scope" }` and `Kind { "isolation" }`. F1 is checked in core: a `NeedTerms::Task.scope` that differs from `kind.scope(need)` is refused as `Kind { "scope" }`. Added: `kind::testing::assert_scope_is_derived`. `Kind` labels `household-only`, `scope` and `isolation` map to `LimitOwner::SelfLimit`, which revises entries 5 and 18.
36. `ManifestContext { manifest, release_root, closure: Option<ClosureKey>, receiver, scope, reach }`, with `ClosureKey { key, root, paths, bytes }` bound to `release_root` and the declared paths. `class` is removed. `GrantRequest` gains `requester`, `in_scope_of`, `reach` and `manifest`. This supersedes entry 30's `closure_bytes`.
37. `RealEvidence::from_readings(subject, readings)`; `settle(..) -> Result<FulfilmentDraft, PlaceError>`, refusing evidence about another subject; `FulfilmentDraft.id` = `fulfilment_id(commitment, period)`; `Ports::settle` is idempotent by that id.
38. `NearestBytes` reads no weights (`factors() = []`). The built-in Task set's weights are empty, which revises entry 12. The ranking contract requires any method that declares factors to be reorderable by its weights.

### Concerns (new)

- `fulfilment_id` and `commitment_id` share `hex16`. The id pins (commitment, period.start) and ignores `len_secs`, so a space that changes its period length mid-period would draft one id for two lengths. No adapter exists yet.
- Evidence is pinned to the need key, not to the provider. `Reading` has no provider field. A real holding reading about a key does not show *which* carrier holds it, and core cannot close that gap with `Reading` as it stands. `quantity` is also still supplied beside the evidence rather than derived from it, and `observed_at` is an RFC 3339 string that core cannot compare with the period without a time dependency, which R-P1 excludes. The custody `Ports::evidence` adapter must supply all three.
- The view-schema follow-on for `IneligibleReason::HouseholdOnly`, under T10 above, belongs to lane S.
- The covenant floor was not needed this round. All edits are inside the brief's write-set, apart from appending to this report.

## Codex review 1 fixes

Codex review 1 raised twelve findings. I checked each one against the code. Findings 2 and 6 are the same defect, and so are 7 and 10. Ten distinct defects remain: nine are fixed, and finding 9 is refused because its fix lies outside this lane's write-set. Nothing is staged or committed.

**Gate evidence:** `just gate elohim-placement` → `EXIT=0`. Steps: `cargo fmt --check -p elohim-placement-core` EXIT=0; `cargo clippy -p elohim-placement-core --all-targets --all-features -- -D warnings` EXIT=0; `cargo test -p elohim-placement-core --all-features` EXIT=0 (67 unit, 15 ceiling, 27 floor, 3 schema, 12 doctests); `cargo test -p elohim-compute --no-default-features` EXIT=0 (21); `cargo clippy -p elohim-compute --no-default-features --all-targets -- -D warnings` EXIT=0. Two earlier gate runs this round failed `cargo fmt --check`, and one failed clippy on `redundant_guards` in the new module resolver. All were fixed before the passing run. One manual `cargo test` failed to link (`undefined symbol: __getrandom_v03_custom`) because it inherited the shell's WASM `RUSTFLAGS`; the gate declares `rustflags: ""`, and the manual runs after that used it. elohim-compute was not edited this round.

**Mutation check:** I reverted four fixes at once: the charging rule, the dedupe of honored holders, `floor_charge`, and the observe-only digest. The run (`--no-fail-fast`) gave EXIT=101, and exactly the five new tests failed: `a_kind_cannot_under_charge_or_over_charge_a_need`, `an_under_charging_kind_cannot_place_two_needs_on_one_offer`, `a_move_the_cap_withholds_charges_no_capacity`, `one_commitment_observed_twice_is_one_holding` (left 1, right 0: the sole holding was released) and `every_explanation_outcome_validates_against_the_view_schema` (digest length 0). The sources were then restored byte-identical (`cmp`). The rail fixes (findings 11 and 12) carry negative tests over fixture sources instead. The cargo berth was claimed with `berth claim cargo --class verify --ttl 3600` (EXIT=0) and released with `berth release cargo` (EXIT=0, "released by" this session).

### Fixed

- **C1: a kind can under-charge (eligibility.rs).** After `kind.fits`, the new `floor_charge` raises each component of the charge to at least `need.requires`. It then checks the result against `remaining`: a component past what is left is `OverCapacity`, and a nonzero component the offer does not state is `Unknown { input: "capacity" }`. `admit` and the `Eligibility` face's `render` both use it. Tests: `a_kind_cannot_under_charge_or_over_charge_a_need` (the lenient kind is charged 10 by both faces, and over-charges and unstated charges are refused) and `an_under_charging_kind_cannot_place_two_needs_on_one_offer` (a decide-level run with the lenient kind: two 100-byte needs on one 100-byte offer give one adoption and one unserved).
- **C2 = C6: capacity is charged for adoptions the move cap withheld (decide.rs).** The charging loop now charges a chosen carrier only when this pass plans its adoption, or when it already has a pending commitment for the need (`has_live && !is_honored`, the retained reservation). An honored holder is still not charged. Each carrier is charged once per need. Test `a_move_the_cap_withholds_charges_no_capacity`: `big` (25 bytes) is preferred under a spare-heavy set with a cap of 1. Two needs are held elsewhere under the old set, and a third, unheld need comes last in serve order. Only `big` can hold the third need, and it does once the capped move takes nothing. Before the fix it was `NoPriorityChosen`.
- **C3: observe-only explanations have an empty digest (explain.rs, decide.rs).** `inputs_digest` is now `observed_digest(need, in_force)` = `digest_of(need, in_force, [])` on every path: the need and the set in force, with no ranked order. `digest_matches_commitment` stays `None` there, because there is no ranking to compare. The schema check asked for is now a test: `every_explanation_outcome_validates_against_the_view_schema` validates Held, NotCarried (with an own refusal), Carried, observe-only (no charter), a constructed InputsGone, and a LastVerified explanation against `views/placement-explanation-view.schema.json` with jsonschema. A retriever over the in-tree schema files resolves its `$ref`s. The test also asserts that the validator rejects an empty digest. Validation exposed one more gap, so `Explanation` gains `parameter_set: CidString` (`parameterSet`). The schema already required that field, and its description noted the Rust mirror lacked it (the "follow-on, with the spec seat" it named). jsonschema 0.28 (`default-features = false`) is a new dev-dependency. It was already in `elohim/Cargo.lock` through elohim-epr; the lock adds only the edge.
- **C4: `LastVerified` is never reported (params.rs, explain.rs).** `InForce` gains a private `fallback: Option<ParamFallback>` and a `fallback()` accessor. `select_in_force` sets it to `LastVerified` when the retained set answers. A retained built-in stays `BuiltIn`, as its ratification says. An elected head and `InForce::builtin` carry `None`. `explain` reports `in_force.fallback()` first, then `BuiltIn` as before. Tests: e3 and e4 now assert `fallback() == Some(LastVerified)` where the retained set answers. Because derived equality now includes provenance, they compare cid and ratification rather than the whole value. The schema test asserts `fallback: last-verified` end to end.
- **C5: duplicate observations release the sole holder (decide.rs).** Honored rows are now kept one per normalized carrier. Where a carrier has several rows, the row under the set in force is kept, so a duplicate under an older set cannot invent a move. Ties go to the smaller commitment id. The counts and the drain simulation therefore see one holding per carrier. If a carrier holds two distinct commitments (a legacy id and its own), only one of them is released in a pass, so the counts never claim fewer holdings than remain. The other drains on a later pass. Test `one_commitment_observed_twice_is_one_holding`: with demand `{1,1}` there is no release, and with `{2,1}` a second holder is adopted.
- **C7 = C10: the pressure check can be bypassed (ceiling.rs).** `Pressure` is now `Serialize` only and has no public constructor. `Draft`'s fields are private: there is `Draft::new(proposed, against, scope, reach, drafted_by, lineage, touches)`, accessors, `press_count`, and `press_own_need(kind, &Need)`, which checks E11 against that draft's own `drafted_by` and `reach`. Neither can change after a key is admitted. A received draft deserializes into `DraftInput` / `PressureInput`, which have public fields and no checks. `DraftInput::validate(own_needs)` admits a keyed pressure only for a need in `own_needs` that has that key, is the drafter's own, is at least as wide as the draft's reach, and has count 1. `TuningSink::propose` still takes `&Draft`, so only a validated draft is published. Tests: `pressures_name_only_the_drafters_own_need` (rewritten onto `Draft`) and `a_received_draft_is_checked_before_it_carries_a_key` cover a wire round trip back to the same draft, and refusal of widening to public, another author, an absent need and an inflated count. Two `compile_fail` doctests cover `serde_json::from_str::<Pressure>` and `d.reach = Public`. A compiling doctest deserializes a `PressureInput`.
- **C8: the ranking scratch grows without bound (rank.rs).** `Scratch::prepare` now retains only the agents of the current snapshot and keeps the digests of those that stay. Test `the_scratch_holds_only_the_current_snapshot`: across 200 disjoint snapshots of 8 agents plus one that stays, the scratch holds 9 entries, the agent that stays keeps its digest, and an old agent is dropped.
- **C11: R-P7 checks only rank.rs, and only for text (lib.rs, kind.rs).** The new `code_only` strips comments and empties strings and char literals; it handles nested block comments and raw and byte strings. `ranking_contract_gaps` finds every `impl Ranking for X` in every scanned source. Each must have an `assert_ranking_contract(&X)` or `(&X(` call inside an active `#[test]` body anywhere in the crate. A test whose attributes include `ignore` or any `cfg(` does not count. Any `cfg(any())`, `cfg(not(test))`, `cfg(FALSE)` or `cfg(false)` in the crate is a gap. The crate-wide scan found kind.rs's `Claims` test double, which never ran the contract. It now does, in `the_adapter_double_runs_the_contract`. Negative test `the_ranking_rail_fails_without_an_executable_contract_call` fails the rail on each of these: a missing call, a line comment, a block comment, a call inside a string, a call outside a test, `#[ignore]` placed before or after `#[test]`, a call for another type, and a cfg-gated module. It also passes the case where the implementation is in one file and its test in another.
- **C12: nested modules escape the purity scans (lib.rs).** `unscanned_modules` reads every scanned source, not only lib.rs. It resolves each `mod x;`, `pub mod x;` and `pub(crate) mod x;` the way rustc does (`lib.rs` and `mod.rs` declare siblings; `foo.rs` declares `foo/x.rs` or `foo/x/mod.rs`) and requires the resolved file to be in `SOURCES`. It refuses `#[path]` and `include!` outright, because the resolution cannot follow them. Negative test `the_module_scan_follows_nested_declarations` covers an unlisted `a/child.rs`, a listed one, a commented-out declaration, `#[path]`, `include!`, and literals that hide nothing.

### Refused

- **C9: the local `cid_serde` copy.** The finding holds, and it is the defect this report has carried since §3 entry 1. The fix is to export the helper from epr-rea and remove this copy. epr-rea is `elohim/epr-rea/src/index.rs`, outside this lane's write-set (`elohim/elohim-compute/**`, `elohim/placement/**`, `elohim/Cargo.toml`, `elohim/Cargo.lock`), so the export cannot land from here. Until it does, `matches_epr_rea_encoding_in_both_formats` pins the copy byte-identical in JSON and DAG-CBOR. The finding is right that this proves equal output, not a shared implementation. The follow-on is unchanged: make the epr-rea module `pub` (or re-export it), switch the two `#[serde(with)]` paths, and remove `src/cid_serde.rs` and its entry in `SOURCES`. It needs a seat whose write-set includes `elohim/epr-rea/**`.

### Spec revisions for the spec seat (adds to §3)

39. Charge floor: the charge a pass deducts is `max(kind.fits(..), need.requires)` per component, checked against `remaining` (`OverCapacity`, or `Unknown { "capacity" }` for an unstated component). This adds to entry 34: `fits` narrows the admission and can only raise the charge.
40. Charging (§D.7): a pass charges planned adoptions and the chosen carriers that have a pending commitment for the need, once per carrier per need. A chosen carrier that the move cap kept from adopting is not charged. This revises entry 34's "charged ... in serve order" for chosen carriers.
41. Holding count (§D.1, D.7): honored holders are counted once per normalized carrier, preferring the row under the set in force. A carrier with two distinct commitments releases one per pass.
42. `Explanation.inputs_digest` on the observe-only paths is `digest_of(need, in_force, [])`, never empty. `Explanation` gains `parameter_set: CidString` (the view schema's required `parameterSet`). Explanations are now validated against the view schema in `floor_contract.rs`, which closes that schema's "not yet pinned to it by a test" follow-on. The schema's description still names both follow-ons and should be revised by lane S.
43. `InForce::fallback() -> Option<ParamFallback>`. `select_in_force` marks a retained non-built-in set `LastVerified`, and `explain` reports that ahead of `BuiltIn`. Derived `PartialEq` on `InForce` now includes provenance. This closes entry 15's "`LastVerified` is never reported".
44. §E drafts: `Draft` has private fields, plus `Draft::new`, `press_count`, `press_own_need` and accessors. `Pressure` is `Serialize` only. `DraftInput` and `PressureInput` are the `Deserialize` forms, and `DraftInput::validate(own_needs) -> Result<Draft, PlaceError>` is the only way from the wire to a publishable draft. `Pressure::count` and `Pressure::own_need` are removed.
45. R-P7 applies across the crate and to executable code only (above). The `Scratch` cache holds the current snapshot's carriers only.

### Concerns (new)

- `floor_charge` refuses a nonzero charge in a component the offer leaves unstated. No kind in the tree charges outside `need.requires` today. A future kind that does must declare the component on its offers.
- R-P7's notion of "active" code is textual. It does not evaluate arbitrary `cfg` predicates. It refuses the four disabling forms by name, and it refuses any `cfg(` on a test that carries the contract. A disabling predicate spelled another way (for example `cfg(all(test, not(test)))`) would pass.
- The explanation view schema's description still says the Rust mirror lacks `parameterSet` and is not pinned by a test. Both statements are now false, and the schema file belongs to lane S.
- The covenant floor was not needed this round. All edits are inside the brief's write-set, apart from appending to this report.

## Codex review 2 fixes

Codex review 2 raised twelve findings. I checked each one against the code. Findings 1 and 5 are the same defect, and so are 2 and 7, and 4 and 12. Nine distinct defects remain: eight are fixed, and the `cid_serde` copy (4 = 12) is refused again because its fix lies outside this lane's write-set. Nothing is staged or committed.

**Gate evidence:** `just gate elohim-placement` → `EXIT=0`. Steps: `cargo fmt --check -p elohim-placement-core` EXIT=0; `cargo clippy -p elohim-placement-core --all-targets --all-features -- -D warnings` EXIT=0; `cargo test -p elohim-placement-core --all-features` EXIT=0 (69 unit, 15 ceiling, 31 floor, 3 schema, 12 doctests); `cargo test -p elohim-compute --no-default-features` EXIT=0 (21); `cargo clippy -p elohim-compute --no-default-features --all-targets -- -D warnings` EXIT=0. The first gate run this round also passed, and the final run above came after the test file's last edit. elohim-compute was not edited this round. The cargo berth was claimed with `berth claim cargo --class verify --ttl 3600` (EXIT=0, "renewed by" this session). `berth release cargo` returned EXIT=0 with "was not held by" this session, because the gate runner's own claim and release had already ended the lease. The first gate attempt was refused before launch by the one-build-at-a-time hook, because another session's `cargo test --workspace` was running. I waited for it to finish and did not touch it.

**Mutation check:** six mutations were each applied alone, run with `cargo test -p elohim-placement-core --all-features --no-fail-fast` (pool target dir, `RUSTFLAGS=""`), and restored. Each failed with EXIT=101, and exactly the intended test failed:

| Mutation | Failing test |
|---|---|
| No pending reservation before allocation | `a_pending_commitment_keeps_its_reservation_out_of_the_prefix_and_against_earlier_classes` |
| Each expression charged on its own (no reservation reuse) | `two_expressions_of_one_commitment_charge_its_carrier_once` |
| Shortage read from the chosen prefix again | `an_honored_need_is_served_though_its_holder_has_no_spare` |
| No holding credit on retention | `an_honored_need_is_served_though_its_holder_has_no_spare` |
| DiversityAware diversifying across the incumbency tier | `rank::tests::every_builtin_ranking_runs_the_contract` |
| `InForce::binds` ignoring scope | `a_set_verified_for_one_scope_is_never_anothers_authority` |

`cmp` against a backup confirmed that `decide.rs` and `rank.rs` were restored byte-identical. The final gate above ran on the restored sources. The settlement and R-P1 fixes carry negative tests over fixture inputs instead.

### Fixed

- **R2c-1 = R2c-5: a pending commitment loses its reservation outside the chosen prefix (decide.rs).** `run_pass` now reserves every outstanding pending commitment before it allocates any need. A pending commitment is a carrying whose answer is not `Absent` and whose carrier is not an honored, live holder of that identity. The reservation is keyed by (carrier, commitment identity), so each commitment is reserved once. Its size is the charge floor (`Charge::of_requires`) of the widest expression of its need. When a need is evaluated, each carrier is measured net of everything charged to it except that commitment's own reservation. If the pending carrier is chosen, the reservation rises to the admitted charge, and it is never added a second time. A pending carrier that has left the prefix keeps its reservation, and a need of an earlier class cannot take that capacity. A carrying whose need is not expressed reserves nothing, because it is released `NeedWithdrawn` or held unread. Test `a_pending_commitment_keeps_its_reservation_out_of_the_prefix_and_against_earlier_classes` has two legs. In the first, a spare-heavy set moves `x`'s prefix to `b`. In the second, under the built-in set, `y` is `Care`, so it is served before `x`. In both legs `y`, which names only `a`, is refused by name and never adopted beside `a`'s pending 10 bytes.
- **R2c-6: two expressions of one commitment charge its carrier twice (decide.rs).** The same reservation map fixes this. Charging sets that commitment's reservation to `max(was, charge)` and adjusts the carrier's total by the difference. `charged` is therefore always the sum of a carrier's reservations, and a second expression that reaches the same carrier with the same identity adds nothing. Test `two_expressions_of_one_commitment_charge_its_carrier_once`: the same key and receiver are expressed at `Public` and at `Commons`, beside a third need, on one 25-byte carrier. All three are placed, with one draft for the shared commitment, and nothing is unserved. Before the fix, one need was refused against phantom consumption.
- **R2c-2 = R2c-7: a fully honored need is reported unserved (decide.rs).** There are two parts. First, retaining a holding is distinguished from adopting a new copy. An honored, live holder is admitted against its remaining capacity with its holding's own footprint given back (`Capacity::credited`), because a posture's spare already nets out what the carrier holds. Second, unmet demand is read from `planned`: the honored holdings, whether or not they are still chosen, together with the chosen carriers that adopt or are pending. The prefix alone no longer decides it. `NoEligible`, `ServedFirst` and `NoPriorityChosen` now mean `planned` is short of holders, and `DomainShort { have }` counts `planned`'s domains. Test `an_honored_need_is_served_though_its_holder_has_no_spare`: a 100% full holder of a 10-byte need gives no unserved entry, no adoption, no release, and an explanation of `Carried`, with the holder still chosen and no own refusal. A variant whose `requires.carriers` excludes the holder is also `Carried`.
- **R2c-3: the pass is quadratic in carryings (decide.rs).** `run_pass` builds `carrying_by: identity → carryings` and `expressed: BTreeSet<identity>` once per pass. `rank_need` receives its own identity's slice, and withdrawal checks the set. Neither path scans every carrying per need or every need per carrying any more. `same_commitment` is replaced by `Ident = (KindId, &NeedKey, &str receiver)`.
- **R2c-8: settlement accepts readings that prove nothing (adopt.rs, floor.rs).** `RealEvidence::from_readings(carrying, period, readings)` requires every reading to meet all of these conditions:
  - `EvidenceStage::Real`;
  - of `fulfilment_promise(carrying.kind)`;
  - about `carrying.key.as_id_string()`;
  - with a `ProbeAnswer::Present(v)` answer, so `Absent` and `Unreachable` are refused;
  - `observed_at` inside `[period.start, period.start + len_secs)`.

  The evidence keeps the commitment, period, promise, subject and `supported` (the least present value) values. `settle` refuses evidence for another commitment, evidence for another period, and a quantity above `supported`. `observed_at` is parsed by a small arithmetic RFC 3339 reader (`rfc3339_epoch`). It covers offsets, fractional seconds, and range and leap-day checks, and it adds no time crate (R-P1) and no clock (R-P2). `T: Into<u64>` is required of the reading's value. Tests: `only_real_present_readings_of_the_fulfilment_promise_settle` covers a fixture reading, absent, unreachable, another promise on the same subject, another subject, before the period, at its end, years later, a non-timestamp, one bad reading in a good set, and a kind with no fulfilment promise. `evidence_settles_only_its_own_commitment_period_and_quantity` covers another commitment's evidence, a quantity above what is supported, the last period's evidence offered for the next, and the least reading bounding the quantity. `rfc3339_reads_by_arithmetic` adds pinned epochs and ten malformed inputs. The floor fixture's `Ports::evidence` now builds its reading inside the period it is asked about.
- **R2c-9: a verified set loses its scope (params.rs, decide.rs).** `InForce` gains a private `scope: Option<Cid>`. It is `Some(scope)` for every set `from_head` verifies and `None` for the built-in, which binds every scope. The new `InForce::binds(scope, kind)` is true when the set has that kind and is either the built-in or verified for exactly that scope. `select_in_force` retains `last_verified` only where it binds, and `decide` consumes a `Snapshot.params` entry only where it binds the need's scope and kind. Otherwise the built-in answers. Test `a_set_verified_for_one_scope_is_never_anothers_authority`: A's set is retained for A, and B gets the built-in. A `Snapshot.params` entry pairing B with A's set, or with another kind's set, is not consumed: every adoption carries the built-in's CID. B's own verified set is consumed.
- **R2c-10: DiversityAware crosses the incumbency tier (rank.rs).** It now diversifies the incumbents and the newcomers separately and concatenates the two lists. `assert_ranking_contract`'s fixture now has two incumbents in one failure domain (`i == 1 || i == 4`, domain `i % 3`). It asserts that every incumbent ranks ahead of every non-incumbent and that both incumbents lead. The storage-strategy vectors carry no incumbents, so they are unchanged. No record has been adopted under `elohim:placement:diversity-aware@1`, because the crate has not landed. The fix therefore keeps `@1` rather than minting `@2`, and the governance note's rule ("a new ranking is a new MethodId") applies from landing on.
- **R2c-11: R-P1's compute-feature check is a substring test (lib.rs).** `compute_feature_violations(toml)` replaces the substring check. It refuses each of these:
  - workspace inheritance for `elohim-compute`, whose defaults this file cannot see;
  - any `default-features` value other than `false`;
  - a non-empty `features` list;
  - any `elohim-compute/…` or `elohim-compute?/…` forwarding in placement's own `[features]` table.

  It reads values through a small spec reader (`spec_value`), not by substring. Test `the_compute_feature_rail_reads_features_not_substrings` covers the finding's fixture (`default-features = false, features = ["runtime"]`), no `default-features`, `default-features = true`, `workspace = true`, the two forwarding forms, `elohim-compute/default`, the dotted-table form with and without `runtime`, and two clean manifests.

### Refused

- **R2c-4 = R2c-12: the local `cid_serde` copy.** The finding holds; it is the defect this report has carried since §3 entry 1, refused in Codex review 1 as C9. I re-checked the code this round: `elohim/epr-rea/src/index.rs:72` still reads `mod cid_serde {`, so the helper is private. The export lands in epr-rea, which is outside this lane's write-set (`elohim/elohim-compute/**`, `elohim/placement/**`, `elohim/Cargo.toml`, `elohim/Cargo.lock`). Re-exporting it from here is not possible, and a second copy under another name would only rename the defect. This seat cannot reach the owning lane: it has no write-set there, and the brief names no epr-rea seat. The coordination therefore goes to the orchestrator. The follow-on is unchanged:
  1. A seat with `elohim/epr-rea/**` in its write-set makes the module `pub` or re-exports it.
  2. That seat or this one, after the export lands, switches placement's two `#[serde(with)]` paths.
  3. It deletes `src/cid_serde.rs` and its `SOURCES` row, and drops the parity test.

  Until then, `matches_epr_rea_encoding_in_both_formats` pins the copy byte-identical. As both reviews say, that proves equal output, not one implementation. **This blocks landing as specified (§1.4)**, and it is recorded as a concern below.

### Spec revisions for the spec seat (adds to §3)

46. Capacity reservations (§D.7): a pass keeps one reservation per (carrier, commitment identity), and `charged` is their sum. Before any need is allocated, each outstanding pending commitment reserves the charge floor of its need's widest expression. A carrier is admitted for a need net of everything except that need's own reservation. A chosen carrier that adopts or is pending raises the reservation to the admitted charge, never adding it twice. This revises entry 40. A pending carrier that leaves the prefix keeps its reservation, and two expressions of one commitment share one reservation.
47. Retention (§D.1, D.7): an honored, live holder is admitted with its holding's own footprint given back (`Capacity::credited`). Shortage is read from `planned`, the honored holdings plus the chosen carriers that adopt or are pending, never from the chosen prefix alone. `DomainShort.have` counts `planned`'s domains. This revises entry 17's `Carried`/`Held` reading only in its inputs.
48. §4's bound: carryings are indexed by commitment identity and expressed identities are a set, both built once per pass.
49. §D.6 settlement: `fulfilment_promise(kind) -> Option<&'static str>` (custody: `holding-is-honored@1`; no other kind is named yet, a placeholder open in §6). `RealEvidence::from_readings(carrying, period, readings)` with `T: Serialize + Copy + Into<u64>` requires `Real`, the kind's promise, the carrying's subject, a `Present` answer, and `observed_at` (RFC 3339) inside the period. `settle` refuses another commitment or period and a quantity above `RealEvidence::supported()`. This supersedes entry 37's `from_readings(subject, readings)`. Accessors `commitment`, `period`, `promise` and `supported` are additive.
50. §E: `InForce` carries the scope it was verified for (`None` for the built-in). `InForce::binds(scope, kind)` and `InForce::scope()` are additive. `select_in_force` retains, and `decide` consumes, only a set that binds.
51. The ranking contract's incumbents-first clause is tested with several incumbents sharing a domain, and every incumbent must precede every non-incumbent. `DiversityAware` diversifies within each incumbency tier.
52. R-P1 checks elohim-compute's feature declarations, as above, not a substring.

### Concerns (new)

- **The `cid_serde` copy blocks landing as §1.4 specifies.** The epr-rea export needs a seat that owns `elohim/epr-rea/**`. The orchestrator must dispatch one before this lane lands, or the spec seat must record the copy as an accepted interim variance.
- `fulfilment_promise` names a promise only for the custody kinds. Relay, live-session, wake-ingress, hub-runtime, inference and index-fold commitments therefore settle through no reading until their promises are named. The task kind settles on its completion event (F11), not here. No adapter exists yet, so running behaviour is unchanged.
- `Reading` still carries no provider. Settlement evidence is bound to the commitment, whose id hashes the provider, but no reading proves *which* carrier held the bytes. That remains the custody `Ports::evidence` adapter's obligation, as stated in the third review's concerns.
- A pending reservation is sized at the charge floor until its own need's evaluation raises it. A kind whose `fits` charges more than `requires` is under-reserved, only in the same pass and only for needs served before it.
- The covenant floor was not needed this round. All code edits are inside the brief's write-set, apart from appending to this report.

## Codex integration addendum — 2026-10-21

Actor: `agent:implementer@codex`. This append preserves the original seat and review history above. The orchestrator extended this integration seat's scope to the existing EPR-REA CID serializer and the k8s consumer lock. No files were staged or committed. Claim for gap `#25` was recorded once, with this brief and `materialize-what-is-read`, `EXIT=0`.

### Current revision summary

- The duplicate CID serializer landing blocker is closed. `elohim-epr-rea::index::cid_serde` is now public; placement's need, parameter and ranking serialization use that one implementation. The placement-local copy, its module declaration, source-rail entry and parity-only test were removed. No second encoding implementation remains.
- `RealEvidence::from_readings` consumes `Reading` through immutable getters. C1 now keeps all seven reading fields private, so callers cannot promote fixture evidence or rewrite the answer, subject, timestamp or disclosure by field mutation or struct update.
- This supersedes the final pending-reservation concern above: before allocating any need, a pending commitment reserves the maximum of its declared requirements and its registered kind's trusted `fits` charge over every expression. If the trusted charge cannot be established, its known offered capacity is held conservatively. Reservations remain once per normalized carrier and commitment identity; the need receives its own reservation back during admission.
- The cross-crate regression is Ark's `a_pending_task_reserves_its_trusted_view_before_earlier_classes`. A real `TaskKind` has a 4,096-byte measured view, a pending declaration lowered to one byte and a 4,097-byte berth. A higher-priority task cannot consume the pending task's space. Both input orders and unknown pending-view measurement pass.
- The reported `InForce::binds` analyzer error was no longer present in source at handoff: the scope-binding implementation already existed. The fresh placement gate compiles and tests it successfully.

### Actual integration evidence

The root orchestrator ran the declared native gates serially, with the cargo berth and pinned Rust 1.96.1. Logs are durable under `genesis/local-dev/placement-codex-handoff/`.

- **Gate evidence:** `just gate elohim-placement` → `EXIT=0` (`elohim-placement-gate-final.log`): fmt and all-feature/all-target Clippy passed; 68 unit, 15 ceiling, 31 floor, 3 schema and 12 doctests passed. The same gate ran 21 `elohim-compute --no-default-features` tests and its all-target Clippy, both `EXIT=0`.
- **Gate evidence:** `just gate elohim-compute` → `EXIT=0` (`elohim-compute-gate-final.log`): 50 default-feature tests passed.
- **Gate evidence:** `just gate elohim-epr` → `EXIT=0` (`elohim-epr-gate-final.log`): workspace format, EPR/REA/index Clippy and 355 tests passed, covering the shared serializer's owner.
- **Gate evidence:** `just gate elohim-ark` → `EXIT=0` (`elohim-ark-gate-final.log`): the real TaskKind reservation regression passed among 155 unit/integration tests and 3 doctests.
- Existing runtime consumer `elohim-storage` passed its final declared gate, `EXIT=0` (`c5-gate-3.log`), after the final source edits. The doorway consumer gate remains pending at this append; completion evidence will be appended before this seat's terminal fulfilment.

The shared eprfs gate initially failed one existing identity-migration steward assertion. The unchanged test then passed one focused run, 20 bounded repetitions and the complete declared gate (`eprfs-gate-final.log`, `EXIT=0`, 1,326 tests). The initial failure remains recorded as intermittent and unexplained; no source change is claimed to fix it.

### Scope still open

This is architecture and fixture evidence. Production Floor adapters, carrier-bound custody evidence, the remaining kinds' fulfilment promises and the task runner/materialization/completion path remain follow-on work. `Reading` itself does not identify a provider. Existing reach checks use openness plus the scoped household rules; this pass supplies no new collective set-inclusion semantics. The household mesh stayed stopped, and no live convergence, task execution or materialization is claimed. Scoped commits: none.

### Consumer verification complete — 2026-10-21

**Status: `DONE_WITH_CONCERNS`.** The doorway verification pending in the integration addendum is complete. Both existing runtime consumers now pass their actual declared gates after the default-on compute feature split. The remaining concerns are the named runtime follow-ons and the retained, unexplained intermittent eprfs test failure; no C2 code or gate blocker remains.

**Gate evidence:** `just gate doorway` → `EXIT=0` (`doorway-gate-final.log`): format, Clippy with warnings denied, and `cargo test --lib --bins` passed. The library passed 1,831 tests with 2 ignored, and the main binary passed 4 tests; no test failed. The gate rebuilt an unmarked pool slot and recorded an 11-minute soft cycle-ceiling observation while returning success. `cargo metadata --manifest-path doorway/doorway-service/Cargo.toml --format-version 1 --locked --offline` → `EXIT=0` (`doorway-metadata-locked-final.log`), with the declared native pool and Rust 1.96.1. Doorway's lock did not change.

The additional k8s consumer also passed `just gate k8s-bridge` → `EXIT=0` (26 tests), followed by locked offline metadata `EXIT=0`; its Cargo-generated lock drops the compute runtime dependencies while admitting the new pure placement/eprfs graph. The C3 report carries the detailed lock review. Scoped commits: none; all changes remain in the shared working tree for review.
