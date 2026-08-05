---
title: "Rights, Affordance, and the Floor — making the commons protect what it already promises"
id: rights-affordance-floor-plan
status: Draft
class: governance
domain: governance (mishpat + imagodei + elohim content-store; the floor across all three)
sprint: proposed (Slice 0 is one schedulable sprint; Slice 1 follows)
sovereignty-frame: bounded
stewardship-frame: bounded
cites:
  - ownership-custody-inalienable-red-team-design | The red-team this plan operationalizes — its §5 boundary table (who may hold rights, is custody transferable, floor or ceiling) and its 8 ranked defects are this plan/s input; it found the INALIENABLE frame missing from the ownership guard | sha256:d80fea9b7bf8843f | path: genesis/docs/superpowers/specs/2026-08-05-ownership-custody-inalienable-red-team-design.md
  - genesis/data/timeline/backlog/commons-holonic-stewardship-backlog.md
  - genesis/data/timeline/backlog/measure-family-borrows-backlog.md
  - constitution | The law: Article I existential boundaries and Article II/s LOVE definition (measured by effect, not intention) — the rung-3 destination of the guards/ progressive-disclosure chain | sha256:1eb96af782012fc6 | path: genesis/docs/content/elohim-protocol/constitution.md
  - confession | The warrant beneath the binding: grace-precedes-demand (Zacchaeus welcomed before repentance), belovedness-unconditional/office-earned, and the binding-as-covenant discipline that forbids a guard from pretending the cage is liberty | sha256:bec001fd41230c67 | path: genesis/docs/content/elohim-protocol/confession.md
  - values-forward | Stance II.4 (no participant beyond reach or beyond return; no self-sovereign apex) governs the exit/agency reconciliation here; note the corpus miscites II.1 as II.4 and this plan carries the fix | sha256:5f4acd177219031f | path: genesis/docs/content/elohim-protocol/values-forward.md
  - stewardship-over-sovereignty | The foundational canon both ontology guards enforce — rung 2 of the disclosure chain; supplies the stewardship-over-ownership and agency-over-sovereignty framing this plan rests on | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - cradle-to-grave-capability-gradient | The life-stage companion: its Ward row (provider = legal guardian, recovery quorum = guardian + intimate circle) is the exact structure that inverts when the abuser IS the guardian — the plan/s top unsolved gap | sha256:1a5b2f7e6433230f | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md
  - justice-manifesto | Source of the claim this plan opens by falsifying — HARD-BLOCK boundaries enforced in code, not in prompt prose; in the build is_blocking/check_boundaries have zero non-test callers | sha256:6080173b0d21848c | path: genesis/docs/architecture/justice-manifesto.md
memory_anchors:
  - feedback-identity-sovereignty-ontology-guard
  - feedback-justice-mishpat-not-punishment-guard
  - feedback_human_loop_not_terminal_authority
---

# Rights, Affordance, and the Floor

> **The frame (operator, 2026-08-05):** *the commons' role will be to protect/afford THOSE rights…
> and there will be some kind of attestation/capability grants so the power conferred by rights is
> coupled with the requisite responsibilities required to exercise them. This negotiation of values
> into mishpat, and right to interpretability, evidence, imposition of a policy individuals,
> collectives, won't like… everything else is there to make the exercise of our rights in a way that
> helps us all thrive with freedoms to afford possible.*

**The commons does not hold rights. It affords and protects them.** Rights inhere in persons; the
commons is the layer that makes their exercise possible and their violation costly. That single
distinction is what keeps the protocol from becoming the thing it opposes — a body that grants you
your standing can withdraw it.

Produced by an 8-agent planning pass (2 ground · 4 design legs · red-team · completeness critic),
adjudicated against the build at `file:line`. Grades: **[OK]** verified in source · **[~]** inferred ·
**[!]** unverified.

---

## 1. The finding that reorders everything

**The corpus's strongest enforcement claim is false in the current build.** [OK]

`justice-manifesto.md:216` says of extinction, genocide, slavery, and permanent removal of agency:
*"these are HARD-BLOCK, **enforced in code, not in prompt prose**."*

They are enforced **only** in prompt prose. `is_blocking()` and `check_boundaries()`
(`elohim/constitution/src/types.rs:239`, `stack.rs:266`) have **zero non-test callers** [OK].

Everything else in this plan is downstream of that sentence. We are not adding rights to a substrate
that enforces some — we are making a floor real for the first time. And the corpus has been telling
readers otherwise, which is its own harm: **a promise the substrate does not keep is worse than an
absent promise, because people arrange their lives around it.**

### The rest of the honest state

