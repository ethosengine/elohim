/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: views/placement-explanation-view.schema.json -- DO NOT EDIT */

/**
 * The key a placement need is ranked and carried under: adjacently tagged, `form` (kebab-case) beside `id`. `blob` is the legacy custody marker (`sha256-<hex>`) until the custody rows migrate, ranked over the marker's UTF-8 bytes so the XOR key is byte-identical to elohim-storage's reconcile/placement.rs hash_into_keyspace; `tree-view` is objects/tree-view's CID (R1: per tree root and view, never per blob); `task` is a TaskSpec's CID (§F); `session` a live session's CID. Every CID is a string in JSON (cid_serde's human-readable branch, G1). Lives in objects/ rather than inside intents/placement-need because two schemas carry it (the need and the explanation view) and the TypeScript codegen resolves cross-file references into objects/ but not into intents/ (scripts/codegen-ts.mjs loadRefMap). Wire shape: this schema is the source of record; placement_core::NeedKey (elohim/placement/core/src/need.rs) is its Rust mirror, not yet pinned to it by a test (follow-on: a placement-core schema_contract case that serializes each form and validates it here). Source of truth: the thing it keys, from which it is derived (Category C): a key, content-derived except the legacy blob marker.
 */
export type NeedKey = NeedKeyBlob | NeedKeyTreeView | NeedKeyTask | NeedKeySession;
/**
 * The closed list of placement kinds: what a need asks to have carried and what a posture offers to carry. One deterministic `decide` pass covers every kind, so a carrier's capacity is charged across kinds in the charter's serve order (interfaces spec §D.1). A new kind is four coupled steps landed together: a variant here and in `placement_core::KindId`, a `PlacementKind` implementation in the crate that owns the carrier, a `grant-scope` entry if it spends compute, and a probe (§D.1 'Extension'). Closed rather than open because the compute-grant scopes a kind consumes are a closed list. Source of truth: this schema; Rust mirror `placement_core::KindId` (elohim/placement/core/src/kind.rs, `#[non_exhaustive]`, kebab-case serde). TypeScript constants only: `_codegen` (no `rust` key, so codegen-rs writes nothing) and never `_dna`, so no constant reaches content_store_integrity and no DNA hash moves (interfaces spec §2.G, G11). placement-core's schema_contract test reads this file with include_str! and compares it, in order, with `KindId::ALL`, a list kept by hand in kind.rs. It catches a value changed here, but a variant added to `KindId` without extending `ALL` still passes: no exhaustive match ties `ALL` to the enum, and the enum is `#[non_exhaustive]` (follow-on: a guard in kind.rs that matches every variant against its index in `ALL`).
 */
export type PlacementKind =
  | 'custody-blob'
  | 'custody-root'
  | 'live-session'
  | 'relay'
  | 'wake-ingress'
  | 'hub-runtime'
  | 'inference'
  | 'index-fold'
  | 'task';
/**
 * How a placement parameter set came to be in force (interfaces spec §E). `built-in`: the floor's own content-addressed set for a kind, shipped with the floor; nobody put it in force, it runs when nothing has been, explanations say 'no tuning chosen', and it is exempt from ratification. `charter`: a set the space's charter itself names (`Charter::genesis_params`); the charter's adoption ratified it. `ratified`: a tuning ratified under the charter's tuning rule (the household default is each touched member's consent). The floor checks `built-in` and `charter` itself; only `ratified` is handed to the adapter's `RatificationCheck` (E9). Source of truth: this schema; Rust mirror `placement_core::RatificationKind` (params.rs). Not named in §2.G's enum list; added because objects/placement-parameter-head's `ratification.by` needs a referenced vocabulary (CONVENTIONS rule 7). Carries neither `_dna` nor `_codegen`: it is a field vocabulary for the placement wire schemas, generated only as a TypeScript union where those schemas inline it, and adds no constant to schema-enums.ts. Not yet pinned: no test holds the Rust enum equal to this list (follow-on: a case in placement-core's schema_contract test).
 */
export type RatificationKind = 'built-in' | 'charter' | 'ratified';
/**
 * Which parameter set an explanation reports as answering when the declared placement parameter head could not be verified (interfaces spec §E, E10): `built-in` (the floor's own set for the kind) or `last-verified` (the last set this floor verified for the scope and kind). Source of truth: this schema; Rust mirror placement_core::ParamFallback (elohim/placement/core/src/params.rs, kebab-case serde). Not named in §2.G's enum list; added because views/placement-explanation-view's `fallback` needs a referenced vocabulary (CONVENTIONS rule 7). Carries neither `_dna` nor `_codegen`: it is a field vocabulary for the placement wire schemas, generated only as a TypeScript union where those schemas inline it, and adds no constant to schema-enums.ts. Not yet pinned: no test holds the Rust enum equal to this list (follow-on: a case in placement-core's schema_contract test).
 */
