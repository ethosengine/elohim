# Cradle-to-Grave Capability Gradient

> **Canon status:** Substrate philosophy. Read [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) and [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) first.

---

## §1 — Why this canon exists

A protocol that serves only crypto-literate adults in their prime is not serving the species. Real humans pass through stages where their capacity to act on their own behalf rises, peaks, and falls. A child cannot wield cryptographic keys meaningfully. A person with intellectual or developmental disabilities (IDD) may have a different shape of agency than the protocol's default assumes. A senior with progressive cognitive decline must remain a voiced participant in their own life even as agency migrates to stewards. A person at end-of-life leaves a legacy that the protocol must witness, transition, and honor.

The Elohim Protocol expresses these gradients explicitly. The substrate provides one primitive — the `delegates-compute` REA Commitment — that instantiates differently for each life stage. The same shape serves a child under guardianship, an adult delegating CI authority, a senior under graduated stewardship, an estate executor. The dignity comes from the consistency.

This canon names the gradient. Other docs (Z.D, the recovery spec, the capability-profile element contract) implement specific rows.

---

## §2 — The Life-Stage Transition Map

| Life stage | Capacity description | Provider (steward) | Recipient (stewardee) | Recovery quorum | Capability surface |
| --- | --- | --- | --- | --- | --- |
| **Ward (child, IDD adult under guardianship)** | Mediated agency; guardian co-authors | Legal guardian | Ward's agentic compute | Guardian + intimate circle (3-5 trusted) | Observer + co-author; elohim-agent authored with guardian counter-signature |
| **Adolescent (probation/transition)** | Escalating agency with oversight | Parent + adolescent (co-steward) | Adolescent's agents | Parent + intimate circle | Graduated surface; lamad-attested capability levels; elohim-mediated escalation |
| **Adult (full agent)** | Independent authority within stewarded relationships | Self | Own delegated agents (CI, AI, household compute, etc.) | Intimate circle (peers of equal standing) | Full capability surface; standing computed from affinity + attestation + history |
| **Senior (diminishing capacity)** | Declining agency; voice-retention focus | Self (pre-arranged) + successor | Senior's agents | Adult child + spouse + trusted peer + elohim-counsel | Observer + voice: preference guards, witness-veto, advisory-only participation in governance |
| **End-of-life** | Legating agency; archive + witness | Designated executor (named in succession Commitment) | Legacy agents (estate, archive) | Executor + household + community witness | Read-only access; executor carries authority; legacy access via household attestation |
| **Deploy-svc-agent (this protocol-level instance)** | Bounded compute authority delegated by operator-steward | Operator-steward | Deploy-svc-agent | Operator + intimate (see Z.D §2 Q5) | Operational; bounded by `delegates-compute` Commitment |

The bottom row is included because the substrate primitive treats CI agents under operator stewardship with the same shape as wards under guardianship. The protocol does not have a special "machine agent" category; it has stewarded compute resources, all bounded by Commitments, all auditable, all recoverable.

---

## §3 — Graduated Recovery Authority (4-Layer Stack)

When a stewardee's standing must be restored — lost device, compromised key, capacity transition, contested attestation — the protocol provides four graduated recovery layers. Each is faster and lower-friction than the next; each is more authoritative and slower to assemble.

### Layer 1 — Intimate-circle quorum (fastest, lowest friction)

Witnesses from emergency-access relationships (`KeyStewardship`-named contacts). Threshold = `ceil(count/2)+1`. Shamir shares released by trusted contacts' elohim-agents when they confirm the recovery is genuine.

This is the everyday recovery path. A grandmother who loses her phone calls her three kids; they confirm the recovery; she's restored. The substrate uses Shamir to make the cryptographic accounting clean, but the *user experience* is "her people said yes."

### Layer 2 — Extended community consensus (mediated)

When intimate quorum cannot assemble (kids unreachable, no recent emergency-access contacts named), the recovery escalates to the extended community: trusted, familiar, and reach-permitted contacts. Voting is intimacy-weighted (intimate=3x, trusted=2x, familiar=1x). Requires more witnesses, takes more time, but does not require the intimate circle to be reachable.

### Layer 3 — Governance act (slow, deliberate)

When community consensus cannot assemble, qahal governance can authorize recovery via a constitutional `StewardshipGrant` plus explicit resolution. This is the path used for contested recoveries (e.g., a person attempting to recover an account whose existing steward circle has been infiltrated). Slow, deliberate, transparent.

### Layer 4 — Global elohim witness / network consent (slowest, last-resort)

When even governance is captured or unreachable, the network's top-layer elohims can consent to restoration. This is the absolute-lockout-prevention layer. Expensive (requires coordinated attestation across multiple network witnesses), slow (no SLA), but always available as a final guarantee.