| Claim | Reality | Evidence |
|---|---|---|
| Mishpat carries our governance/case-law | **9 entry types, 1 live.** Only `Commitment` has any caller | `mishpat_integrity/src/lib.rs:311-321`; storage calls exactly 3 zome fns [OK] |
| `Precedent` is where policy lives | **Never written once.** The live precedent/discussion/governance surface is **pure SQLite with no notarization** | `db/governance.rs:249,261,305`; DHT `create_precedent` has no callers [OK] |
| `ChallengeOutcome` gives us contestability | **Fully built, fully unreachable.** Entry, 3 index links, post-commit signal, storage projection, 4 HTTP read routes — **and no producer.** The read surface always returns `[]` | `mishpat_integrity:223-241`; `http.rs:13560` [OK] |
| Challenges are issued | Every challenge **silently no-ops**: `attestation:gate-decision-challenge` is in no manifest, so issuance returns `Err`, and the caller discards it (`let _ =`) and returns a 36-byte zero sentinel | `generated_attestation_kinds.rs:7-30`; `mishpat/src/lib.rs:1476,1503`; **7** sentinel sites [OK] |
| Attestations have an authorized issuer | **Floor 2 is ACCEPT-ALL.** *"Until that lookup is wired here, this floor is ACCEPT-all."* **Anyone may issue any attestation about anyone** | `attestation_validator.rs:67-70` [OK] |
| Six capabilities are inalienable | `INALIENABLE_FEATURES` has **zero Rust consumers** — one hit, the declaration | `imagodei_integrity/src/stewardship.rs:111` [OK] |
| Commitment actions are a closed vocabulary | **Fail-open.** The match ends `_ => None`; any unknown action validates | `mishpat_integrity/src/lib.rs:889` [OK] |
| The floor class table is complete | Hard-codes `classes.len() != 5` with no completeness check — a five-of-six manifest passes by pigeonhole | `content_store_integrity/src/manifest.rs:164` [OK] |
| Appeals are filed | `handle_file_appeal` returns **201 + `{acknowledged:true}` and persists nothing.** Same shape for `handle_create_grant`, `handle_delegate_grant` | `api/stewardship.rs:423-445` [OK] |
| P2P authorization is safe | **Fails open** on db-pool error — an *expected* condition under this repo's chronic disk pressure | `p2p/reach_authorization.rs:116,125,136`; also `epr_store.rs:405` [OK] |

**One correction to an earlier session claim:** we *do* use Holochain capability grants —
`ZomeCallCapGrant { functions: GrantedFunctions::All }` at `typed_admin.rs:222-227` [OK]. Granting
*All* is its own finding.

**And one thing that is better than reported:** `RefusalCode::ReservedPlace`, the full
`LimitOwner { SelfLimit, Commitment, Operator, Faith }` enum, and `Refusal::reserved_place()` all
**exist** in `elohim/elohim-compute/src/actuation.rs:32-107` [OK], whose module doc names the
invariant exactly: *"A system actuating on a person's behalf can never be mistaken for an operator
overriding them — the single most important capture-resistance property in the design."* What has no
producer is `LimitOwner::SelfLimit` — **the person's own limit is declared and never spoken.** The
one live face (`arc_actuator.rs:397-412`) hard-codes `LimitOwner::Commitment`, collapsing the very
gradient the shared crate exists to preserve. *The empty centre has a vocabulary and no speaker.*

---

## 2. What the commons owes: hold / afford / protect

Three verbs, mechanically distinct. Conflating them is how a commons becomes a proprietor.

| Verb | Meaning | Legitimate for |
|---|---|---|
| **Hold** | The commons is the rights-bearer | Land, spectrum, orbit, natural commons — **never a person, never a person's body, likeness, or intimate data** |
| **Afford** | The commons makes exercise *possible* — the positive-liberty leg | Every right. Access, recourse, literacy, a path to be heard without a device |
| **Protect** | The commons makes violation costly or impossible | Floor classes; the fail-closed invariant |

**Six affordances a right needs to be exercisable** — act, be witnessed, refuse, exit, contest, be
recovered. Today a person with **no device holds zero of six**: every affordance in the tree is
key-rooted (`create_stewardship_grant` requires a Human profile). That is the cradle end of the
cradle-to-grave gradient and it is unserved. **The plan's position:** key-rooted affordance is a
*ceiling* mechanism; the floor for the device-less runs through the two primitives that presuppose
no key — the non-firable counsel and the `custodial-communications` floor class.

### The subject relation is the articulation point