export type ParamFallback = 'built-in' | 'last-verified';
export type UnansweredIneligible = Unanswered & {
  why: IneligibleReason;
};
/**
 * When the answer can change: a time a tier or gate gave (`at`), or an event after which it can change (`on-event`, e.g. tier-reachable, posture-changed, parameters-declared, berth-offered). The core never reads it from a local clock: the time is supplied by whoever gave it.
 */
export type ComeBack = ComeBackAt | ComeBackOnEvent;
/**
 * What the asker can do locally while the request is unanswered. `nothing` must say why, so 'nothing' is a statement and never a gap.
 */
export type Meanwhile =
  | MeanwhileUseHeld
  | MeanwhileRunHere
  | MeanwhileBuiltInInForce
  | MeanwhileNothing;
/**
 * Why a candidate could not carry a need, internally tagged by `reason`. Eligibility takes no parameters, so no parameter set can reach any of these (D contract 8). `unknown` names a fact the floor does not hold, which is never read as a default (spec §7.2).
 */
export type IneligibleReason =
  | IneligibleNoPosture
  | IneligibleUnknown
  | IneligibleOutsideReach
  | IneligibleOverCapacity
  | IneligibleNoConsent
  | IneligibleConsentExpired
  | IneligibleIsReceiver
  | IneligibleNotNamed
  | IneligibleCrossHouseholdBelowPublic
  | IneligibleKind;
/**
 * Content reach/visibility level. Ordered from most restrictive to most open. Source of truth: DNA-notarized CORE_REACH_LEVELS constant in content_store_integrity zome. Category A — enumeration values are part of the protocol vocabulary enforced by gateways without parsing payload.
 */
export type Reach =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';
/**
 * Content reach/visibility level. Ordered from most restrictive to most open. Source of truth: DNA-notarized CORE_REACH_LEVELS constant in content_store_integrity zome. Category A — enumeration values are part of the protocol vocabulary enforced by gateways without parsing payload.
 */
export type Reach1 =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';
/**
 * What happened to the need, internally tagged by `outcome`: carried by the asker, held by others, not carried (with R7's three parts), or the inputs are gone, so only the commitment's method, parameter-set CID and inputs digest remain (spec §7.4). Inputs gone has one form only: outcome `inputs-gone` with reason `inputs-gone`; outcome `not-carried` refuses that reason, and outcome `inputs-gone` refuses every other. The Rust mirror's types admit both forms (Outcome::NotCarried can hold NotCarriedReason::InputsGone, and Outcome::InputsGone any reason); placement_core::explain never builds either today, and narrowing the Rust types is a named follow-on.
 */
export type Outcome = OutcomeCarried | OutcomeHeld | OutcomeNotCarried | OutcomeInputsGone;
export type UnansweredNotCarried = Unanswered1 & {
  /**
   * Any NotCarriedReason except `inputs-gone`, which has its own outcome.
   */
  why: NotCarriedReason & {
    [k: string]: unknown;
  };
};
/**
 * When the answer can change: a time a tier or gate gave (`at`), or an event after which it can change (`on-event`, e.g. tier-reachable, posture-changed, parameters-declared, berth-offered). The core never reads it from a local clock: the time is supplied by whoever gave it.
 */
export type ComeBack1 = ComeBackAt1 | ComeBackOnEvent1;
/**
 * What the asker can do locally while the request is unanswered. `nothing` must say why, so 'nothing' is a statement and never a gap.
 */
export type Meanwhile1 =
  | MeanwhileUseHeld1
  | MeanwhileRunHere1
  | MeanwhileBuiltInInForce1
  | MeanwhileNothing1;
/**
 * Why a need is not carried, internally tagged by `reason`. An unmet need is named, never dropped. This is the whole vocabulary (placement_core::NotCarriedReason); `inputs-gone` is written in an explanation only as outcome `inputs-gone`, never under outcome `not-carried` (see UnansweredNotCarried and UnansweredInputsGone).
 */
export type NotCarriedReason =
  | NotCarriedNoEligible
  | NotCarriedDomainShort
  | NotCarriedServedFirst
  | NotCarriedNoPriorityChosen
  | NotCarriedMethodNotRun
  | NotCarriedInputsGone;
