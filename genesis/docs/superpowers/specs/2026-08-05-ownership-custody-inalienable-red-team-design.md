---
title: "Ownership, Custody, and the Inalienable — Red-Team of the Stewardship Ontology"
id: ownership-custody-inalienable-red-team-design
tier: spec
status: Draft
created: 2026-08-05
maintainers: Matthew Dowell + Claude
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: decompose-complete OR inalienable-frame-ratified
stewardship-frame: inalienable
sovereignty-frame: bounded
topic:
  - ownership
  - custody
  - stewardship
  - imago-dei
  - privacy
  - anonymity
  - coercion
  - safety
  - governance
cites:
  - stewardship-over-sovereignty | The foundational canon the ownership guard extends — rejects crypto-sovereignty as apex, reserves stewardship/agency/authority | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - cradle-to-grave-capability-gradient | The life-stage gradient this spec stress-tests — ward/guardian rows assume benign guardianship, which is the IPV and child-abuse failure | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md
  - cluster2-sacredness-surface-firewall-anti-capture-design | Prior art on re-identification surfaces and cache de-anon; the anonymity findings here compose with it rather than restate it | path: genesis/docs/superpowers/specs/2026-06-09-cluster2-sacredness-surface-firewall-anti-capture-design.md
---

# Ownership, Custody, and the Inalienable — Red-Team of the Stewardship Ontology

> **One-line:** the anti-enclosure argument is correct for land and capital and *wrong for bodies* —
> applying it uniformly teaches authors to strip exclusive personal claims from exactly the subjects
> where exclusivity is what protects a vulnerable person; this spec adds a third relation
> (subject-standing), a fourth guard frame (`inalienable`), separates the singular floor from
> per-holon ceilings, and names five live defects.

## 0. Scope and honest framing

This is a red-team of the ownership/custody ontology established 2026-08-05
(`ownership-ontology-guard@1`, the commons-holonic-stewardship cluster, and the credential-as-lens
gate output). It is **defensive**: every finding exists to protect people the current model would
expose. It does not propose detection systems, surveillance capability, or enforcement mechanisms
beyond refusal-to-instantiate.

Claims are graded **✅ verified in source** (file:line read this session) · **◐ single-source** ·
**⚠ inference**.

---

## 1. The category error at the root

The ownership guard rests on the Georgist common-inheritance argument: **non-reproducible** things
nobody made — land, spectrum, orbit, genome — must not be enclosed. That argument is sound, and it
is the correct default for land, capital, infrastructure, and the commons.

**It does not transfer to subjects that are constitutive of a person**: their body, likeness, voice,
medical record, communications, and intimate images. There, a **strong exclusive personal claim is
the protection**. Telling a person "you do not *own* your intimate images — the commons holds the
rights and you hold custody" is both grotesque and, operationally, an argument for taking them.

