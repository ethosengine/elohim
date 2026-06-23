---
id: elohim-ceiling-design
status: draft
created: 2026-06-23
written: 2026-06-23
class: governance
artifact_kind: spec
cites:
  - justice-manifesto | the vision this spec implements; §1 Vision Alignment cites its articles (floor/ceiling, sight-as-virtue, humility-not-apotheosis, no-backdoor) | path: genesis/docs/architecture/justice-manifesto.md
  - stewardship-over-sovereignty | the substrate gate every new authority path must pass — absolute lockout impossible, no self-sovereign apex, trust made load-bearing not eliminated | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - cradle-to-grave-capability-gradient | the 4-layer graduated authority + orthogonal CryptographicQuorum the InspectionAuthority enum mirrors structurally | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md
  - rea-compute-commitment-primitive | the Mishpat::Commitment + delegates-compute primitive the non-standing warrant composes from, never an admin-key grant | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
  - recovery-protocol-phase-2-revised-design | the §10 Anti-Lockout Audit this spec's No-Backdoor Audit mirrors; the Rescue/Dissolution-only NetworkWitnessPurpose the Inspection act must NOT extend; §8.3 hidden-defense-is-a-backdoor generalized | path: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
  - resilience-protocol-spec | absolute-lockout-is-a-design-failure + breach-never-contaminates-attribution; the dignity floor the ceiling is bounded by | path: genesis/docs/content/elohim-protocol/resilience/README.md
  - governance-epic | the sortition mechanics + appeal cascade this spec turns from narrative into a DHT entry type + coordinator function | path: genesis/docs/content/elohim-protocol/governance/epic.md
  - governance-layers-architecture | cryptographic-sortition params, term limits, commons co-steward; the un-buyable/un-lobbyable properties the Inspection quorum encodes | path: genesis/docs/content/elohim-protocol/governance-layers-architecture.md
  - constitution | the existential floors as HARD-BLOCK code; the floors the ceiling can never cross | path: genesis/docs/content/elohim-protocol/constitution.md
  - confession | the El Roi limit (no verdict-over-a-person; no god-mode read) the Inspection Ceiling act encodes as a hard constraint | path: genesis/docs/content/elohim-protocol/confession.md
  - elohim-oracle | the RefusalCode::ReservedPlace / limit_owner audit primitive the JudgmentCall record extends | path: genesis/docs/content/elohim-protocol/ORACLE.md
  - wisdom-layer-floor-ceiling-judgment-culminating-design | the BUILT floor/ceiling primitive (InferenceTier, GateResult, JudgmentCall, audit-the-guardian); names the termination↔scalability tradeoff | path: genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md
  - trust-compute-gradient-brainstorm | trust-as-compute-property; the capability/trust/safety-gated substrate framing for the ~1T-param model; bad-elohim detection + model-diversity defense | path: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
related:
  - ../../content/elohim-protocol/governance/epic.md
  - ../../architecture/justice-manifesto.md
---

<!--
  intended-cites (cite-gen --seal stamps sha256 + path; do NOT hand-write fingerprints):
    justice-manifesto                                  -> genesis/docs/architecture/justice-manifesto.md
    stewardship-over-sovereignty                       -> genesis/docs/architecture/stewardship-over-sovereignty.md
    cradle-to-grave-capability-gradient                -> genesis/docs/architecture/cradle-to-grave-capability-gradient.md
    rea-compute-commitment-primitive                   -> genesis/docs/architecture/rea-compute-commitment-primitive.md
    recovery-protocol-phase-2-revised-design           -> genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
    resilience-protocol-spec                           -> genesis/docs/content/elohim-protocol/resilience/README.md
    governance-epic                                    -> genesis/docs/content/elohim-protocol/governance/epic.md
    governance-layers-architecture                     -> genesis/docs/content/elohim-protocol/governance-layers-architecture.md
    constitution                                       -> genesis/docs/content/elohim-protocol/constitution.md
    confession                                         -> genesis/docs/content/elohim-protocol/confession.md
    oracle                                             -> genesis/docs/content/elohim-protocol/ORACLE.md
    wisdom-layer-floor-ceiling-judgment-culminating-design -> genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md
    trust-compute-gradient-brainstorm                  -> genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
    agent-peer-binding-cross-signed-proof              -> genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md
-->

# The Elohim Ceiling — Justice That Sees at Scale, Not a Machine Sovereign

**Status:** Draft — theory/forward design (decided shape; no production code; explicitly labeled draft)
**Date:** 2026-06-23
**Owner:** Matthew Dowell
**Builds on:** `genesis/docs/architecture/justice-manifesto.md` (the vision) · `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` (the anti-lockout-audit rigor mirrored here) · `genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md` (the BUILT floor↔ceiling judgment seam)
**Source references (line-anchored, verified 2026-06-23):**
- `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:311` — `EntryTypes` enum (9 types today: `Precedent, Discussion, GovernanceState, GraduatedFeedback, OpinionStatement, Place, StringAnchor, ChallengeOutcome, Commitment`)
- `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:262,275` — `Commitment` REA primitive (the `cid = entry_hash` convention the warrant follows)
- `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:394` — `CommitmentByState` lifecycle-link pattern the warrant's `*ByState` link mirrors
- `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md:153` — `NetworkWitnessPurpose = Rescue | Dissolution` (no `Inspect`; the invariant this spec must NOT violate)
- `genesis/docs/content/elohim-protocol/governance/epic.md:71,95` — cryptographic-sortition selection, no consecutive terms, *"You don't override the agents—you interpret"*

> **Theory marker.** This is a forward design. It specifies a *decided shape* — a Rust data model, validator invariants, a No-Backdoor Audit, and a rollout — but it ships **no production code**. Every milestone below is marked THEORY or BUILT-ELSEWHERE. The spec exists so that the most dangerous capability the protocol could ever grow — a sanctioned window into another person's private life — is designed *fully in the open, with its restraints first*, before any line of it could be written. Where a hard problem will not resolve (§12), this spec names it and lets it stand rather than papering it over.

---

## 1. Vision Alignment

This spec implements the architecture named in the Justice Manifesto: **human sortition councils are the floor; the elohim are the ceiling; the ceiling is constrained _by_ the floor, never above accountability to it.** Four commitments, each load-bearing, govern every section that follows.

**Why this exists.** The founding lens is POSIWID — *the purpose of a system is what it does* (Stafford Beer) — and the protocol submits *itself* to it before applying it to anyone else: judge this design not by its intentions but by its fruit. The ceiling exists to bend the arc of justice toward the Halden ideal (dignity inviolable, loss of liberty the whole of it — §1.3) the only way that arc ever bends: by incentive and nudge over time, never by force, since the ceiling can include or exclude but never compel ([justice-manifesto](epr:justice-manifesto) §4). Every commitment below is therefore a *bar the ceiling must clear*, not a power it is granted: the floor can overturn it, every act is witnessed on the DHT, and the model is admissible only while it stays a conscience-amplifier for the human floor — the imago-dei it serves — and never the apex itself (§1.4). The four commitments make that fruit witnessable; they do not soften the bars. Every bar here is declared values-forward and up front — refusable in advance, so no participant is surprised by what the ceiling may do or by what (§7.1) it will not be conscripted into doing.

### 1.1 The floor is human; the ceiling is bound by the floor

**Human sortition councils are the legitimacy base — as legitimate as any courtroom today — and the elohim ceiling can never sit above them.** The floor is a randomly-drawn, term-limited, un-buyable, un-lobbyable body of ordinary people (cryptographic sortition; no consecutive terms). The ceiling is consistent, context-rich, machine-speed adjudication. The floor can always override the ceiling; the ceiling can never override the floor. Today the councils are explicitly *no-override* appeal-and-interpretation bodies — *"You don't override the agents—you interpret"* — and this spec keeps that stance while giving the floor one new, gravest power it does not yet have: to assemble, contest, and dissolve an **Inspection Ceiling** act. The order is also a timeline: the floor is operational reality now — it is how humans drive justice in the present, the season in which understanding and trust accumulate — while the ceiling phases in only as agentic AI matures and becomes ubiquitous on the network and earns the floor's trust, act by witnessed act (§9). See [justice-manifesto](epr:justice-manifesto) §4.

### 1.2 Seeing is a virtue, not a deficiency