/**
 * The serve-order classes a space's charter ranks needs by when capacity is scarce: a need is taken in (charter serve-order position of its class, need id) order within one pass (interfaces spec §D.2, §D.7). The order listed here is the vocabulary's, not a serve order: the charter declares the order, and an empty `serve_order` is the built-in order, said so in the explanation. Source of truth: this schema; Rust mirror `placement_core::NeedClass` (need.rs, `#[non_exhaustive]`). TypeScript constants only: `_codegen` (no `rust` key, so codegen-rs writes nothing) and never `_dna`, so no constant reaches content_store_integrity and no DNA hash moves (interfaces spec §2.G, G11). placement-core's schema_contract test reads this file with include_str! and compares it, in order, with `NeedClass::BUILTIN_ORDER`, a list kept by hand in need.rs. It catches a value changed here, but a variant added to `NeedClass` without extending `BUILTIN_ORDER` still passes: no exhaustive match ties `BUILTIN_ORDER` to the enum, and the enum is `#[non_exhaustive]` (follow-on: a guard in need.rs that matches every variant against its index in `BUILTIN_ORDER`).
 */
export type NeedClass = 'care' | 'member' | 'agent' | 'commons';
export type UnansweredInputsGone = Unanswered2 & {
  why: NotCarriedInputsGone;
};
/**
 * When the answer can change: a time a tier or gate gave (`at`), or an event after which it can change (`on-event`, e.g. tier-reachable, posture-changed, parameters-declared, berth-offered). The core never reads it from a local clock: the time is supplied by whoever gave it.
 */
export type ComeBack2 = ComeBackAt2 | ComeBackOnEvent2;
/**
 * What the asker can do locally while the request is unanswered. `nothing` must say why, so 'nothing' is a statement and never a gap.
 */
export type Meanwhile2 =
  | MeanwhileUseHeld2
  | MeanwhileRunHere2
  | MeanwhileBuiltInInForce2
  | MeanwhileNothing2;

/**
 * Why a need is placed where it is, recomputed rather than stored (spec §7.4, interfaces spec §D.9, G13): the need, its key and kind, the ranking method, the parameter set in force by CID and what ratified it, which fallback applied when one did, the inputs digest the commitment carries, the asker's own devices' refusals listed, other members' refusals COUNTED by label and never listed, how many candidates were eligible, who was chosen (each named only at the reach its posture declared), the outcome, and whether the recomputed digest matches the commitment's. Asking why re-runs the pure ranking for one need over the inputs the peer holds; once those are gone, the outcome is `inputs-gone` and the three committed fields are what remain: `method`, `parameterSet` and `inputsDigest`. Revision of interfaces spec §D.9: its prose names these three fields, while its `Explanation` struct has no parameter-set CID; placement_core::Explanation follows the struct and does not carry `parameterSet` yet (follow-on, with the spec seat). Wire shape: this schema is the source of record; placement_core::Explanation (elohim/placement/core/src/explain.rs; `inputsDigest` as 64 hex characters, CIDs as strings) is its Rust mirror, not yet pinned to it by a test (follow-on: a placement-core schema_contract case that validates one Explanation per Outcome form, and one Ineligible, here). Source of truth: the peer's own snapshot of the placement inputs, recomputed per request (Operational, Category C). Not persisted.
 */