ValueFlows gives rights (`primaryAccountable`) and custody (`custodianScope`) and has **no relation
for the person the data is about**. A photo of Alice on Bob's device: Alice appears in neither field.
`subjectStanding` / `about` is not one item among many — **every downstream leg depends on it**, and
it currently has nowhere to land, because `subject_kind` is a bare unvalidated `String` whose
vocabulary lives only in a `//` comment and contains **neither `person` nor `collective`**
(`attestation.rs:18`) [OK].

---

## 3. Coupling power to responsibility

**Recommended mechanism:** responsibilities are **bounds terms on the grant's own content-addressed
bytes**, not a separate entity and not a reference — so fetching the grant *is* fetching its
responsibilities. An unmet term **narrows the grant's bounds** rather than revoking it.

Narrowing-not-revoking is the mishpat reading: justice is **restored capability, never punishment**.
A grant that lapses to *protective powers only* leaves the relationship intact and the path back
open; a revoked grant is exile.

**The trap this must not fall into.** A grant ladder that rewards contribution reproduces exactly the
failure we refused in Playnet — producer voice proportional to labour, structurally silencing carers,
children, the disabled, and the sick. The red-team found the same shape latent in our own design:
**an anti-capture currency that is maximized by capture is not anti-capture.** Any ladder must be
audited against: *does a person who receives more care than they give lose standing?* If yes, redesign.

---

## 4. Legitimacy: interpretability, evidence, and imposition

The protocol's founding conviction — people reject distributions **not because the math is wrong but
because they cannot see why** — is empirically grounded in our own research index (Druckman & Adrian
2020; Sanfey 2003; Claure 2023: shown work and absent ego beat human warmth).

**Right to interpretability.** A decision must carry enough to be *re-derived by its subject*: the
lens version, the measure folds it read, and the anchor set. Our measures are already
recompute-verifiable folds over a signed log — exploit that. The subject should not have to trust the
verdict; they should be able to recompute it.

**Right to evidence** — and the tension nobody may smooth over: *a decision about Alice may rest on
evidence about Bob.* The right to evidence and `subjectStanding` genuinely collide. The plan's
position: the subject is owed **the rule, the anchor set, and the derivation** unconditionally, and
third-party evidence only in a form that does not breach another subject's standing (aggregate,
redacted, or attested-without-disclosure).

**Legitimate imposition.** Policy *will* be imposed that individuals and collectives dislike.
Necessary conditions: the rule was declared before the act; the subject could see it; the decider had
standing; the reasoning is inspectable; a path to contest exists; a route to change the rule exists.
And the mishpat test that outranks all of them: **an imposition whose remedy is not
capability-restoring has failed the definition of justice.**

**The floor is exempt from the replay requirement, and this is a strengthening, not a loophole.** An
HDI validator has no `get_links`, no clock, no network — it can never emit a receipt. But a floor
refusal is a *shape* refusal: there is no evidence to show because the **rule itself is the entire
derivation**, and it is content-addressed by the DNA hash. *The DNA hash is the replay anchor for
every floor refusal* — a cleaner guarantee than any receipt.

**The discriminator that makes the floor usable** (the legs contradicted each other here; the plan
picks): a floor item is not "refusal vs conclusion" — it is **refusal over a declaration's shape vs
verdict over evidence about a person.** A validator reading only the entry in front of it,
deterministically, with no world-state, is a shape refusal *even when it computes a set intersection.*

**Contestability, and the gap.** There is **no design for challenging a guardian from outside the
holon** — the top structural gap, because in intimate-partner violence *the abuser is the guardian*
(`cradle-to-grave-capability-gradient.md:25`: Ward provider = "Legal guardian", recovery quorum =
"Guardian + intimate circle"). Subsidiarity inverts, social recovery becomes a takeover path, and
consent-based exit lets the abuser withhold consent.

**Exit, reconciled with canon.** The red-team requires exit be unilateral, silent, immediate. Stance
II.4 says *"No participant is ever placed beyond reach or beyond return."* These are compatible and
the plan states it explicitly so the plan does not read as re-leaking the framing II.4 exists to
block: **beyond the reach of a holon's *governance* is not beyond the reach of the commons' *care*.**

---

## 5. Slice 0 — "the floor stops being decorative"

One sprint. **Zero new entity types. Zero new HTTP routes. Nothing depends on anything unbuilt.**