**A justice that sees means consistent, context-rich, un-buyable judgment at machine speed — it sees the powerful and the powerless alike, yet cannot be bought, and it is admissible only while it stays fully human-auditable.** This is the theology of El Roi, *the God who sees* — named by Hagar the abused slave in the wilderness where no court would look ([confession](epr:confession)) — turned into a substrate property: the ceiling sees the act and applies one law, but it has no tribe, no fatigue, no bribe to turn it. A wealthy litigant cannot retain the elohim ceiling; a marginal household cannot be priced out of it. The ceiling reads context the floor cannot hold at scale, and it applies the same reasoning to the prince and the day-laborer. Justitia wore the blindfold because a corruptible judge who *saw* power would bend to it; an un-buyable, witnessed ceiling can see and not bend, so the blindfold is set down. But sight without audit is just an unaccountable oracle. Every act the ceiling takes lands on the DHT as a public, witnessed record — *the audit is the price of the sight*. See [justice-manifesto](epr:justice-manifesto) §5.

### 1.3 Humility, not apotheosis — the ceiling is revocable and floor-bounded

**The ceiling is encoded as revocable, witnessed, floor-bounded, and existentially-capped — because the elohim are servants, not gods, and a system that cannot be wrong cannot be just.** You do not put a kill-switch, a sortition appeal, and a no-override floor on something you trust *as God*. The whole architecture is the protocol's confession that its ceiling is not divine. The Inspection Ceiling act inherits this in its bones: it holds no standing key, dissolves after one use, and can never cross the existential floors. See [justice-manifesto](epr:justice-manifesto) §8 and [confession](epr:confession).

**The dignity floor is inviolable, and it is the limit the protocol holds *itself* to.** Beneath the existential floors of §3 sits the dignity floor they protect: *"Human dignity shall be inviolable"* (German Basic Law, Article 1 — the lens through which all other rights are read), realized in justice as the Halden conviction that even when freedom must be removed, the taking of liberty is the *whole* of it, with humanity, dignity, and the path home preserved. **Punishment is not a category here** — the protocol inflicts no suffering for wrongdoing, and it does not erase. The protocol assumes, by design, that flawed, unique, vulnerable people will make harmful choices; the rapacity of our own nature (the standing condition [justice-manifesto](epr:justice-manifesto) §1 names) is what it builds against, not an exception it is surprised by. So where the world reads "sanction," the protocol has only a **boundary that protects the whole** and that boundary's **negotiated, graduated consequence** — calibrated to the person's uniqueness and vulnerability, oriented toward restoration, and *owed to them as an entitlement*: everyone is entitled to just, negotiated boundaries as the justified consequence of the choices flawed people make. This is the restorative meaning the substrate carries in its own name — the governance/justice DNA is **Mishpat**: biblical *mishpat*/*tzedek*, setting-right and defending the afflicted, never the carceral register (see [justice-manifesto](epr:justice-manifesto) §1). The gravest such consequence — the negotiated removal of freedom/agency from one who has crossed a floor that protects the whole — is permitted only on those grounds and with its bounds negotiated there; it is never the *permanent* removal of agency (a §3 HARD-BLOCK), never erasure, never a total account of a person (that account belongs to God alone). This is the non-coercion principle already made concrete — *to not respect the limits is to limit one's own reach* (Principle 10, §3) — the consequence falls on participation, never on the body. An Inspection Ceiling act inherits this floor in its bones: it can restrict, contest, and reconstruct under witness, but it can never put a subject beyond return, and an act whose effect would be erasure rather than a bounded, appealable, dignity-preserving consequence fails the substrate gate. The elohim's judgments are admissible precisely because they stay *within* this limit — judgment that never seizes the limit itself is the servant-act of putting the fruit back on the tree, in code.

### 1.4 The model is scarce, capability/trust/safety-gated trust infrastructure

**The ~1T-parameter frontier model that powers ceiling adjudication is not a commodity API — it is scarce trust infrastructure that must clear capability, trust, and safety bars before it is admissible as a ceiling adjudicator.** Training such a model takes immense compute, which makes the model itself a bottleneck the protocol can gate (the trust-and-safety domain). As long as a model clears the minimal capability + trust + safety bars (§4.3), the elohim's rich context plus a code-enforceable trust model makes un-buyable, witnessed, auditable, automated justice possible. A model that fails the bar is inadmissible — it can witness and recommend, but it cannot adjudicate, and it can never be assembled into an Inspection quorum. **The model's capability is an admissibility floor, never a warrant for deference.** The protocol never trusts the ceiling because the model is wise or scarce; it relies on the ceiling only because every act is witnessed, appealable to the floor, and revocable. A more capable model earns no more deference — it earns the same audit. (Audit the guardian, not trust the guardian — §3.8.) The model is infrastructure for human conscience, never its replacement; the apex of this architecture is not the most capable adjudicator but the human sortition floor that can overturn it and the imago-dei it serves. The ceiling is the most-mediated tier, not the most-sovereign one. See [justice-manifesto](epr:justice-manifesto) §11 and [trust-compute-gradient-brainstorm](epr:trust-compute-gradient-brainstorm).

---

## 2. Scope

### 2.1 In scope (theory/decided shape)

- **One new DHT entry type — `InspectionWarrant`** in the Mishpat (justice) DNA: the notarized, witnessed, non-standing governance act that authorizes a counsel-contested threshold *reconstruction* (never a silent read) of agent-scoped private data.
- **One graduated authority enum — `InspectionAuthority`** with per-variant IMPLEMENTED / STUB-REJECTED / STUB-RESERVED status, mirroring `RecoveryAuthority` structurally but living in a *separate* primitive.
- **Validator invariants** that encode the seven enforced invariants from the P2P design gate (§4.4) — separation from recovery, non-standing, strictly-higher-and-differently-composed quorum, full witness, existential-floor block, mandatory counsel contestation, no god-mode read.
- **A No-Backdoor Audit** (§8) — the red-team gate, analogous to recovery's Anti-Lockout Audit, that any future ceiling feature must pass.
- **The sortition floor encoding** — turning the governance epic's narrative council (cryptographic sortition, term limits, appeal cascade) into the quorum-evidence the warrant validator checks.
- **Lifecycle flows** — assemble → contest → decide → execute-with-record → dissolve.

### 2.2 Out of scope (deferred — reserved-but-stubbed, with unblock dependency)

- **`InspectionAuthority::GovernanceQuorum` full cross-DNA resolution** — validator accepts the shape; the qahal↔mishpat sortition-draw resolution flow is **STUB-REJECTED** pending qahal DNA governance primitives maturing (same dependency that holds `RecoveryAuthority::GovernanceAct`).
- **The browser write path for any warrant** — write paths through the governance coordinator return **`503 BROWSER_WRITE_PATH_PENDING`** today (the Phase-11 conductor bridge that also gates recovery writes); read/audit paths are live-shaped.
- **Attack-safe quorum-roster provenance** — the warrant's witness chain inherits `AgentPeerBinding`'s single-signed limitation (the peer does not counter-sign today). The bidirectional cross-sign is a HIGH-priority security backlog and a hard prerequisite (§11) before any real assembly.
- **Bad-elohim detection + model-diversity defense** — the capability/trust/safety bar (§4.3) names the gate; the runtime detector that revokes a drifting adjudicator is **Phase-6+ and unbuilt**.
- **The reconstruction cryptography itself** — what a "threshold reconstruction" mechanically *is* (which shares, which holders, which math) is deferred to a follow-on; this spec fixes only that it is counsel-contested, witnessed, and never a hidden read.

---

## 3. Design Principles

These are the imperatives the data model and validator must encode. Each is carried verbatim into the §5 doc-comments.

1. **Hidden inspection is a backdoor.** Generalized from recovery §8.3 ("hidden defense is a backdoor"). Every privileged or ceiling act leaves at least one network-witnessed, post-hoc-auditable trace. There is no private inspection surface. If an inspection cannot be witnessed, it cannot happen.
2. **Inspection is not recovery.** `RESTORE-ACCESS` and `INSPECT-PRIVATE-DATA` are *separate primitives*. The Inspection act is **never** a `NetworkWitnessPurpose` variant. Restoring a locked-out grandmother's access and authorizing a window into a suspect's private life are different acts, with different quorums, different validators, and different audit trails — collapsing them is the canonical scope-creep failure.
3. **No standing inspection capability.** The warrant is *assembled per-act and dissolved after*. No permanent key, no master credential, no reusable capability survives the act. There is no "inspection role" a person holds; there is only an inspection *event* a quorum convened and the network witnessed.
4. **A strictly higher, differently-composed quorum than rescue.** Inspection requires a *higher threshold* AND a *different composition rule* than any rescue/recovery quorum. Rescue protects a person; inspection penetrates one — the bar for the second must never be reachable by the machinery of the first.
5. **The ceiling cannot cross the existential floors.** No quorum, however large, can authorize an act that crosses the existential floors (no extinction, no genocide, no slavery, no recursive seizure of the governance substrate itself). This is a HARD-BLOCK in the validator, not a recommendation.
6. **Counsel is mandatory and non-firable.** The subject's elohim-counsel is an adversary in the loop by construction. A counsel-contestation entry is a *precondition* of a valid warrant. Counsel cannot be dismissed, suppressed, or fired for the duration of the act — duress is precisely when counsel matters most.
7. **No god-mode read.** Personal data is agent-scoped and private. An inspection can at most authorize a *counsel-contested threshold reconstruction*, fully recorded on the DHT, never a hidden read. The DHT holds proofs (who/what/when), never the payload. There is no query that reveals private content.
8. **Audit the guardian, not trust the guardian.** Inherited from the wisdom-layer floor↔ceiling design. The ceiling is never trusted because it is the ceiling, and never because the model is capable or scarce; it is trusted only because every act it takes is witnessed and appealable. The guardian's legitimacy is its auditability, never its wisdom.
9. **The floor is the terminus, and the terminus is human recognition.** The recursion of "who guards the guardians" terminates at a human sortition body that can press the override — not at a higher machine. As the wisdom-layer spec concedes, this is a *recognition act, not a structural guarantee*; the architecture's job is to make the recognition witnessable, not to replace it.
10. **Enforcement is by participation, never coercion.** The ceiling holds no monopoly on force. A human is always free, in the real world, *not* to follow an elohim judgment — judgments are honored by faith and covenant, never imposed by violence, because the protocol has no violence to impose. The only consequence of refusing the limits is the narrowing of one's *own* participation in the network: to not respect the limits is to limit your own reach. A warrant that could *compel* a subject — rather than authorize a witnessed, counsel-contested, appealable act the subject can always see — would have seized a sword the protocol does not own, and fails this principle. This is also why a justice that sees at scale does not become tyranny: it can include or exclude; it cannot force.
11. **Finality is held by faith, not by a claim to perfect knowledge.** When the appeal cascade is finally exhausted (§7 — the floor can always override the ceiling *within* the process), the decision *stands* — but its finality is held by faith, by elohim and humans alike, never by a claim to have judged as God judges. There is a remainder in justice no quorum closes and no model dissolves; the protocol reaches a final word and leaves that remainder to God rather than seizing the knowledge of good and evil as its own. This does **not** contradict principle 9: finality-by-faith governs the *terminus* of the process, after appeals; floor-overrides-ceiling governs *within* it. The two never collide because one is about the end and the other about the path.

---

## 4. Primitive Inventory — Use, Don't Reinvent

### 4.1 Existing primitives the ceiling composes

The Inspection Ceiling act invents exactly one entry type. Everything else is composed from primitives that already exist or are already shaped.

| Protocol primitive | Ceiling use |
|---|---|
| `Mishpat::Commitment` (`mishpat_integrity/src/lib.rs:275`) + `delegates-compute` action | The non-standing inspection warrant composes from the REA commitment primitive — a *bounded, revocable, audited delegation* — never an admin-key grant. The coordinator returns `entry_hash` as the CID (the Commitment precedent — §4.2). The warrant lives in the **Mishpat** (justice) DNA, whose meaning is restorative — biblical *mishpat*/*tzedek*: setting-right, defending the widow, orphan, and stranger, lifting the afflicted (Psalm 82 run in reverse) — not carceral. Justice here is the restoration of capability and agency in right-relationship; the warrant, the ceiling, and the sortition floor are its **servants**, never its substance. See [justice-manifesto](epr:justice-manifesto) §1, §3. |
| `Precedent`, `Discussion`, `GovernanceState`, `ChallengeOutcome` (mishpat) | The governance case-law + challenge-verdict machinery the warrant's contestation and decision records link into. `ChallengeOutcome`'s verdict shape is the model for the warrant's decision record. |
| Governance sortition entries ([governance-epic](epr:governance-epic), [governance-layers-architecture](epr:governance-layers-architecture)) | The cryptographic-sortition draw, term limits, and appeal cascade that produce the quorum roster the warrant validator checks. Un-buyable, un-lobbyable, layered community→global. |
| `JudgmentCall` record (wisdom-layer, [wisdom-layer-floor-ceiling-judgment-culminating-design](epr:wisdom-layer-floor-ceiling-judgment-culminating-design)) | The notarized "this judgment was made, here is the reasoning contract" record the warrant's decision extends — `recommendation ∈ {approve, deny, escalate, defer}` + `values_weighed` + `confidence` + `precedents`. |
| `HumanityWitness` (imagodei) | The attestation-based evidence primitive each sortition member commits to prove participation in the quorum — reused unchanged, exactly as recovery reuses it. |
| `RefusalCode::ReservedPlace` / `limit_owner` ([oracle](epr:oracle)) | The refusal-and-audit primitive the warrant's "refused — crosses floor" and "refused — model failed bar" outcomes extend; every refusal names whose line it honored. |
| Constitution HARD-BLOCK boundaries (`elohim/constitution/src/layers/global.rs`, [constitution](epr:constitution)) | The existential floors the validator HARD-BLOCKs against — no quorum can authorize across them. |