export interface PlacementExplanationView {
  /**
   * Need::id, the need's content-derived CID.
   */
  need: string;
  key: NeedKey;
  kind: PlacementKind;
  /**
   * The ranking method in force (MethodId, raw codec).
   */
  method: string;
  parameters: RatificationRef;
  /**
   * The parameter set in force for this need's kind, by its CID (ParameterSet::cid, dag-cbor). It names WHICH set was in force; `parameters` names what ratified it. It is one of the three fields a commitment carries (method, parameter-set CID, inputs digest, spec §7.4 and §D.9), so it is written for every outcome, `inputs-gone` included.
   */
  parameterSet: string;
  /**
   * Which fallback applied when the declared head could not be verified (E10). Null when the declared head was in force (one provenance).
   */
  fallback: null | ParamFallback;
  /**
   * The sha-256 inputs digest, 64 lower-case hex characters (G13).
   */
  inputsDigest: string;
  /**
   * The asker's own devices' refusals, listed with R7's three parts.
   */
  ownRefusals: Ineligible[];
  /**
   * Other members' refusals, counted by IneligibleReason label (the `reason` tag), never listed.
   */
  othersRefused: {
    /**
     * A u32 in Rust; bounded to its width.
     */
    [k: string]: number;
  };
  /**
   * How many candidates were eligible.
   */
  eligible: number;
  /**
   * The chosen carriers, each named only at the reach its posture declared.
   */
  chosen: string[];
  outcome: Outcome;
  /**
   * Whether the recomputed inputs digest equals the commitment's; null when there is no commitment to compare (one provenance).
   */
  digestMatchesCommitment: boolean | null;
}
export interface NeedKeyBlob {
  form: 'blob';
  /**
   * The legacy custody marker.
   */
  id: string;
}
export interface NeedKeyTreeView {
  form: 'tree-view';
  /**
   * TreeView::cid(): the dag-cbor CID of a tree root plus view.
   */
  id: string;
}
export interface NeedKeyTask {
  form: 'task';
  /**
   * The TaskSpec's canonical dag-cbor CID.
   */
  id: string;
}
export interface NeedKeySession {
  form: 'session';
  /**
   * The session's CID.
   */
  id: string;
}
/**
 * What put the parameter set in force (who or what ratified it, not which set: that is `parameterSet`); `built-in` reads as 'no tuning chosen'.
 */
export interface RatificationRef {
  by: RatificationKind;
  /**
   * For `ratified`, the ratifying act's record hash; for `charter`, the charter's CID. For `built-in`, the value the floor's InForce::builtin writes; interfaces spec §E leaves it unnamed (recorded as an open point in the lane-S report).
   */
  id: string;
  /**
   * Null unless the tuning was ratified under a named clause (one provenance).
   */
  rule: null | string;
  /**
   * Whose consent the household default (TouchedConsent) requires: the members the tuning touches (E8).
   */
  touches: string[];
}
export interface Ineligible {
  /**
   * An agent key (`uhCAk…`).
   */
  agent: string;
  why: UnansweredIneligible;
}
/**
 * Ruling 7's one shape for every refusal, absence or 'not carried' across the placement seams: why (the seam's own typed reason), when to come back, and what can be done meanwhile, plus an escalation on a repeat ask. The base envelope leaves `why` unconstrained on purpose, as objects/answer leaves `value`: a per-seam envelope narrows it with `allOf` to that seam's reason (objects/item-absence `$defs.UnansweredItemAbsence`, views/placement-explanation-view's refusals), so the wire stays monomorphic and no TypeScript generic is needed (CONVENTIONS rule 11). Serialization: `comeBack` is internally tagged by `when`, `meanwhile` by `local`, kebab-case tags and camelCase fields; `escalation` is always written and is null when this is not a repeat ask (one provenance: null means 'not a repeat', never 'unknown'). Wire shape: this schema is the source of record; Rust mirror seam_contracts::Unanswered<R> / ComeBack / Meanwhile / Escalation (crates/seam-contracts/src/unanswered.rs). That module calls its `serde` derives (feature `serde`) the internal convenience form, yet they carry the same tags and field names, and a seam adapter that writes this envelope from them must serialize to this schema; no test pins the two yet (follow-on: a seam-contracts contract test that serializes every ComeBack, Meanwhile and Escalation form and validates it here). Source of truth: the seam that answered, per request (Category C): a wire envelope for an answer, never persisted as an entity of its own.
 */
export interface Unanswered {
  /**
   * The seam's own typed reason. Unconstrained in the base envelope; a per-seam envelope narrows it.
   */
  why: unknown;
  comeBack: ComeBack;
  meanwhile: Meanwhile;
  /**
   * Null unless this is a repeat ask before come-back.
   */
  escalation: null | Escalation;
}
export interface ComeBackAt {
  when: 'at';
  /**
   * The time to come back, in seconds since the Unix epoch. A u64 in Rust, bounded here to 9007199254740991 (2^53 - 1, the largest integer every JSON reader, JavaScript included, holds exactly); a larger value is refused, never rounded.
   */
  epochSecs: number;
}
export interface ComeBackOnEvent {
  when: 'on-event';
  /**
   * The event's name, kebab-case.
   */
  event: string;
}
export interface MeanwhileUseHeld {
  local: 'use-held';
  /**
   * What is already local, named.
   */
  what: string;
}
/**
 * The device can do it itself.
 */
export interface MeanwhileRunHere {
  local: 'run-here';
}
/**
 * A built-in default is answering in its place.
 */