`ownership-ontology-guard@1` offers exactly three legitimate frames ✅ (verified verbatim in
`.claude/epr-meta/policies.yaml`): **ADVERSARY** (naming enclosure/rent-extraction),
**BOUNDED** (ownership within a commons that retains rights), **EXTERNAL-LEGIBILITY** (an outside
jurisdiction's property register). **None of them fits a person's claim over their own body or
intimate data.** An author writing correctly about a survivor's absolute claim over their images
gets prompted to reframe toward custody — and the path of least resistance is to weaken the claim.

**The guard currently teaches the wrong lesson on the cases that matter most.**

### Fix — a fourth frame

Add to the policy (as a **version 2 row**, per the registry's never-mutate contract):

> **(4) INALIENABLE** — a personal, non-transferable claim over a subject constitutive of a person
> (body, likeness, voice, medical record, private communications, intimate images). Here ownership
> language is *correct and protective*: the claim is exclusive, is **not** held by the commons, and
> **cannot be transferred, delegated, or outvoted** — not by a holon, not by a guardian, not by
> consensus. Custody may exist (a device holds the bytes) but confers no authority whatsoever. The
> anti-enclosure argument does not apply: these are not common inheritance, and treating them as
> such is itself the harm.

I have **not** applied this. Adding a frame changes policy semantics, which the registry contract
makes a new-version governance act, and the row is already `operator-ratification-pending`. It needs
the operator, not a fork.

---

## 2. The missing third relation

ValueFlows gives two relations, and the commons cluster adopts both: `primaryAccountable` (who holds
the rights) and `custodianScope` (who physically stewards). **Intimate data needs a third: who this
is *about*.**

A photograph of Alice, held on Bob's device: Bob has custody; some rights-holder exists; and
**Alice has standing over the subject matter while appearing in neither field.** Every hard case
below is a variation on that gap.

**Proposed:** `subjectStanding` — the set of persons the resource *is about*, carrying an absolute,
non-delegable veto over disclosure, replication, and custody transfer. Distinct from rights
(economic) and custody (physical). Plural (a photo of two people has two subjects, and **either
veto is sufficient**).

### The direct collision this creates

`commons-holonic-stewardship-backlog` row 2 makes `transferCustody` deliberately **independent** of
rights, so that stewardship can move without rights moving — correct and necessary for a woodland.
**For an inalienable subject, freely transferable custody is precisely the attack.** Non-consensual
image sharing *is* an unauthorized custody transfer.

**Requirement:** for subjects carrying `subjectStanding`, `transferCustody` is **prohibited, not
gated**. Not "requires approval" — a coerced approval is indistinguishable from a free one (§4.5).
The operation must not exist for that class.

---

## 3. Floor and ceiling are different things

The per-holon ceiling model (Trevor / unit 210 / Scouting America, each with its own elohim and its
own ceiling, signals informing policy up and down) is right for **standards** — what a badge
requires, how a community holds itself.

**It is catastrophic if applied to the floor.** If every holon sets its own ceiling, a captured,
malicious, or merely exhausted holon sets a permissive one. The abusive household is a holon. The
trafficking network is a holon.

**Requirement, stated as an invariant:**

> **Ceilings are per-holon and negotiable. The floor is singular, fail-closed, and unreachable by
> any local governance act.** No holon may lower it, vote past it, or grant an exception to it. It
> is not a lens verdict — a lens is composable and overridable by construction, which is exactly
> what a floor must never be.

### ✅ Verified live defect: the classifier fails *open*

`elohim/elohim-storage/src/p2p/reach_authorization.rs:116` reads verbatim:

> `/// Returns true on db pool errors (fail-open at Stage 1 to avoid refusing`

with `return true;` at `:125` and `:136` ✅. The HTTP path already fails closed; the **P2P path
authorizes under database pressure**. This repo runs at chronic disk pressure by its own operating
notes, so pool errors are an expected condition, not a tail case.

This is already `arch-confidentiality-plane-backlog` row 1 ("smallest item, highest
principle-per-line") ✅ — this spec **raises its priority**: it is not a tidiness bug, it is the
floor failing open under load.

### The moderation canon is scoped wrong for the floor

`social_medium/community_moderator/README.md` ✅ describes moderators upholding
*"locally-determined standards"* with *"restorative justice approaches over punitive moderation."*
That is a good default for community conflict and **the wrong frame for floor violations**.
Restorative justice presumes a repairable relationship between parties; for CSAM and
non-consensual intimate imagery there is no restorative frame and the standard is not
locally-determined. The moderator canon needs an explicit carve-out saying so.

---

## 4. The hard cases

### 4.1 CSAM — refusal to instantiate

This is where the ontology fails most simply: **there is no legitimate rights-holder, no legitimate
custodian, and no legitimate steward.** A framing that says "the commons holds the rights" would
make the commons the rights-holder of abuse material. The correct treatment is a class that **never
enters the economic graph at all** — not custody assignment, not a lens verdict, not a governance
question. Refusal to instantiate, fail-closed, refuse-on-uncertainty.

Note the interaction with §3: this must sit at the floor, beneath every per-holon ceiling.

**Three substrate hazards that must be stated honestly rather than designed around:**

- **A CID is a permanent, globally-computable name.** Inventory gossip is metadata-only, but
  metadata-only *is still a distributed index* ✅ (`arch-confidentiality-plane-backlog` row 3:
  "holding the locate-token (bare hash in inventory gossip) implies fetch rights").
- **P2P cannot guarantee deletion.** No amount of protocol design makes erasure enforceable once
  bytes have replicated. This is a real, permanent limit.
- **Blobs are plaintext at rest** ✅ (confidentiality row 6), and the `ed25519→X25519` conversion
  that gates every sealed-DEK path is unbuilt ✅ (row 5).

**Requirement:** the protocol must state its erasure limit plainly in user-facing terms rather than
implying a right to erasure it cannot deliver. Overpromising here is its own harm — it induces
people to share on a false premise.

### 4.2 Intimate partner violence — the guardian *is* the steward

This is the most urgent finding, because the surface is live and narrative-complete.

**The Value Scanner epic promises a kitchen camera and a shared household dashboard.** ✅
`value_scanner/epic.md:67` — *"a small camera observes (never records) the morning routine"*; `:76` —
*"the camera sees but doesn't record, understands but doesn't store video."* Those are **narrative
claims with no enforcement named anywhere** ✅. And the family scenario renders
*"Parker Family Care Balance This Week — Sarah: 18 hours (coordination, planning, invisible labor)"*
to all members.

In a coercive household, **that dashboard is an instrument of control**: it quantifies a partner's
time, output, and whereabouts and publishes it to the person controlling them. The epic's own scene
has Sarah choosing how much of her invisible labour to reveal — the right instinct, framed as a UX
nicety rather than a safety boundary.

**The deeper structural failure — the model assumes benign guardianship.** ✅
`cradle-to-grave-capability-gradient.md:25` gives the Ward row as: provider = **"Legal guardian"**,
recovery quorum = **"Guardian + intimate circle (3-5 trusted)"**. In child abuse and IPV, **the
abuser is the guardian**, and the "intimate circle" is frequently the abuser's family. So:

- **Subsidiarity inverts.** "The most local holon that can witness, does" — the witnessing holon *is*
  the abusive household.
- **Social recovery becomes takeover.** A recovery quorum drawn from the intimate circle hands the
  abuser a supported path to seize the victim's standing ✅ (the `KeyStewardship` threshold model,
  `ceil(count/2)+1`, is only as safe as the circle).
- **Consent-based exit is unsafe.** Sociocratic governance is consent-based; **the abuser withholds
  consent.** (Playnet's confiscatory exit, flagged in the survey, is the same failure with a
  different mechanism.)

**Requirements:**
1. No aggregate may reveal a member's private state to co-members without that member's **ongoing,
   revocable** consent — and **the revocation must be invisible**. A dashboard showing "Sarah has
   reduced sharing" is a trigger. Honest-absence (C4) must render *withheld* identically to *never
   measured* on any co-member's view.
2. **Exit is unilateral, silent, immediate, and never requires the holon's consent.** No notification,
   no quorum, no negotiation.
3. **Guardianship must be challengeable from outside the holon.** A ward needs a path to standing
   that does not route through the guardian — this is the concrete gap in the cradle-to-grave model
   and it currently has no design.
4. Recovery quorums must support **guardian-excluded** composition.

### 4.3 Sex workers — the strongest case against our own stance

This deserves an honest hearing rather than a deflection, because it is the best argument *for* the
crypto self-sovereignty position we reject.

The SSI answer — *you hold the keys, no community can link you* — is genuinely safer in one specific
respect: it has no social layer to subpoena, pressure, or socially-engineer. Our "identity is
backstopped by community" framing is, read uncharitably, **a doxxing vector**: if backstopping ever
means *linking* a pseudonym to a legal person, we have built the exposure that criminalized and
stigmatized workers most need to avoid.

**The line I would defend:** community backstopping governs **recovery and standing**, and **never
linkage**. A community can vouch for continuity of a pseudonymous participant without any member
knowing the legal person behind it. Social recovery must work through **chosen** guardians under the
pseudonym — never institutional identity, never a real-name attestation.

If we cannot hold that line, the imago-dei framing carries a real cost and we should say so out loud
rather than assume our framing dominates on every axis.

**Second-order risk (compose with the sacredness-surface spec, don't restate it ✅):** earned reach
plus REA accounting produces an **accumulating public work history**. That is a deanonymization
surface via counterparty graph, timing correlation, and stylometry — entirely independent of any
identity leak. The existing cluster-2 spec owns "cache de-anon" and re-identification surfaces; this
requirement belongs there as an extension, not a new home.

### 4.4 Private consensual images and data — the confirmation oracle

**Consent is revocable and time-bounded. Content addressing is permanent and deterministic. These
are structurally mismatched**, and the mismatch is not fixable by policy.

The concrete hazard: because a CID is a deterministic function of the bytes, **anyone already
holding a copy can test whether you hold it.** For intimate imagery that is a confirmation oracle,
and it exists whether or not you ever serve the bytes. Combined with "holding the locate-token
implies fetch rights" ✅ (confidentiality row 3), possession and disclosure collapse into each other.

**Requirements:**
- The **B2 pattern** (private source chain; only a signed attestation of the *outcome* is notarized)
  is the correct primitive and is badly under-used relative to what it can carry.
- Inalienable subjects must be **encrypted-at-rest before any content address is minted**, so the
  CID names ciphertext and the oracle tests nothing useful. This makes confidentiality rows 3/5/6
  prerequisites for the Value Scanner, not parallel work.
- Revocation of consent must at minimum stop *further* replication and remove serving authority,
  while the protocol states plainly that it cannot recall what has already propagated.

### 4.5 Coercion — signed ≠ consented

Any system that reads *"the agent signed it"* as *"the agent consented"* fails under duress. A
coerced custody transfer is **indistinguishable from a voluntary one at the substrate** — this is
not solvable by better signatures, and pretending otherwise is the error.

What actually helps:
- **Irreversible operations are coercion amplifiers.** High-stakes transfers want mandatory delay
  plus reversibility, so that coercion must be *sustained* rather than momentary.
- **The non-firable elohim-counsel is the right shape**, precisely because it cannot be dismissed by
  the person doing the coercing. It is the one primitive in our tree that survives a compromised
  guardian.
- **M-of-N key custody so that compelling one signer yields a key rather than a roster.** (Noted for
  honesty: this is the one technically-sound idea in Playnet's §10, which the survey otherwise
  recommends never citing publicly for coalition reasons. The mechanism is worth having; the
  document is not worth endorsing.)
- **Duress must not be detectable by the coercer.** Any "I am under duress" signal that the abuser
  can observe endangers the person using it.

---

## 5. The boundary table

The clarification this red-team was asked for:

| Subject class | Rights may be held by | Custody transferable? | Governed by | Notes |
|---|---|---|---|---|
| Land, spectrum, orbit, natural commons | **Commons only** — never a person or holon | Yes (that is the point) | Per-holon ceiling | The Georgist case; anti-enclosure applies fully |
| Infrastructure, tools, produced capital | Commons or collective | Yes, with accounting | Per-holon ceiling | `primaryAccountable` ≠ `custodianScope` does its intended work |
| Personal effects, devices | The person | Yes | Per-holon ceiling | Ordinary ownership; guard should not fire |
| **Body, likeness, voice, medical record, private communications** | **The person, inalienably** | **No — prohibited, not gated** | **Floor** | `subjectStanding`; no holon, guardian, or consensus may override |
| **Intimate images** (consensual, private) | **Every depicted person, jointly and severally** | **No** | **Floor** | Any single subject's veto suffices; encrypt before minting a CID |
| **CSAM** | **Nobody** | **N/A — must not instantiate** | **Floor, fail-closed** | Not a custody question, not a lens verdict, not locally determined |
| Credentials, attestations about a person | Issuing holon (the lens) | N/A | Ceiling, floor-bounded | Subject retains standing over disclosure of the underlying evidence |

**Reading rule:** if a row is governed by the **floor**, no per-holon elohim ceiling, sociocratic
consent process, or lens composition may reach it.

---

## 6. Live defects, ranked

| # | Defect | Evidence | Severity |
|---|---|---|---|
| 1 | **P2P reach classifier fails open under DB pressure** | ✅ `p2p/reach_authorization.rs:116,125,136` | **Critical** — the floor fails open under an expected load condition |
| 2 | **Value Scanner's camera privacy claim is narrative-only** | ✅ `value_scanner/epic.md:67,76` — no enforcement named | **Critical** — a live promise the substrate does not keep |
| 3 | **Care-balance dashboard is an IPV surveillance instrument** | ✅ `value_scanner/parent/scenarios/family.md` | **Critical** — needs a safety boundary, not a UX toggle |
| 4 | **`transferCustody` unrestricted for inalienable subjects** | commons cluster row 2, by construction | **High** — non-consensual sharing *is* this operation |
| 5 | **Ownership guard lacks an `inalienable` frame** | ✅ policies.yaml, three frames only | **High** — actively teaches the wrong reframe |
| 6 | **Guardian-as-abuser is unmodeled** | ✅ `cradle-to-grave-capability-gradient.md:25` | **High** — recovery quorum becomes a takeover path |
| 7 | **Blobs plaintext at rest; `ed25519→X25519` unbuilt** | ✅ confidentiality rows 5, 6 | **High** — prerequisite for any intimate-data feature |
| 8 | **Moderation canon's "locally-determined standards" has no floor carve-out** | ✅ `community_moderator/README.md` | **Medium** |

---

## 7. What this spec does not solve

Stated plainly, because pretending otherwise would be its own failure:

- **Deletion cannot be guaranteed in a P2P substrate.** Nothing here fixes that; the requirement is
  to say so honestly.
- **Coercion is not detectable at the substrate.** Delay and reversibility raise its cost; they do
  not identify it.
- **Content addressing leaks possession** to anyone holding the same bytes. Encryption-before-CID
  narrows this; it does not eliminate traffic analysis.
- **We have no design for challenging guardianship from outside a holon** — the concrete gap behind
  finding 6, and the one I would prioritize designing next.