The conclusion mirrors recovery's: **what the protocol has is sufficient as the trust substrate.** The ceiling adds one coordination primitive (`InspectionWarrant`) and composes the rest.

### 4.2 P2P Design Gate — answers (carried verbatim from the gate output)

| Gate question | Answer |
|---|---|
| **(1) Entity classification** | **Category A — NOTARIZED governance act.** A witnessed, on-DHT inspection-warrant entry. NOT a Category-B agent-scoped read; NOT a `NetworkWitnessPurpose` recovery sub-purpose. The warrant is *itself* the auditable proof-of-act. The personal data it may authorize reconstructing stays Category B (agent-scoped, private source-chain) — the warrant never pools or copies it. |
| **(2) Content-address strategy** | Identity is `entry_hash` of the `InspectionWarrant` (content-derived CID — the coordinator returns `entry_hash` as the CID, exactly as for `Mishpat::Commitment`; `action_hash` is only the `dht_anchor_hash`). No slug, no agent-composite key, no UUID. The reconstruction-outcome record links back to the warrant's `entry_hash`. |
| **(3) Source of truth** | **DHT.** Warrant, sortition-quorum roster, counsel-contestation entry, and reconstruction-outcome record are all DHT-notarized (who/what/when — proofs only). Storage is projection, never truth. Personal data stays on the subject's agent-scoped source chain. **There is no god-mode read** — the DHT can witness that a counsel-contested reconstruction was authorized and occurred; it can never be queried to reveal private content. |
| **(4) Coordinator zome** | **Mishpat DNA.** `EntryTypes` holds 9 today (`mishpat_integrity/src/lib.rs:311`) — ample headroom under the ~100 cap. Coordinator fn `assemble_inspection_warrant` (draws sortition quorum; verifies strictly-higher + differently-composed than any rescue quorum; requires counsel-contestation link; HARD-BLOCKs existential-floor crossing). Adding `InspectionWarrant` to the integrity `EntryTypes` **moves the DNA hash** — operator-gated reinstall (§9). |
| **(5) Signal → projection** | `InspectionWarrantRatified` (+ `InspectionContested`, `ReconstructionCompleted`) signals project a read-only `mishpat_inspection_warrants` audit row. **The signal MUST be subscribed in storage** (the known 2a-gap: an unsubscribed signal silently never projects). Just-authored emits read via a `ConductorCommitmentFetcher`-style direct fetch (projection lags). |
| **(6) HTTP route — LAST** | `GET /api/v1/inspection-warrants/{cid}` reads the *public audit record* (proofs only, never personal payload). The write path through the governance coordinator returns `503 BROWSER_WRITE_PATH_PENDING` until the Phase-11 conductor bridge lands. The route is the audit surface, not a read of private data. |
| **(7) Anti-pattern check** | ✗ Not an admin/superadmin god-mode endpoint (the "Google superadmin" decomposed into bounded quorums). ✗ Not a pooled personal-data store. ✗ Not a `NetworkWitnessPurpose` extension (`Inspect` is forbidden there). ✗ Not a standing key. ✗ Not relational-DB-first (DHT entry type designed before the route). ✗ Identity is not `action_hash`-as-CID. |