export interface MeanwhileBuiltInInForce {
  local: 'built-in-in-force';
}
export interface MeanwhileNothing {
  local: 'nothing';
  /**
   * Why nothing can be done locally.
   */
  because: string;
}
/**
 * Set on a repeat ask before come-back: the answer escalates rather than repeats.
 */
export interface Escalation {
  /**
   * How many times the request has been asked, this ask included, so the first escalation is 2 (`Unanswered::escalate`).
   */
  asked: number;
  /**
   * What changed since the last answer, said even when it is "nothing".
   */
  changed: string;
  /**
   * Who else can be asked.
   */
  whoCanBeAsked: string[];
}
export interface IneligibleNoPosture {
  reason: 'no-posture';
}
export interface IneligibleUnknown {
  reason: 'unknown';
  /**
   * The missing input, e.g. liveness, reach, consent.
   */
  input: string;
}
export interface IneligibleOutsideReach {
  reason: 'outside-reach';
  need: Reach;
  admitted: Reach1;
}
export interface IneligibleOverCapacity {
  reason: 'over-capacity';
}
export interface IneligibleNoConsent {
  reason: 'no-consent';
}
export interface IneligibleConsentExpired {
  reason: 'consent-expired';
}
export interface IneligibleIsReceiver {
  reason: 'is-receiver';
}
export interface IneligibleNotNamed {
  reason: 'not-named';
}
export interface IneligibleCrossHouseholdBelowPublic {
  reason: 'cross-household-below-public';
}
export interface IneligibleKind {
  reason: 'kind';
  /**
   * The kind's own refusal label, e.g. scope, isolation.
   */
  label: string;
}
export interface OutcomeCarried {
  outcome: 'carried';
}
export interface OutcomeHeld {
  outcome: 'held';
}
export interface OutcomeNotCarried {
  outcome: 'not-carried';
  why: UnansweredNotCarried;
}
/**
 * Ruling 7's one shape for every refusal, absence or 'not carried' across the placement seams: why (the seam's own typed reason), when to come back, and what can be done meanwhile, plus an escalation on a repeat ask. The base envelope leaves `why` unconstrained on purpose, as objects/answer leaves `value`: a per-seam envelope narrows it with `allOf` to that seam's reason (objects/item-absence `$defs.UnansweredItemAbsence`, views/placement-explanation-view's refusals), so the wire stays monomorphic and no TypeScript generic is needed (CONVENTIONS rule 11). Serialization: `comeBack` is internally tagged by `when`, `meanwhile` by `local`, kebab-case tags and camelCase fields; `escalation` is always written and is null when this is not a repeat ask (one provenance: null means 'not a repeat', never 'unknown'). Wire shape: this schema is the source of record; Rust mirror seam_contracts::Unanswered<R> / ComeBack / Meanwhile / Escalation (crates/seam-contracts/src/unanswered.rs). That module calls its `serde` derives (feature `serde`) the internal convenience form, yet they carry the same tags and field names, and a seam adapter that writes this envelope from them must serialize to this schema; no test pins the two yet (follow-on: a seam-contracts contract test that serializes every ComeBack, Meanwhile and Escalation form and validates it here). Source of truth: the seam that answered, per request (Category C): a wire envelope for an answer, never persisted as an entity of its own.
 */
export interface Unanswered1 {
  /**
   * The seam's own typed reason. Unconstrained in the base envelope; a per-seam envelope narrows it.
   */
  why: unknown;
  comeBack: ComeBack1;
  meanwhile: Meanwhile1;
  /**
   * Null unless this is a repeat ask before come-back.
   */
  escalation: null | Escalation1;
}
export interface ComeBackAt1 {
  when: 'at';
  /**
   * The time to come back, in seconds since the Unix epoch. A u64 in Rust, bounded here to 9007199254740991 (2^53 - 1, the largest integer every JSON reader, JavaScript included, holds exactly); a larger value is refused, never rounded.
   */
  epochSecs: number;
}
export interface ComeBackOnEvent1 {
  when: 'on-event';
  /**
   * The event's name, kebab-case.
   */
  event: string;
}
export interface MeanwhileUseHeld1 {
  local: 'use-held';
  /**
   * What is already local, named.
   */
  what: string;
}
/**
 * The device can do it itself.
 */
export interface MeanwhileRunHere1 {
  local: 'run-here';
}
/**
 * A built-in default is answering in its place.
 */