**Absolute lockout is impossible by design.** Layer 4's existence is the protocol's commitment to that property.

Cryptographic primitives (Shamir, threshold sigs, hardware-rooted attestation) plug into any layer to accelerate it. Their absence never prevents recovery; it just slows it.

This is `project_graduated_recovery_authority` made canonical.

---

## §4 — The Elohim Mediation Layer

Elohim agents are not optional in this protocol. They are the substrate that makes the gradient legible to humans. Three architectural roles:

### Counsel (standing role)

Every human has an elohim-agent. That agent has first-class standing to defend the human's interests on the protocol — including against the human's current-moment preferences if those preferences appear to reflect duress.

The counsel role is non-firable during attack scenarios. A spouse under coercion cannot tell their elohim "you're fired, let me do this" and have the elohim comply — the counsel role is constituted by the substrate, not by the moment-to-moment preferences of the human it represents.

This sounds paternalistic; in context, it is the opposite. The counsel role exists precisely because the human's moment-to-moment preferences may not reflect their best self. A pre-committed defense layer is what keeps the human's actual values represented when they are under attack.

`project_elohim_as_counsel` made canonical.

### Specialist subagents (ephemeral roles)

Counsel does not act monolithically. It spawns specialist subagents with focused context:

- **Defender** — spawned on attack detection; reads imagodei profile deeply; authors defensive DHT entries (anomaly markers, freeze requests, counter-challenges).
- **Gate-discernment** — evaluates recovery authorization with relationship context. Distinguishes "your sister confirming your recovery" from "an attacker who has compromised your sister's session."
- **Advocate** — represents the human in governance disputes. Argues their position.
- **Steward** — makes content/resource stewardship decisions on the human's behalf.

Each specialist has a manifest declaring inputs, outputs, and disclosure rules. Specialists are stateless between invocations — counsel is the standing role they spawn from.

`project_elohim_subagent_specialists` made canonical.

### Commons-elohim co-steward (collective role)

Every Qahal has an autonomous elohim that co-stewards alongside human stewards, representing the **commons interest** distinct from individuals. It:

- Reflects what the collective cannot directly voice.
- Holds custody of the commons share.
- Speaks in governance councils.
- Mediates disputes.
- Cannot be silenced (structurally embedded at genesis).

For abstract sensemaking collectives (Tier 3 — the dissolved bureaucracies of the post-elohim era), the commons-elohim is *primary*, not shadow. This ensures collective values are voiced even when individual stewards are distracted or captured.

`project_commons_elohim_co_steward` made canonical.

---

## §5 — The "Google Superadmin for Stewardship" Pattern

In a Google Workspace, the superadmin can:

- Reset passwords.
- Suspend accounts.
- Recover access.
- Audit activity.
- Configure delegation.

The Elohim Protocol provides an equivalent for stewardship — but with a critical difference: the "superadmin" is a **graduated-trust circle holding the constituting `delegates-compute` Commitments**, not a single point of authority.

Each capability has its own bound:

| Capability | Holder | Recovery quorum |
| --- | --- | --- |
| Identity recovery | Self | Intimate-circle quorum (Layer 1) |
| Capacity-transition adjustment (senior → adult-child) | Self (pre-arranged) + successor | Adult child + spouse + elohim-counsel |
| Compute revocation (deploy-svc-agent) | Operator-steward | Any steward with active `delegates-compute` on the recipient |
| Audit access | Steward-tier viewers | Anyone with `attested-observer` standing |
| Bound modification (changing rate limits, reach ceilings) | Original steward | Original steward only OR quorum + waiting period |
| Identity revocation (last-resort) | Network witness (Layer 4) | Multi-network elohim consent |

The pattern decomposes "superadmin" into many small bounded authorities, each held by a different party, each recoverable through a different quorum. No single party can do everything; many parties together can ensure nothing essential is permanently lost.

---

## §6 — Capability Gradient UI Expression

The protocol expects elohim-elements (the Lit custom-element library) to express capability levels visually. Six disclosure lenses are defined in [`capability-profile-element-contract-design.md`](../superpowers/specs/2026-05-20-capability-profile-element-contract-design.md):

`minimal` → `simple` → `standard` → `detail` → `debug` → `trace`

Climbing disclosure never hides, only reveals. The lens is gated by the viewer's standing, not by a global config.

**The gradient is per-viewer per-surface, not per-content.** The same EPR (e.g., a project-epr Commitment for a doorway hosting agreement) renders differently to:

- A commons viewer (sees: "this doorway hosts these surfaces").
- An adult-account viewer with no stewardship relationship (sees: same as commons).
- A reach-permitted viewer (sees: standard lens — title, state, recent activity).
- A steward-tier viewer (operator of the doorway, sees: detail lens — full Commitment, bounds, rate-usage chart, pending acknowledgements).
- The operator's elohim-agent in counsel mode (sees: trace lens — every event, every signal, every signature).