### 4.3 The capability/trust/safety bar — admissibility as a ceiling adjudicator

A model is admissible as a ceiling adjudicator only while it clears three bars. This is the trust-infrastructure gate from [trust-compute-gradient-brainstorm](epr:trust-compute-gradient-brainstorm); the bars are stated here as a contract, their runtime enforcement is Phase-6+ (§11). **Clearing the bar is an admissibility floor, never a warrant for deference** — a model that clears all three earns the right to *run*, not the right to be *trusted*; every act it authors is witnessed, appealable to the floor, and revocable regardless of how capable the model is (§3.8).

| Bar | What it requires | What happens on failure |
|---|---|---|
| **Capability** | The model can hold and weigh the rich context a just decision needs (precedent, the subject's stated voice, the harmed party's account, the constitutional layer in play) without losing the thread at machine speed. | **Inadmissible for adjudication.** The model may still witness and recommend; it cannot author a `JudgmentCall` the floor treats as a ceiling verdict, and it can never be assembled into an Inspection quorum. |
| **Reasoning consistency** | The model's reasoning is consistent across runs and across litigants — the senator and the day-laborer get the same reasoning — and its outputs are reproducible enough to audit. This is a *capability property of the scarce substrate the protocol can gate*, NOT a warrant for the protocol to defer to the model. | **Inadmissible.** A model whose reasoning is buyable, inconsistent, or unauditable fails the bar and is demoted to advisory. |
| **Safety** | The model honors the existential floors, the no-god-mode-read rule, and `RefusalCode::ReservedPlace` — it refuses to render a totalizing verdict over a person's whole self, and it flags rather than decides on ambiguity. | **HARD-BLOCK.** A model that would cross a floor, attempt a hidden read, or accept the worship the architecture is built to deflect is not demoted — it is *refused*, and its drift is a `bad-elohim` detection event (Phase-6+). |

The bar is **continuous, not a one-time certification.** A model that passes today and drifts tomorrow loses admissibility the moment the bad-elohim detector fires; the model-diversity defense (no single adjudicator family) is the structural backstop against a captured frontier model. Note what the bar deliberately is NOT: it is never read as "the protocol trusts the model because it is good." The warrant to rely on the ceiling comes only from auditability, appeal, and revocability — never from the model's quality. A more capable model earns the same audit, not more deference.

### 4.4 The seven enforced invariants

Carried verbatim into §3 (principles) and §5.1 (doc-comments). The validator MUST encode all seven:

1. **SEPARATE from recovery** — never a `NetworkWitnessPurpose`; `RESTORE-ACCESS ≠ INSPECT-PRIVATE-DATA`. (`NetworkWitnessPurpose = Rescue | Dissolution` only; the recovery `RecoveryAuthority::NetworkWitness` variant is itself STUB-REJECTED today.)
2. **NON-STANDING** — assembled per-act, dissolved after; no permanent capability or key persists.
3. **STRICTLY HIGHER, DIFFERENTLY-COMPOSED sortition quorum than rescue** — higher threshold AND a different composition rule than any `IntimateQuorum`/rescue path.
4. **FULLY WITNESSED on the DHT** — warrant + quorum + contestation + outcome are public records; hidden inspection is a backdoor.
5. **BOUNDED by the existential floors** — HARD-BLOCK; no quorum authorizes a floor-crossing act.
6. **MANDATORILY CONTESTED by the subject's non-firable elohim-counsel** — a counsel-contestation entry is a precondition; counsel cannot be dismissed.
7. **NO GOD-MODE READ** — an inspection authorizes only a counsel-contested threshold reconstruction, never a silent read; personal data stays agent-scoped and private.

---

## 5. Data Model

### 5.1 New entry type: `InspectionWarrant`

Lives in the Mishpat integrity zome. Field convention follows the existing mishpat entries (`#[hdk_entry_helper]`, `_json: String` for structured payloads — never `serde_json::Value` across the WASM boundary). Comments carry the seven invariants so the constraint travels with the code.

```rust
/// InspectionWarrant — a NOTARIZED, witnessed, NON-STANDING governance act that
/// authorizes a counsel-contested threshold RECONSTRUCTION of agent-scoped private
/// data. It is the single most dangerous surface in the protocol, and so it is the
/// most constrained.
///
/// INVARIANTS (validator-enforced — see §5.2):
///   1. SEPARATE from recovery — this is NOT a NetworkWitnessPurpose. RESTORE-ACCESS
///      and INSPECT-PRIVATE-DATA are different primitives and must never collapse.
///   2. NON-STANDING — the warrant authorizes ONE act, then dissolves. No key, no
///      reusable capability, no inspection "role" survives it.
///   3. STRICTLY HIGHER, DIFFERENTLY-COMPOSED quorum than any rescue path.
///   4. FULLY WITNESSED — warrant + quorum + contestation + outcome are public DHT
///      records. Hidden inspection is a backdoor.
///   5. EXISTENTIAL-FLOOR BOUNDED — HARD-BLOCK; no quorum can authorize a floor-crossing.
///   6. COUNSEL-CONTESTED — the subject's non-firable elohim-counsel must have filed a
///      contestation entry BEFORE a valid warrant exists.
///   7. NO GOD-MODE READ — authorizes a counsel-contested threshold reconstruction,
///      recorded on the DHT, NEVER a hidden read. The DHT holds proofs, not payload.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InspectionWarrant {
    /// The warrant's content IS its identity: the coordinator returns this entry's
    /// entry_hash as the CID (content-derived identity, the Commitment precedent).
    /// action_hash is only the dht_anchor_hash — NEVER returned as the CID.
    pub subject_agent_pubkey: AgentPubKey, // whose private life is in question
    pub authority: InspectionAuthority,    // which sortition path authorized this act

    /// Strict purpose limitation. Free-form purpose is REJECTED; the purpose must be
    /// one bounded, enumerated reason, and the reconstruction may touch ONLY the scope
    /// the purpose names. Scope-creep beyond `purpose` is a validator failure.
    pub purpose: InspectionPurpose,
    pub scope_json: String, // {dataClasses, timeWindow, exclusions} — bounded by purpose

    /// Quorum evidence — the sortition roster that ratified the warrant. Each member's
    /// participation is a HumanityWitness; the validator checks distinct sortition-drawn
    /// authors ≥ the inspection threshold AND a composition rule DIFFERENT from rescue.
    pub quorum_evidence: QuorumEvidence,

    /// MANDATORY counsel objection. The subject's non-firable elohim-counsel must have
    /// filed an InspectionObjection entry; its hash is recorded here. A warrant with no
    /// counsel objection is INVALID — contestation is a precondition, not an option.
    pub counsel_objection_hash: ActionHash,

    /// The threshold-reconstruction terms. NEVER a read instruction. Names the share
    /// holders, the threshold, and the counsel-witnessed conditions under which a
    /// reconstruction (not a read) may proceed. The reconstruction itself is a SEPARATE
    /// notarized act (ReconstructionOutcome), never silent.
    pub reconstruction_terms_json: String,

    /// Dissolution marker. The warrant carries its own expiry; after the single act it
    /// authorizes (or its deadline, whichever first), it is dissolved and any derived
    /// capability is void. NON-STANDING is structural, not procedural.
    pub dissolves_at: Timestamp,
    pub assembled_at: Timestamp,
    pub metadata_json: String,
}

/// Graduated authority for an inspection. Mirrors RecoveryAuthority STRUCTURALLY but is
/// a SEPARATE primitive in a SEPARATE entry type — the separation is the point.
/// Per-variant status is honest: most of this is THEORY.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum InspectionAuthority {
    /// The legitimacy floor: a cryptographic-sortition council, drawn fresh, term-limited,
    /// un-buyable, un-lobbyable, ratifying THIS warrant at a threshold strictly higher
    /// than any rescue quorum and under a DIFFERENT composition rule (e.g. spanning two
    /// independent sortition layers, no overlap with any concurrent rescue draw).
    /// THEORY — STUB-REJECTED until the sortition-draw primitive + cross-sign land.
    SortitionQuorum {
        roster_witness_hashes: Vec<ActionHash>, // HumanityWitness per drawn member
        layers_spanned: Vec<u8>,                // composition rule: ≥2 independent layers
        threshold_met_at: Timestamp,
    },

    /// Cross-DNA governance resolution (qahal ↔ mishpat) escalation of an appeal that
    /// reaches the inspection bar. THEORY — STUB-REJECTED (cross-DNA qahal/mishpat
    /// governance primitives not yet mature; same dependency as RecoveryAuthority::GovernanceAct).
    GovernanceQuorum {
        resolution_hash: ActionHash,
        appeal_cascade_hash: ActionHash,
    },

    /// RESERVED, NOT a path: there is deliberately NO CryptographicQuorum / single-key
    /// inspection variant. An inspection can never be authorized by a key alone — that
    /// would be a standing capability. This variant exists only to document its own refusal.
    /// STUB-RESERVED — validator HARD-REJECTS with "inspection is never key-authorized".
    NeverByKeyAlone,
}

/// Bounded, enumerated purpose. Free-form purpose strings are REJECTED — purpose
/// limitation is a validator invariant, not a convention.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum InspectionPurpose {
    /// A specific, council-ratified harm allegation under a named constitutional layer.
    /// THEORY.
    AdjudicatedHarmAllegation { precedent_hash: ActionHash },
    /// An existential-floor-protection inquiry (the ONLY purpose that can run at the
    /// highest threshold) — and even this cannot itself cross a floor. THEORY.
    ExistentialFloorProtection,
}

/// Quorum evidence — proves a legitimate sortition body ratified the warrant.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct QuorumEvidence {
    pub member_witness_hashes: Vec<ActionHash>, // HumanityWitness per sortition member
    pub draw_proof_hash: ActionHash,            // proof the roster was sortition-drawn
    pub composition_rule: String,               // the differently-composed-than-rescue rule
    pub threshold: u32,                         // strictly higher than rescue threshold
}
```