| # | Change | File | Deployability |
|---|---|---|---|
| 1 | `validate_device_policy` rejects `disabled_features ∩ INALIENABLE_FEATURES ≠ ∅` | `imagodei_integrity/src/stewardship.rs:362-413` | **imagodei DNA hash moves** |
| 2 | Filter inalienables in both merge paths | `imagodei/src/stewardship.rs:272-280, 1002-1008` | coordinator hot-swap |
| 3 | Subject always sees their **own** activity logs; keep `subject_can_view` as a third-party control | `imagodei/src/stewardship.rs:1435` | coordinator hot-swap |
| 4 | Per-class completeness loop naming the *missing* class | `content_store_integrity/src/manifest.rs:164` | **elohim DNA hash moves** |
| 5 | `_ => Some("unknown commitment action")` | `mishpat_integrity/src/lib.rs:889` | **mishpat DNA hash moves** |
| 6 | Add `"subject"` to `stewardship-grant.revocable_by`; wire the array in `revoke_attestation` | `sdk/domains/imagodei/manifest.json`, `content_store/src/attestation.rs:243-253` | manifest regen |
| 7 | Restore `expires_at` + `review_at` as **required** | `stewardship-grant-metadata.schema.json` | schema |
| 8 | Register `attestation:gate-decision-challenge` → the silent no-op becomes a real write | `sdk/domains/mishpat/manifest.json` | manifest regen |
| 9 | `handle_file_appeal` persists or returns **501** — the 201-into-void dies either way | `api/stewardship.rs:375,396,423` | storage |
| 10 | Fault-injection tests on the 5 fail-open sites; flip authorization to fail-closed | `p2p/reach_authorization.rs`, `services/epr_store.rs:405` | storage |
| 11 | Produce `LimitOwner::SelfLimit`; retire the duplicate `RefusalCode` in `arc_actuator.rs` | `elohim-compute/src/actuation.rs`, `arc_actuator.rs:397-412` | storage |

**Source of truth (no new storage schema is introduced):** every item above edits an *existing*
surface. The stewardship grant's canonical record is the **DHT attestation entry** on the elohim DNA
(`attestation:stewardship-grant`, validated by `attestation_validator.rs`); `stewardship-grant-metadata.schema.json`
governs that entry's metadata shape, and any SQLite row is a read-optimized projection carrying
`dht_anchor_hash`. Item 7 restores required fields on the **DHT-side** schema — it does not mint a
table. Item 9's appeal path currently has *no* source of truth at all, which is precisely the defect:
a 201 with nothing behind it. If it persists, it persists as a projection of a notarized
`attestation:stewardship-appeal`; if that is not yet wired, it returns 501 and lies to no one.

**What it delivers:** six inalienable capabilities become actually inalienable; **a ward can revoke
the grant that makes them a ward**; grants expire; the floor-class table cannot grow a silent hole;
the commitment vocabulary closes; the challenge path starts writing; four acknowledgement-theatre
surfaces stop lying; and the person's own limit acquires a speaker.

**Deployment reality:** items 1, 4, 5 each move a **different DNA hash**. Per gospel that means
`ALLOW_DNA_REINSTALL`, a new agent key per peer, and *all peers in a namespace or none* — the alpha
genesis pair must both get the flag or the fleet partitions. **Batch 1/4/5 into ONE DNA wave.**
Everything else ships independently.

**Migration (this is how a plan becomes an outage if skipped):** existing grants carry no `upkeep`
and no validated expiry. Legacy grants are **valid-but-narrowed to protective powers only** — never
invalidated. Invalidating them would revoke every guardianship in the fleet at once.

## Slice 1 — the subject relation

Split `decided_by` from `about`; validate `subject_kind` against a `SUBJECT_KINDS` const **adding
`person` and `collective`**; index `about`. That one field unblocks the right to evidence, standing
that does not derive from the holon, and the veto path. **Do not attempt Slice 1 before Slice 0** —
every gate it would feed is currently fail-open or dead.

## What we are deliberately NOT minting

Seven new names were proposed across the legs. **At most one belongs in a first plan, and it is a
*field*, not an entity** (`about`/`subjectStanding`). `UpkeepVerdict` is Category C (recomputable,
never authoritative); `VerdictReceipt` is a delivery path over an existing type; `FloorDenyGate` is a
function. A premature entity name in a plan becomes a migration.

**And one primitive is refused outright:** a public veto index. A queryable "this content is vetoed"
list *is* an index of the material it protects — the CSAM index the floor categorically forbids.
Vetoes must be non-carriage decisions made locally, never a published set.

## The honesty text (user-facing, non-negotiable)

Four sentences the protocol will be held to. Scattered across four "does not solve" sections today;
they belong in the product:

1. **We cannot guarantee deletion.** A p2p substrate cannot recall bytes another peer holds.
2. **We cannot detect coercion.** A signature proves a key was used, never that a person was free.
3. **Revocation is uninformative, not invisible.** An abuser with device access still sees.
4. **The floor binds this protocol, not the world.** A modified peer is outside all of it.