export interface MeanwhileBuiltInInForce1 {
  local: 'built-in-in-force';
}
export interface MeanwhileNothing1 {
  local: 'nothing';
  /**
   * Why nothing can be done locally.
   */
  because: string;
}
/**
 * Set on a repeat ask before come-back: the answer escalates rather than repeats.
 */
export interface Escalation1 {
  /**
   * How many times the request has been asked, this ask included, so the first escalation is 2 (`Unanswered::escalate`).
   */
  asked: number;
  /**
   * What changed since the last answer, said even when it is "nothing".
   */
  changed: string;
  /**
   * Who else can be asked.
   */
  whoCanBeAsked: string[];
}
export interface NotCarriedNoEligible {
  reason: 'no-eligible';
}
export interface NotCarriedDomainShort {
  reason: 'domain-short';
  /**
   * Honored distinct domains.
   */
  have: number;
  /**
   * Domains wanted.
   */
  need: number;
}
export interface NotCarriedServedFirst {
  reason: 'served-first';
  class: NeedClass;
}
export interface NotCarriedNoPriorityChosen {
  reason: 'no-priority-chosen';
}
export interface NotCarriedMethodNotRun {
  reason: 'method-not-run';
}
export interface NotCarriedInputsGone {
  reason: 'inputs-gone';
}
export interface OutcomeInputsGone {
  outcome: 'inputs-gone';
  why: UnansweredInputsGone;
}
/**
 * Ruling 7's one shape for every refusal, absence or 'not carried' across the placement seams: why (the seam's own typed reason), when to come back, and what can be done meanwhile, plus an escalation on a repeat ask. The base envelope leaves `why` unconstrained on purpose, as objects/answer leaves `value`: a per-seam envelope narrows it with `allOf` to that seam's reason (objects/item-absence `$defs.UnansweredItemAbsence`, views/placement-explanation-view's refusals), so the wire stays monomorphic and no TypeScript generic is needed (CONVENTIONS rule 11). Serialization: `comeBack` is internally tagged by `when`, `meanwhile` by `local`, kebab-case tags and camelCase fields; `escalation` is always written and is null when this is not a repeat ask (one provenance: null means 'not a repeat', never 'unknown'). Wire shape: this schema is the source of record; Rust mirror seam_contracts::Unanswered<R> / ComeBack / Meanwhile / Escalation (crates/seam-contracts/src/unanswered.rs). That module calls its `serde` derives (feature `serde`) the internal convenience form, yet they carry the same tags and field names, and a seam adapter that writes this envelope from them must serialize to this schema; no test pins the two yet (follow-on: a seam-contracts contract test that serializes every ComeBack, Meanwhile and Escalation form and validates it here). Source of truth: the seam that answered, per request (Category C): a wire envelope for an answer, never persisted as an entity of its own.
 */
export interface Unanswered2 {
  /**
   * The seam's own typed reason. Unconstrained in the base envelope; a per-seam envelope narrows it.
   */
  why: unknown;
  comeBack: ComeBack2;
  meanwhile: Meanwhile2;
  /**
   * Null unless this is a repeat ask before come-back.
   */
  escalation: null | Escalation2;
}
export interface ComeBackAt2 {
  when: 'at';
  /**
   * The time to come back, in seconds since the Unix epoch. A u64 in Rust, bounded here to 9007199254740991 (2^53 - 1, the largest integer every JSON reader, JavaScript included, holds exactly); a larger value is refused, never rounded.
   */
  epochSecs: number;
}
export interface ComeBackOnEvent2 {
  when: 'on-event';
  /**
   * The event's name, kebab-case.
   */
  event: string;
}
export interface MeanwhileUseHeld2 {
  local: 'use-held';
  /**
   * What is already local, named.
   */
  what: string;
}
/**
 * The device can do it itself.
 */
export interface MeanwhileRunHere2 {
  local: 'run-here';
}
/**
 * A built-in default is answering in its place.
 */
export interface MeanwhileBuiltInInForce2 {
  local: 'built-in-in-force';
}
export interface MeanwhileNothing2 {
  local: 'nothing';
  /**
   * Why nothing can be done locally.
   */
  because: string;
}
/**
 * Set on a repeat ask before come-back: the answer escalates rather than repeats.
 */
export interface Escalation2 {
  /**
   * How many times the request has been asked, this ask included, so the first escalation is 2 (`Unanswered::escalate`).
   */
  asked: number;
  /**
   * What changed since the last answer, said even when it is "nothing".
   */
  changed: string;
  /**
   * Who else can be asked.
   */
  whoCanBeAsked: string[];
}