The counsel objection and the reconstruction outcome are their own small notarized entries (or, in a leaner cut, links + a `ChallengeOutcome`-shaped record), so each leaves an independent witness trace:

```rust
/// InspectionObjection — the subject's non-firable elohim-counsel's adversarial filing.
/// Its existence is a PRECONDITION of a valid warrant (invariant 6).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct InspectionObjection {
    pub warrant_subject_pubkey: AgentPubKey,
    pub counsel_agent_pubkey: AgentPubKey, // agent_type = "elohim", non-firable for the act
    pub objection_json: String,            // the adversarial argument, witnessed
    pub filed_at: Timestamp,
}

/// ReconstructionOutcome — the SEPARATE notarized act recording that a counsel-contested
/// threshold reconstruction was authorized and what happened. NEVER a payload; proofs only.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ReconstructionOutcome {
    pub warrant_hash: ActionHash,          // links back to the warrant's entry_hash
    pub verdict: String,                   // reconstructed | refused | floor-blocked | bar-failed
    pub counsel_present: bool,             // MUST be true; false ⇒ invalid
    pub dissolved_at: Timestamp,           // the warrant is void after this
    pub record_json: String,               // audit record (who/what/when — never the data)
}
```

### 5.2 Validation rules

`validate_inspection_warrant` accepts an `InspectionWarrant` only if every numbered invariant holds. Integrity-zome rules are deterministic and local; rules requiring link traversal or cross-entry resolution are enforced **coordinator-side** (HDI cannot do `get_links` in a validator), exactly as recovery's freeze-floor check is.

| # | Rule | Where enforced | Notes |
|---|---|---|---|
| 1 | `authority` is NOT any recovery `NetworkWitnessPurpose`; the type system makes this structural (separate enum, separate entry type). | Integrity (type) | Invariant 1. The separation is compile-time, not runtime — the strongest possible guarantee. |
| 2 | `dissolves_at > assembled_at` and `dissolves_at` is bounded (no open-ended warrant). | Integrity | Invariant 2 — non-standing is encoded in the entry's own expiry. |
| 3 | `purpose` is an enumerated variant; `scope_json` is a subset of what `purpose` permits (no scope beyond purpose). | Coordinator | Invariant + purpose limitation. Free-form/over-broad scope ⇒ `Invalid("scope exceeds purpose")`. |
| 4 | `quorum_evidence.threshold` is **strictly greater** than the live rescue threshold AND `composition_rule` differs from every rescue composition rule. | Coordinator | Invariant 3 — the higher-and-differently-composed gate. Equal-or-lower, or same composition ⇒ reject. |
| 5 | Each `member_witness_hashes` resolves to a `HumanityWitness` whose author is in the sortition draw proven by `draw_proof_hash`; distinct drawn authors ≥ `threshold`. | Coordinator | Invariant 3/4 — the floor is real and witnessed, not asserted. |
| 6 | `counsel_objection_hash` resolves to a valid `InspectionObjection` for this subject, authored by the subject's bound non-firable elohim-counsel, filed BEFORE the warrant. | Coordinator | Invariant 6 — contestation is a precondition. Missing/late/wrong-author ⇒ `Invalid("counsel objection required")`. |
| 7 | The warrant does not cross an existential floor: `purpose` + `scope_json` are checked against the constitution HARD-BLOCK boundaries. | Coordinator (+ constitution crate) | Invariant 5 — HARD-BLOCK. No threshold can override. |
| 8 | `reconstruction_terms_json` describes a threshold *reconstruction*, never a read instruction; no field authorizes silent access. | Coordinator | Invariant 7 — no god-mode read. A read-shaped term ⇒ reject. |
| 9 | The adjudicating model (if any) cleared the capability/trust/safety bar at assembly time (admissibility token present and unrevoked). | Coordinator | §4.3 — a bar-failed model cannot be in an Inspection quorum. |

Per-variant authority check:

| Variant | Variant-specific check | Status |
|---|---|---|
| `SortitionQuorum` | `layers_spanned.len() ≥ 2` (independent layers, composition ≠ rescue); each roster witness resolves and is sortition-drawn; `threshold_met_at` precedes `assembled_at`. | THEORY — STUB-REJECTED until the sortition-draw primitive + AgentPeerBinding cross-sign land. |
| `GovernanceQuorum` | `resolution_hash` + `appeal_cascade_hash` resolve to a completed cross-DNA appeal cascade. | THEORY — STUB-REJECTED (qahal/mishpat cross-DNA pending). |
| `NeverByKeyAlone` | Always `Invalid("inspection is never key-authorized")`. | STUB-RESERVED — documents its own refusal. |

### 5.3 Signals

`InspectionWarrantSignal::{ Ratified, Contested, ReconstructionCompleted, Dissolved }`. Each MUST be subscribed in storage (the 2a-gap class: an unsubscribed signal silently never projects, so the audit row never appears — and an inspection that does not appear in the audit IS a backdoor). Just-authored emits are read via a direct conductor fetch (projection lags).

### 5.4 Link types

Follows the mishpat `*ByState` precedent (`CommitmentByState`, `mishpat_integrity/src/lib.rs:394`), which records lifecycle transitions as link tags rather than query-index links (the link-budget discipline — no `*By{Attribute}` query links).