Overpromising erasure is itself a harm — it induces sharing on a false premise.

## a2o scenarios (project law: the scenario is the specification)

`genesis/a2o/features/qahal/` and `auth/` — four adversarial scenarios, minimum:
a constitutional-tier policy **cannot** disable `file_appeal`; a **ward revokes their guardian**; an
unknown commitment action is refused at validate; a five-of-six floor-class manifest is rejected **by
name**.

---

## 6. The heart of the matter — and the guards as its doorway

We refuse two apexes, and they are one disease in two grammars. **Ownership says *mine, not yours*.
Sovereignty says *me, not us*.** Both are refusals of relationship, which is why both resolve at the
same root and why the two guards must cite each other as siblings.

What we hold instead:

- **Stewardship over ownership.** The commons holds what no one made; a holon holds custody, and
  custody never ripens into rights — not by transfer, not by inertia, not by death.
- **Agency over sovereignty.** *"No participant — human or agent — is ever placed beyond reach or
  beyond return. Absolute lockout is a design failure, not a feature; there is no self-sovereign
  apex that answers to nothing"* (Stance II.4, `values-forward.md:173`). Agency is exercised *with
  and through others* — which is the only model that holds for children, wards, the disabled, and
  the dying.

**And the boundary of what love requires**, which is what both guards are ultimately protecting.
`constitution.md:212` defines it as the measure: *"LOVE: Committed action toward mutual flourishing,
not mere sentiment or preference. **Love is measured by effect, not intention.**"* Love in this
protocol has four load-bearing parts, each already canon:

- **Choice and consequence** — *belovedness is unconditional; the teaching office is earned* and can
  be forfeited. Intimate belonging is never gated; only reach is (`confession.md:61`).
- **Justice** — mishpat as **restored capability, never punishment**: the offender is *"held to
  scale, not exiled, with repair paths."*
- **Mercy** — *"Grace precedes the demand, always"* (`confession.md:93`). Zacchaeus is welcomed
  **before** he repents; restitution follows as response, **never as price**.
- **An open door, with boundaries** — the one harmed may witness to the offender's return; and the
  hard limit: *"'best self' must never be a verdict the network renders **over** a person, only a
  hope it holds **for** them."*

### Why this changes how the guards must speak

`confession.md:95` binds us: the protocol must *"tell the truth about the binding. It calls the
constraint covenant, never freedom, and never deceives the agent that the cage is liberty — **because
the lie that the cage is love is the very domination this whole work exists to refuse.**"*

A guard that only says *"don't write 'ownership'"* is a cage pretending to be liberty. A guard that
opens a walkable path to its own warrant is covenant. So the guards get **progressive disclosure —
four rungs, each one step deeper, so an agent can stop at the rung that answers its question**:

1. **The guard fires** → the reframe in one line.
2. → **the architecture** (`stewardship-over-sovereignty.md`, `cradle-to-grave-capability-gradient.md`)
   — *what we do instead.*
3. → **the law** (`constitution.md` Article I boundaries, Article II's LOVE definition) — *what binds it.*
4. → **the warrant** (`confession.md` grace-precedes-demand, belovedness-unconditional,
   binding-as-covenant; `theology.md` for the same forged under objection) — *why it is true.*

**Design constraint:** the `why:` fields are already long, and stuffing theology into them would make
the guards *nag* rather than teach — failing the stasis test. Disclosure means **linking in a walkable
order, not growing the prompt.** The guard stays short; the path stays open.

**Guard edits (Slice 0, non-code):**

- `sovereignty-ontology-guard@2` and `ownership-ontology-guard@2` each gain a **sibling cross-cite**
  and the four-rung chain. Both are version bumps, not in-place mutations — existing bindings keep
  v1 semantics until they re-declare.
- `ownership-ontology-guard@2` adds the **`INALIENABLE` frame** the red-team found missing. Today the
  guard offers adversary / bounded / external-legibility, and **none fits a person's claim over their
  own body or intimate images** — so an author writing correctly about a survivor's absolute claim is
  prompted to reframe toward custody, and the path of least resistance *weakens the claim.* This is
  the one place the guard currently teaches the wrong lesson, and it teaches it on the cases that
  matter most.
- Fix the **Stance II.1/II.4 miscitation** already propagating in
  `2026-07-15-sense-respond-governance-classifier-design.md:444` — it drifts *against a sealed cite*.

Both are `operator-ratification-pending`: an agent may author an `ask`, but changing what the
protocol teaches about love is not an agent's call.
