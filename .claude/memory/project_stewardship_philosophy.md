---
name: Elohim stewardship philosophy — graduated capability, accountable authority, visible shape
description: Six principles the protocol commits to across the cradle-to-grave stewardship spectrum (ward-child, legal custody, criminal-justice, intellectual disability, elder care, full autonomy)
type: project
originSessionId: a501f46a-fd6b-4b45-b9f0-a1ad3770d20d
---
Elohim treats stewardship as a single cradle-to-grave lifecycle, not a collection of special-case features. Every human is born a ward, progressively acquires self-sovereignty, may acquire wards of their own (children, elders), and may re-enter stewardship themselves (infirmity, dementia, incapacity). The same protocol primitives — capability grants, attested authorization, graduated scope — flow through all of it.

**The cleave the protocol takes:** self-sovereign-identity ideology says capability should never be mediated; traditional systems (Apple Family Link, custodial crypto wallets, enterprise IAM) say authority should be hierarchical and opaque. Elohim's answer is that **capability is relational and bounded, authority is graduated and accountable, and the shape of both is visible to everyone who has standing in it.**

**The six principles:**

1. **Stewardship is scoped capability, not substitution of identity.** The ward always has their own source chain. Stewards hold capability grants *over* that chain, bounded by policy. Opposite of custodial-wallet pattern where the guardian literally holds the keys — in elohim, the ward is always the named author; the steward is the named authorizer.

2. **Capability is graduated and relational, not age-keyed.** A mature 15-year-old can hold more scope than a fragile 85-year-old. Age defaults exist but are rebuttable. Competence-based transitions (financial literacy, completed therapy, court-restored capacity) move the envelope, not just birthdays.

3. **Stewardship is visible to the ward.** No invisible paternalism. At an age/comprehension-appropriate level, every ward can see the shape of their own envelope, who holds what grant, and why. Distinguishes elohim from every parental-controls pattern where the ward is a managed object rather than a witnessed subject.

4. **Stewards are accountable, not omnipotent.** Actions taken under grant are attested — the steward's source chain signs the authorization. Reframes "parental controls" (and custody, probation, POA) as *accountable trust*: the steward bears liability, the ward gets an audit trail, courts and mediators can reconstruct what happened.

5. **Plural stewardship is normal.** Divorced parents, co-guardians, probation-officer + family, medical POA + adult child. Multiple concurrent stewards with overlapping or orthogonal scopes, quorum requirements for high-stakes actions. Hierarchical custody is just the n=1 special case of the plural pattern.

6. **Transitions are first-class events.** Aging into majority, graduating out of probation, entering dementia care — these are ceremonies the protocol recognizes, not silent flag flips. The ward (where possible) and affected stewards participate in them; the transition is itself recorded on-chain.

**Why:** A protocol that aspires to cradle-to-grave use must have a coherent position on the humans who don't fit the "autonomous adult equal" default — because that's a majority of humans across a lifetime, not an edge case. Without a declared philosophy, imagodei will grow ad-hoc special cases for children, for elder users, for custody situations, for disability support — and they'll drift apart, leak assumptions, and eventually break. The six principles give downstream designers a shared rubric: when you're building a flow, you can check it against them.

**How to apply:** When touching imagodei identity primitives, capability-grant issuance, authentication flows, parental/guardian controls, custodial enrollment, consent flows, or any scenario where one human acts on behalf of another — check the design against the six principles. Red flags: flows where the ward can't see their own envelope (violates #3), grants with no expiry or no attestation trail (violates #4), single-steward assumptions that won't generalize to co-parents (violates #5), transitions that are silent flag flips (violates #6), substitution-of-identity patterns (violates #1), age-keyed capability gates without rebuttable override (violates #2). When designing new scenarios in `genesis/a2o/features/auth/`, span the spectrum: child, legal-custody, probation, intellectual-disability-partial-capacity, elder-memory-decline, full-POA. Cross-references: `project_stewarded_child_identity.md` (Timothy, the canonical child case), `project_multi_device_humans.md` (one human, multiple devices/authority levels).

**Status:** philosophy declared in this memory as of 2026-04-17 conversation thread. A durable doc in `genesis/docs/content/elohim-protocol/` is a reasonable next artifact if/when the principles face their first real design pressure — don't preemptively create it, but propose it when imagodei custodial work begins in earnest.