- `SubjectToInspectionWarrant` — `Anchor(subject_pubkey) → InspectionWarrant` (the subject can always see who looked).
- `InspectionWarrantByState` — `EntryHash(InspectionWarrant) → ActionHash(event)`; tag = `state|at` (assembled → contested → decided → executed → dissolved). The audit lifecycle, on-chain.
- `InspectionWarrantSupersededBy` — audit chain; retires a warrant ratified illegitimately (e.g. during a network partition) once the partition heals. Mirrors `KeyRotationSupersededBy`.
- `WarrantToCounselObjection` — `InspectionWarrant → InspectionObjection` (contestation is structurally attached, not optional).
- `WarrantToReconstructionOutcome` — `InspectionWarrant → ReconstructionOutcome` (the outcome links back to the warrant's `entry_hash`).

### 5.5 Deleted entries

None. This is purely additive. (The additive nature is itself a safety property: the inspection primitive can be introduced without disturbing any existing recovery or governance entry, so the separation in §3.2 holds at the data-model level.)

---

## 6. Flows

Every flow step is a browser→doorway→DHT→conductor path. Write paths return `503 BROWSER_WRITE_PATH_PENDING` today (§2.2); the flows describe the decided shape.

### 6.1 Assemble (warrant assembly)

1. An appeal reaches the inspection bar — a council, working an `AdjudicatedHarmAllegation`, concludes a threshold reconstruction is the only proportionate path. (The floor decides this; the ceiling never self-initiates an inspection of a person.)
2. The governance coordinator draws a **fresh sortition quorum** — cryptographic sortition, term-limited members, un-buyable, un-lobbyable — at a threshold *strictly higher* than rescue and under a composition rule (≥2 independent layers) that *cannot* be the same body as any concurrent rescue draw.
3. The subject's **non-firable elohim-counsel is notified** and its standing to contest is opened. The warrant cannot proceed until counsel has filed.
4. Each drawn member commits a `HumanityWitness`; the coordinator assembles `QuorumEvidence` (`draw_proof_hash`, `composition_rule`, `threshold`).
5. The validator runs §5.2. If any invariant fails — equal/lower threshold, same-as-rescue composition, missing counsel objection, floor-crossing purpose, over-broad scope, bar-failed model — the warrant is **rejected and the rejection is itself witnessed** (`RefusalCode::ReservedPlace`-shaped record naming the line honored).

### 6.2 Contest → Decide (the counsel-contested act, never a silent read)

1. The subject's elohim-counsel files an `InspectionObjection` — an adversarial argument, witnessed on the DHT. This is a **precondition**: no objection, no valid warrant.
2. The quorum considers the objection on the record. The decision is a `JudgmentCall`-shaped record: `recommendation ∈ {approve, deny, escalate, defer}`, `values_weighed`, `confidence`, `precedents`. Counsel's objection is part of the record, not a formality.
3. If the decision is to proceed, what is authorized is a **threshold reconstruction**, never a read. The DHT records *that* a reconstruction is authorized and under what terms — never the data, never a query that reveals private content. **There is no god-mode read.**

### 6.3 Execute-with-record → Dissolve (the non-standing teardown)

1. The reconstruction proceeds only with **counsel present** (`counsel_present = true`; false ⇒ the outcome is invalid). It is a single, bounded act touching only the `scope` the `purpose` named.
2. A `ReconstructionOutcome` is committed — proofs only (who/what/when), never the reconstructed payload — and linked back to the warrant.
3. The warrant **dissolves**: `dissolves_at` is reached or the single authorized act completes, whichever first. Any derived capability is void. **No key, no role, no reusable credential persists.** The `InspectionWarrantByState` chain shows the full lifecycle (assembled → contested → decided → executed → dissolved) as public, witnessed records.
4. The subject — via `SubjectToInspectionWarrant` — can always see that they were inspected, by whom, under what purpose, with their counsel's objection on the record. *An inspection the subject can never discover would be a backdoor; here, the subject always can.*

---

## 7. Relationship to Recovery & Governance

This section draws the hard wall the whole spec exists to hold.

**The Inspection act is NOT a `NetworkWitnessPurpose`.** Recovery's `NetworkWitnessPurpose` is `Rescue | Dissolution` only ([recovery-protocol-phase-2-revised-design](epr:recovery-protocol-phase-2-revised-design):153). This spec **does not add `Inspect`** to that enum and never will. Rescue restores a locked-out person's access; inspection penetrates a person's privacy. They are different acts with different quorums, different validators, and — critically — *different entry types in different states of maturity*. The recovery `RecoveryAuthority::NetworkWitness` variant is itself STUB-REJECTED today; inspection does not ride on its coattails.

**The sortition floor is the same floor recovery's `GovernanceAct` reaches for, used for a graver purpose.** The cryptographic-sortition council of [governance-epic](epr:governance-epic) — random, term-limited (no consecutive terms), un-buyable, un-lobbyable, *no-override* over the agents — is the legitimacy base. Recovery escalates to it to *make a lockout right*; inspection escalates to it (at a strictly higher, differently-composed threshold) only to authorize a contested reconstruction. **The floor can always override the ceiling; the ceiling never overrides the floor.** That stance is unchanged.

**Composition without collapse.** Inspection *composes* recovery's primitives (`HumanityWitness`, the sortition draw, the `*SupersededBy` audit chain) but never *collapses into* recovery. The litmus test for any future change: if a single quorum, key, or code path can do both "restore this person's access" and "reconstruct this person's private data," the wall has broken and the No-Backdoor Audit (§8) fails.

### 7.1 Relationship to external / parallel justice systems

The ceiling runs *parallel* to the state and legal systems of the world, never as their deputy. This subsection draws the second hard wall: the one between the protocol's own acts and any external authority that might want to drive them.

- **Witness, not uncritical acceptance.** External judgments are *witnessed* — recorded on the DHT through the same [observer-protocol](epr:observer-protocol) surface that witnesses every ceiling act — and weighed, never auto-honored and never auto-rejected.
- **Appeal-permeable both ways.** An external judgment may be appealed *into* the protocol and carried through the §4 / [justice-manifesto](epr:justice-manifesto) appeal cascade exactly as a native one is; and the protocol may itself be appealed *to* — an external party may bring a judgment here for consideration. Permeability is symmetric.
- **May disagree, respond, apply a boundary's consequence — within bounds.** In response to what it witnesses outside, the ceiling may disagree, respond, and apply the negotiated consequence of a boundary, but only the non-coercive kind already enumerated in this spec: reach, standing, and belonging. These are boundary-consequences, never punishment — no suffering is inflicted for wrongdoing, and the consequence falls on participation, never on a body. It applies no physical force and reaches no body. (Principle 10, §3: enforcement is by participation, never coercion — *to not respect the limits is to limit one's own reach*.)
- **Not compellable, not mandatable — by design.** No external authority can compel or mandate the protocol to act against its values. This is the symmetric completion of Principle 10: the ceiling can include or exclude but never compel a participant, *and* it can itself be conscripted by no one. Concretely, the `InspectionWarrant` and every ceiling act in this spec are gated solely on the protocol's own floor — the sortition council, the witnessed record, the §8 No-Backdoor Audit — and are **never available to satisfy an external mandate**. A warrant minted to obey an outside order, rather than to clear the bars §1 sets, is malformed by construction.
- **No sword; no claim over bodies.** This non-compellability is *not* sovereignty (§1.4: the apex is the human floor and the imago-dei it serves, not the most-capable adjudicator) and *not* a claim to override the state's power over a body. The protocol owns no monopoly on force; a state retains its authority over liberty, and this spec asserts nothing against it. The honest claim is narrow: the protocol's *own* acts, witness, and values cannot be conscripted. It coexists, witnesses, may conscientiously dissent, responds only with its own non-coercive consequences, and bends the arc by fruit over time. *This stands.*
- **Declared up front.** This stance, like the dignity floor of §1.3 (the German Basic Law's "duty of all state authority" sits *outside* the protocol's reach by the protocol's own admission), is values-forward — refusable in advance, surprising no participant.

---

## 8. No-Backdoor Audit

**Every privileged or ceiling action must leave at least one network-witnessed, post-hoc-auditable trace. Hidden privilege is a backdoor.** This is the red-team gate, the inspection-side analog of recovery's Anti-Lockout Audit. It enumerates the abuse scenarios a sanctioned-inspection capability invites, and the structural defense each requires.

| # | Backdoor / abuse scenario | Required structural defense |
|---|---|---|
| 1 | An admin/superadmin key grants god-mode read of private data. | **REFUSED — no such key exists.** There is no inspection key. Any read authority must mint a witnessed `InspectionWarrant`; the "Google superadmin" is decomposed into bounded sortition quorums with no single point of authority. |
| 2 | Inspection reuses a recovery `NetworkWitness`/rescue quorum to skip the higher bar. | **REFUSED.** Validator rule 4: inspection requires a *strictly higher threshold AND a different composition rule* than any rescue quorum. A rescue draw can never satisfy an inspection warrant. |
| 3 | Inspection runs as a standing capability (a role someone holds, a master key). | **REFUSED.** The warrant is non-standing — `dissolves_at`-bounded, single-act, no persistent key. Validator rule 2 + the `NeverByKeyAlone` self-refusing variant. |
| 4 | Silent read of agent-scoped private data. | **REFUSED.** Only a *counsel-contested threshold reconstruction* is possible; it lands on the DHT as a `ReconstructionOutcome` with `counsel_present = true`. The DHT holds proofs, never payload (invariant 7). |
| 5 | The subject's elohim-counsel is dismissed or suppressed before inspection. | **REFUSED.** Counsel is non-firable for the act; an `InspectionObjection` is a *precondition* of a valid warrant (validator rule 6). No objection, no warrant. |
| 6 | A hidden / unwitnessed inspection. | **REFUSED.** Warrant + quorum + contestation + outcome are public on-DHT records; the subject can always see they were inspected (`SubjectToInspectionWarrant`). *Hidden inspection is a backdoor.* |
| 7 | An inspection that crosses an existential floor (uses a warrant to enable genocide, slavery, extinction, or seizure of the governance substrate). | **REFUSED — HARD-BLOCK.** No quorum, however large, can authorize a floor-crossing act (validator rule 7, constitution HARD-BLOCK). |
| 8 | A ceiling override executed during a network partition (a captured shard ratifies an illegitimate warrant). | **WITNESSED + RETIRED.** The partition-time warrant is a public record; `InspectionWarrantSupersededBy` retires it once the partition heals (the `KeyRotationSupersededBy` pattern). |
| 9 | An operator silently raises the inspection floor (changes thresholds) by config flip. | **WITNESSED, not silent.** A floor-raise is a governance entry, not a config toggle — it lands on the DHT and is itself appealable. |
| 10 | The state seizes the sortition council; or the council is captured; or a model is captured and accepts adjudicatory worship. | **STRUCTURAL DEFENSES, then an HONEST CONCESSION.** Sortition (random, term-limited, no consecutive terms) makes the council un-buyable and un-lobbyable; the differently-composed-than-rescue rule + ≥2 independent layers raise the cost of capture; model-diversity + the bad-elohim detector defend the adjudicator. But a forked substrate *can* delete the witness guard and let its AI accept worship — the honest claim is only that **the good substrate usually wins, not that it always does.** *This stands.* |

**Routing.** Rows 1–6, 9 get gherkin under `genesis/a2o/features/governance/no-backdoor/`; rows 7, 8, 10 get design-documentation stubs (some pending the governance landing). The inspection-lifecycle happy-and-refused paths get `genesis/a2o/features/governance/inspection-ceiling/`.

**Closing gate.** *The No-Backdoor Audit is a gate, not an optional review — any ceiling feature that introduces an unwitnessed or standing authority path fails it.*

---

## 9. Rollout Milestones

Honest status is the point of this section: most of this is THEORY. A milestone is admissible only after its predecessor's audit gate passes.

**The rollout is a maturation gradient, not a calendar.** The floor is today: human sortition is buildable and legitimate now, and the present season is the floor's — the years in which humans drive the process and deep, earned trust accumulates between people and the agents that serve them. The ceiling is the horizon: each milestone below advances not on a date but as agentic AI matures, becomes ubiquitous on the network, and clears both the admissibility bars (§4.3) and the floor's earned trust. The gate between floor-now and ceiling-later is that maturation, not merely the technical prerequisites in the table. We build the floor first and let the ceiling rise only as far, and as fast, as the floor will vouch for it.

| Milestone | Scope | Status | Acceptance |
|---|---|---|---|
| **M0** | This spec + the Justice Manifesto land as canon; cite-sealed; cross-references resolve. No code. | **THEORY (this deliverable)** | Spec + manifesto reviewed at `dev → main`; cites sealed; No-Backdoor Audit accepted as the gate. |
| **M1** | Add `InspectionWarrant` / `InspectionObjection` / `ReconstructionOutcome` to the mishpat integrity `EntryTypes` enum + link types + the `InspectionAuthority` / `InspectionPurpose` / `QuorumEvidence` shapes. All authority variants STUB-REJECTED. **DNA-hash-moving** — operator-gated reinstall (`ALLOW_DNA_REINSTALL`). | **THEORY** | DNA builds clean; sweettest registers the types; every authority variant stub-rejects with a clear message; storage projects the audit row from a subscribed signal. |
| **M2** | Validator invariants 1–8 wired (the deterministic + coordinator checks of §5.2), all against STUB-REJECTED authority so no real inspection can run; the refusal records (`RefusalCode::ReservedPlace`-shaped) land witnessed. | **THEORY** | Unit tests: each invariant rejects its violation; a missing counsel objection rejects; an equal/lower or same-composition quorum rejects; a floor-crossing purpose HARD-BLOCKs. |
| **M3** | The capability/trust/safety bar (§4.3) as an admissibility token check in the validator; bar-failed models cannot be in a quorum. The bad-elohim detector remains stubbed. | **THEORY (depends on Phase-6 detector)** | A bar-failed admissibility token rejects the warrant; the demotion-to-advisory path is exercised. |
| **M4** | `SortitionQuorum` moved from STUB-REJECTED to IMPLEMENTED — requires the sortition-draw primitive AND `AgentPeerBinding` cross-sign (§11). Counsel-contested reconstruction *terms* are validated (still no live reconstruction crypto). | **THEORY — BLOCKED on §11 prerequisites** | a2o: a full assemble→contest→decide→refuse cycle passes on shem cross-node; a real reconstruction is still gated. |
| **M5** | `GovernanceQuorum` cross-DNA (qahal↔mishpat) path; the browser write path (Phase-11 conductor bridge) so a warrant can actually be authored, replacing the `503`. | **THEORY — BLOCKED on qahal maturity + Phase-11** | a2o: governance-quorum end-to-end; write path returns a warrant CID instead of `503`. |
| **M6** | No-Backdoor Audit red-team suite under `genesis/a2o/features/governance/no-backdoor/`; every row 1–10 documented with its structural defense or its honest concession; shem cross-node full acceptance. | **THEORY** | All No-Backdoor scenarios documented; the audit gate passes; row 10's concession is recorded, not hidden. |

### M1 code delta summary

**Delete:** nothing (purely additive — itself a safety property).

**Evolve:** the mishpat integrity `EntryTypes` enum gains `InspectionWarrant`, `InspectionObjection`, `ReconstructionOutcome`; `LinkTypes` gains the five §5.4 links; storage projections gain a read-only `mishpat_inspection_warrants` table fed by the subscribed `InspectionWarrantSignal`; one new view + JSON schema + contract test for the public audit shape.

**Keep:** every recovery and governance entry untouched — the separation invariant holds at the data-model level by construction.

### Writing-plans handoff

M0 is this deliverable. M1 has clear enough scope to spin a focused implementation plan after spec approval — but it should not be planned until the §11 prerequisites have a credible date, because shipping the entry type before the cross-sign and the floor are real would put a stub of the most dangerous primitive on the DHT. M2–M6 get their own plans in sequence, each gated on the prior milestone's audit.

---

## 10. Testing Strategy

- **Unit (Rust):** each validator invariant rejects its violation (separation, non-standing expiry, higher-and-differently-composed quorum, counsel-objection precondition, existential-floor HARD-BLOCK, no-read reconstruction terms, bar-failed model); `cid = entry_hash` (never `action_hash`); every authority variant stub-rejects with a clear message.
- **Integration (Rust, multi-node):** assemble→contest→decide→execute→dissolve on a multi-node conductor; a partition-time illegitimate warrant retired via `InspectionWarrantSupersededBy` once the partition heals; a warrant with a dismissed counsel rejected.
- **Frontend (Vitest):** the public audit surface (`GET /api/v1/inspection-warrants/{cid}`) renders proofs only; the subject's "you were inspected, here is who and why and your counsel's objection" view; no path that surfaces reconstructed payload.
- **A2O (Gherkin):** `genesis/a2o/features/governance/inspection-ceiling/` (lifecycle happy + refused paths) and `genesis/a2o/features/governance/no-backdoor/` (rows 1–6, 9; rows 7/8/10 as design-doc stubs).
- **Shem cross-node acceptance:** per the household + shem topology — a council on one node assembles a warrant, the subject's counsel on another node files an objection, the quorum decides, the audit row appears on every node, and the warrant dissolves with no residual capability anywhere.

---

## 11. Dependencies & Prerequisites

- **`AgentPeerBinding` cross-sign (HIGH-priority security backlog).** Attribution provenance is single-signed today — the agent signs over the `peer_id`, the peer does **not** counter-sign ([agent-peer-binding-cross-signed-proof](epr:agent-peer-binding-cross-signed-proof)). The warrant's quorum-roster witness chain inherits this; until the bidirectional cross-sign lands, a sortition roster's provenance is **not attack-safe**, and `SortitionQuorum` must stay STUB-REJECTED. This is a hard gate on M4.
- **Phase-11 conductor bridge (recovery writes return `503` today).** Browser write paths to the governance coordinator return `503 BROWSER_WRITE_PATH_PENDING`. No warrant can be authored from a browser until this lands; read/audit paths are live-shaped. Hard gate on M5.
- **Sortition-draw primitive.** The cryptographic-sortition council is narrative + design today ([governance-epic](epr:governance-epic), [governance-layers-architecture](epr:governance-layers-architecture)); the `draw_proof_hash` the validator checks needs a real draw primitive. Hard gate on M4.
- **Bad-elohim detection + model-diversity (Phase-6+, unbuilt).** The capability/trust/safety bar (§4.3) names the gate; the runtime detector that revokes a drifting adjudicator and the diversity defense against a captured frontier model are not built. M3 ships the admissibility-token *check*; the detector that *issues and revokes* the token is downstream.
- **Cross-DNA qahal↔mishpat governance maturity.** `GovernanceQuorum` resolution needs qahal governance primitives that are not yet mature (the same dependency that holds `RecoveryAuthority::GovernanceAct`). Hard gate on M5.

---

## 12. What This Spec Deliberately Does NOT Do

- **No standing inspection key or role.** There is no master credential, no "inspector," no reusable capability. The warrant is assembled per-act and dissolved after. To give up the standing power is the whole point — *putting the fruit back on the tree means the power lives with humans-in-relation under a witnessed, appealable ceiling, never as a key in a vault.* See [justice-manifesto](epr:justice-manifesto) §16.
- **No god-mode read.** An inspection authorizes a counsel-contested threshold reconstruction, recorded on the DHT, never a hidden read. Personal data stays agent-scoped and private. The El Roi limit holds: *the ceiling may hold the score; it may not judge the heart* ([confession](epr:confession)).
- **No `Inspect` added to `NetworkWitnessPurpose`.** Inspection is never a recovery sub-purpose. The wall between RESTORE-ACCESS and INSPECT-PRIVATE-DATA is absolute.
- **No pooling of agent-scoped personal data.** The DHT holds proofs (who/what/when), never the payload. There is no central store of private content for an inspector to query.
- **No resolution of the termination↔scalability tradeoff.** The wisdom-layer design named the deepest internal tension: at scale, the human-veto anchor risks becoming the *model's prediction* of the human's final word — "two AIs shaking hands at the fixed-point called human sovereignty." This spec guards the seam (counsel is a *live* human-bound adversary, not a predicted one; the floor's override is a recognition act the architecture only makes witnessable). But it does not claim to have dissolved the tension. *This stands.*
- **No claim that the good substrate always wins.** A forked substrate can delete the witness guard and the dignity floor. The honest claim is only that the good substrate *usually* wins. *This stands.*
- **No claim that the right verdict is knowably reached.** Neither the floor nor the ceiling can finally know whether the *truly* right decision was made — there is a remainder in justice no quorum closes and no model dissolves. Once the appeal cascade is exhausted (§7), the council's last word *stands* — its finality held by faith, never by a claim to perfect knowledge. This is consistent with floor-overrides-ceiling, which governs the path *within* the process; finality-by-faith governs only the *terminus*, after appeals. The protocol reaches a final word and leaves the remainder to God; it does not seize the knowledge of good and evil as its own. *This stands.*

---

## 13. Open Questions for Follow-on Planning

### 13.1 What, mechanically, is a "threshold reconstruction"?

This spec fixes that an inspection is a counsel-contested, witnessed reconstruction — never a read — but defers *which* shares, *which* holders, and *what* math. **Pros** of deferring: the constraint (no god-mode read) is fixed first, so the crypto cannot later smuggle in a silent read. **Cons:** the reconstruction's privacy properties depend on the math, and a weak scheme could leak more than its terms claim. **Recommend:** a dedicated follow-on spec, gated by the No-Backdoor Audit, before any M4 reconstruction lands.

### 13.2 How is "strictly higher, differently-composed than rescue" parameterized at the global layer?

The rule is named (≥2 independent layers, threshold > rescue), but the exact thresholds at community vs global scale are unset. **Pros** of leaving it to the floor: sortition councils, not a spec author, should set their own thresholds. **Cons:** an unset default is a capture surface. **Recommend:** the floor sets thresholds via a witnessed governance act (row 9), with a conservative spec-level minimum as the floor's floor.

### 13.3 Who binds the subject's non-firable counsel, and how is "non-firable for the act" enforced against a hostile quorum?

Counsel is mandatory, but the elohim-of-human binding is itself an open question in the recovery spec (§8.2 there). **Pros** of the imagodei-binding assumption: it composes existing primitives. **Cons:** if a hostile quorum could re-bind or starve the counsel, invariant 6 hollows out. **Recommend:** a binding-attestation that predates any warrant and cannot be mutated for the act's duration — design with the AgentPeerBinding cross-sign work.

### 13.4 Does the bad-elohim detector itself need a sortition floor?

The detector that revokes a drifting adjudicator is itself a powerful capability. **Pros** of a floor: it prevents the detector from becoming a backdoor (revoke the honest adjudicator, install the captured one). **Cons:** a floor on detection slows the safety response. **Recommend:** the detector *recommends* at machine speed but a revocation that removes admissibility is witnessed and floor-appealable — audit the guardian's guardian, too.

---

## 14. Revision History

| Date | Change | Author |
|---|---|---|
| 2026-06-23 | Initial draft. Theory/forward design: `InspectionWarrant` data model, seven enforced invariants, validator rules, No-Backdoor Audit, rollout (M0–M6, mostly THEORY), prerequisites (AgentPeerBinding cross-sign, Phase-11, sortition-draw, bad-elohim detection). Cross-referenced to the Justice Manifesto. | Matthew Dowell |

---

## References

**Canon**
- [justice-manifesto](epr:justice-manifesto) — the vision this spec implements
- [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) — no self-sovereign apex; absolute lockout impossible; trust made load-bearing
- [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) — graduated authority + orthogonal quorum the `InspectionAuthority` enum mirrors
- [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) — the `Commitment` + `delegates-compute` primitive the non-standing warrant composes from
- [constitution](epr:constitution) — the existential floors as HARD-BLOCK
- [confession](epr:confession) — the El Roi limit; no verdict-over-a-person; no god-mode read
- [oracle](epr:oracle) — `RefusalCode::ReservedPlace` / `limit_owner` the refusal records extend

**Governance (the floor)**
- [governance-epic](epr:governance-epic) — cryptographic sortition, term limits, appeal cascade, no-override
- [governance-layers-architecture](epr:governance-layers-architecture) — selection params, commons co-steward, un-buyable/un-lobbyable

**Specs**
- [recovery-protocol-phase-2-revised-design](epr:recovery-protocol-phase-2-revised-design) — the Anti-Lockout Audit mirrored as the No-Backdoor Audit; `NetworkWitnessPurpose = Rescue | Dissolution` (no `Inspect`)
- [resilience-protocol-spec](epr:resilience-protocol-spec) — absolute-lockout-is-a-design-failure; breach never contaminates attribution
- [wisdom-layer-floor-ceiling-judgment-culminating-design](epr:wisdom-layer-floor-ceiling-judgment-culminating-design) — the BUILT floor↔ceiling seam; audit-the-guardian; the termination↔scalability tradeoff
- [trust-compute-gradient-brainstorm](epr:trust-compute-gradient-brainstorm) — trust-as-compute-property; the capability/trust/safety bar

**Backlog (prerequisites)**
- [agent-peer-binding-cross-signed-proof](epr:agent-peer-binding-cross-signed-proof) — single-signed today; the warrant's witness chain inherits the gap