For the cradle-to-grave specifically:

- **A ward viewing their own surface** sees content + the guardian's signature on the constituting Commitment. They understand they are stewarded.
- **A guardian viewing the ward's surface** sees standard lens for their relationship plus an "act on behalf" affordance that requires guardian counter-signature.
- **A senior with diminishing capacity viewing their surface** sees content + an "elohim-agent suggests" affordance for actions that exceed their current capacity. The suggestion is rendered as "your daughter Jane recommends" (the elohim-counsel's framing).

This is how the protocol's capability gradient becomes legible to humans across life-stage capacities.

### §6.1 — Rendering-layer realization (Capability Profile + Element Contract)

The gradient above is *realized at render time* by the elohim-elements capability-profile element contract (compacted here from `2026-05-20-capability-profile-element-contract-design.md` + its plan + the protocol-omni-component plan; raw bodies retire to git). The rendering layer exposes **two render-time observables**:

- **Capability Profile** — what the *viewer* can do with this surface (the six disclosure lenses above, gated by standing).
- **Content Certainty** — how settled/notarized the *content itself* is.

A custom element declares its render-time needs in a **`capabilityContract` CEM block**, and a build-time **analyzer** validates the contract against the element's actual usage. Each surface passes **three all-or-nothing precondition gates** — there is **no "designed-but-not-yet-a11y" half-state cell**; a surface is either gated-ready across all three or it is not rendered. The `ProtocolOmniComponent` and `EprNavContextView` are **Category-C operational** (reconstructed, never notarized — the p2p-design-gate flag is pre-cleared for both).

**Load-bearing constraints to preserve (a planner will want to flip these — don't):**

- **Sabbath-as-default inversion** — the rendering default is `stimulus: still`, `textuality: textual`; motion is **opt-up** and **OS-capped**. This is deliberate; preserve it.
- **Lens narrowing is a one-way downward ratchet** — climbing disclosure reveals more; narrowing never silently re-widens.
- **Protocol vocabulary is preserved verbatim across locales** — translation localizes prose, never the protocol terms.
- **Deferred sub-projects #3–#7 are real and unbuilt.** In particular **#3 (steward-lock persistence) IS a real DHT entry** and MUST re-invoke the `p2p-design-gate` skill before any design — it is not a Category-C operational shortcut.

The doorway-side `renderCapability` (whether a *doorway* can server-render) is a **distinct** observable from the viewer-side Capability Profile here — see the [doorway-SSR runtime seed](../content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md); do not merge the two.

---

## §7 — Cradle-to-Grave Stewarded Compute

The protocol does not distinguish "AI agentic compute" from "human compute" structurally. Both are bounded by `delegates-compute` Commitments. Both are recoverable through graduated authority. Both must operate within reach gates and reciprocity expectations.

What this means concretely for AI agent deployment in the protocol:

- A child's AI agent (e.g., a learning companion) is mediated by guardian-steward: the agent's `delegates-compute` Commitment is signed by the guardian, scope is age-appropriate, reach ceiling is set to `intimate` or lower, rate limits prevent spam.
- An IDD adult's AI agent operates under graduated trust circle delegation. The adult is the named recipient; the circle is the provider; bounds are calibrated to the adult's current capacity and renewed periodically as capacity is reassessed.
- A senior with diminishing capacity has gradual stewardship migration. Pre-arranged Commitments specify the transition path: at capacity-level A, the senior provides their own delegations; at capacity-level B, adult-child countersigns; at capacity-level C, the steward circle provides; at capacity-level D, only the executor.
- An end-of-life human's legacy agents are bounded by testamentary Commitments: scope is read-only-archive, reach is at the level the human committed to, the executor is the named successor. The agents continue to participate in the protocol on the deceased human's behalf for a defined witness period.

Same primitive throughout. Same audit trail. Same recovery quorums. Same elohim-counsel watching for compromise.

This is what mature AI deployment in the network looks like: AI agents are first-class participants but always-bounded, always-stewarded, never sovereign.

---

## §8 — Capacity Loss / Transition Flows

Narrative examples to ground the canon.

### Flow 1: Adolescent graduating to full agency

At 14, Maya's guardian-Commitment to her elohim-agent is renewed with broader scope (adolescent tier). At 17, she begins co-stewarding: every new delegation requires her guardian's countersignature, but she initiates the bounds. At 18, she publishes a new `delegates-compute` Commitment with her guardian as provider and herself as recipient — explicitly graduating to adult-tier authority. Her guardian's Commitment continues to exist (revocable, observable) but her new Commitment becomes the primary authority.

The substrate enforces graduation as a chain of explicit Commitments, not a flag flip.

### Flow 2: Adult traveling, temporarily offline

Sarah is traveling abroad with limited connectivity. She has named her intimate-circle quorum (her spouse, her sister, her best friend) as her emergency-access contacts via `KeyStewardship` entries. While she is offline, the intimate quorum holds her recovery authority. If her phone is stolen during the trip and a new login attempt is made from her hotel, the intimate quorum can validate the attempt is genuine and authorize the rotation. Sarah's elohim-counsel watches for anomaly patterns; if the recovery request looks coerced, it slows the rotation and escalates.

### Flow 3: Senior with diagnosed cognitive decline

Robert is 78 and recently diagnosed with mild cognitive impairment. His pre-arranged stewardship Commitment names his adult child as successor with capacity-level-B authority. As Robert's capacity progresses to level C, the Commitment's automatic provisions migrate primary authority to his adult child + spouse + elohim-counsel. Robert retains voice: he can still publish FeedbackSignals, advise in governance, veto specific transactions. His elohim-counsel surfaces every consequential decision to him in `simple` lens and to his adult child in `detail` lens. The substrate enforces capacity transitions as graduated explicit acts, not invisible escalations.

### Flow 4: End-of-life

Helen has named her daughter as executor in her succession Commitment. When she dies, her household publishes a `transition-stewardship` EconomicEvent referencing the succession Commitment. The substrate witnesses the transition: her daughter inherits authority over Helen's legacy agents and content. Helen's content remains accessible at the reach she committed to (her family photos at intimate reach, her published essays at commons). Her elohim-counsel continues to operate in archive mode for a defined witness period (per the succession Commitment), allowing her household to access her perspective on legacy questions before the elohim is retired.

The protocol witnesses death not as a failure mode but as a transition to be honored.

---

## §9 — Implementation Status

What's implemented (or implementing in the current sprint):

| Row of §2 table | Status | Reference |
| --- | --- | --- |
| Adult (full agent) — recovery primitives | Spec landed, partial implementation | `2026-04-22-recovery-protocol-phase-2-revised-design.md` |
| Deploy-svc-agent (protocol-level instance) | Spec landed; implementation in current Z.D sprint | `2026-05-25-stagespablob-substrate-correct-deploy.md`; this canon's plan |
| Capability-profile element contract | Spec landed, primitive surface available | `2026-05-20-capability-profile-element-contract-design.md` |
| Ward / IDD / senior / end-of-life flows | Canon written (this doc); concrete specs pending | This document is the entry point |
| Elohim-counsel + specialists | Memory anchored; spec extraction pending | `project_elohim_as_counsel`, `project_elohim_subagent_specialists` |
| Commons-elohim co-steward | Memory anchored; spec extraction pending | `project_commons_elohim_co_steward` |

**What's next** (after Z.D Phase 1+2 lands):

1. Extract concrete specs from this canon for ward / IDD / senior / end-of-life flows.
2. Implement the corresponding action discriminators (`act-on-behalf`, `transition-stewardship`, etc.) per the REA primitive's §8 pattern.
3. Extend elohim-elements with the gradient-aware components (counsel suggestions, capacity-aware affordances).
4. Update lamad content to reflect cradle-to-grave shape (curriculum on stewardship, recovery, capacity transitions).

---

## §10 — References

### Canon (this directory)

- [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) — the foundational lens.
- [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) — the substrate primitive that every row of §2 instantiates.

### Specs

- `genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md` — the graduated authority recovery substrate.
- `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` — the disclosure-lens surface for capability gradients.
- `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` — Z.D, the deploy-svc-agent instance.

### Memory anchors (agent-side)

- `project_graduated_recovery_authority` — 4-layer recovery stack.
- `project_socially_derived_security` — cryptographic primitives as accelerators.
- `project_recovery_grandma_standard` — the UX bar.
- `project_elohim_as_counsel` — counsel standing role.
- `project_elohim_subagent_specialists` — specialist subagent pattern.
- `project_commons_elohim_co_steward` — commons representation.
- `project_qahal_graduated_capability_surface` — community capability gradient.
- `project_rea_compute_commitment_primitive` — the substrate primitive.
- `project_household_living_core_lived_contrast_diffusion` — household as the protocol's living core (where most life-stage transitions happen).

---

## §11 — Closing Note

The gradient is the point. A protocol that supports only adults in their prime supports nobody. A protocol that meets a person where they are — at any life stage, with any capacity, in any relationship — is a protocol that serves the species.

Every spec author touching identity, authority, recovery, or capability must ask: which row of the §2 table does this surface serve? If the answer is "only row 3," the spec is incomplete. Extend it.

The grandma standard, the ward standard, the senior standard, the executor standard, the deploy-svc-agent standard — these are not separate standards. They are the same primitive (`delegates-compute` Commitment with bounds and reciprocity) instantiated at different scopes. The substrate's discipline is its uniformity. The dignity is the consistency.
