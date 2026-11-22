---
title: The standing read for counting — which principal a signer counts as
id: standing-read-for-counting-principals-design
status: Draft
class: process-meta
process_subdomain: agents
serves: [build-earns-reach-by-attestation]
date: 2026-11-22
context-tier: disclosed
steward: human:matthew
author: "Claude (Fable 5.2), at the steward's direction; integrity-class, ruled by the session model"
refines: native-dev-plane-one-derivation-earned-reach-design
tags: [standing, reach, graduation, principals, roster, collective, mishpat, qahal, brit, integrity]
graduation-trigger: >-
  A storage test in which three keys bound to one human yield one qualifying voice passes through
  the native `resolve` and `count` of §3, and a record whose brit signer has no binding row is
  reported `unbound-signer` and counted zero by `epr peer verdicts`. Receipts are attached to
  build-earns-reach-by-attestation, which then names its first check.
cites:
  - "native-dev-plane-one-derivation-earned-reach-design | One derivation, earned reach | sha256:024a89c3dba2699f | path: genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md"
  - "native-dev-tooling-over-peers-plan | Native developer CI/CD over peers | sha256:69d20cd723005913 | path: genesis/docs/superpowers/plans/2026-10-29-native-dev-tooling-over-peers-plan.md"
  - "claim-backed-credentials-and-contextual-authority-design | Claim-backed credentials and contextual authority | sha256:c16fffbadbe818e9 | path: genesis/docs/superpowers/specs/2026-09-30-claim-backed-credentials-and-contextual-authority-design.md"
  - "commons-pool-as-collective-party-design | A commons pool is a collective that is party to its members' commitments | sha256:803fa69bf3735eb9 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
---

# The standing read for counting — which principal a signer counts as

This is station F6-standing of the native dev tooling plan. Rulings D4 and D5 of the native dev
plane design say that reach is earned by principals with standing and that graduation carries one
standing proof per counted principal. Neither can be built until something answers one question:
**given a signature on a record, which principal does it count as, and is that principal distinct
from the others already counted?**

## 1. Why this is a narrower design than the one that failed

The claim-backed credentials design (§12, 2026-10-04) failed two reviews. It tried to settle
standing in general: how standing is first made, changed, recovered and made current, for every
use. Its open findings are about change. Old claims replay ("authentic is not current"), a lower
address can be ground to win a fork, and the founding authority's recovery power is named and not
limited. The operator's ruling after the second review was "final by witness": a change of
standing is provisional until a named body witnesses it.

Counting needs none of that settled. It needs a read, taken at one moment, by one act, inside one
space, that the space's own members can check. This design supplies only that read. It does not
say how standing is recovered, who the founding authority is, or what a person alone may do.
Those stay with the general redraft, and §7 lists what this design leaves to it.

Terms used below: a **signer** is a key that signed a record. A **principal** is a person or a
collective. A **space** is the collective whose policy governs a rung of reach. **k** is the
number of distinct principals counted toward that rung.

## 2. Rulings

**S1 — Standing for counting is membership in the space that owns the rung.** A principal counts
toward a rung only as a member of the collective whose policy governs that rung, admitted by that
collective's own roster. No key, device, credential or attestation held by the principal confers
it. This is the existing rule that a holon's authority comes only from its own roster
(`arch-authority-in-integrity` backlog row 19), applied to counting. It keeps the authority with
the network: nobody declares themselves a second principal.

**S2 — The default is "the same", and unknown counts zero.** Two signers are the same principal
unless the read positively shows two distinct members. A signer that does not resolve counts as
nothing and is reported as unresolved. The aim is that every failure of this read lowers k or
leaves it unchanged, so an error can delay a graduation and cannot produce one. A validator cannot
see every failure, so S6, S7 and S9 say which failures it catches, which the witness catches, and
where an unresolved signer refuses the act outright because counting it as nothing would raise k.

**S3 — A signer resolves through a chain of cross-signed bindings, and every link is checked.**

| Link | From → to | Record | State on dev 6934782dc |
|---|---|---|---|
| a | record signer → device `did:key` | a signed roster row beside `PeerEndpoint`, which already binds an iroh NodeId to a device this way | Missing for brit: its signer is a separate key per clone (`brit-build-ref/src/git_dir.rs:35`) and no row binds it. Present for the iroh NodeId |
| b | device → `human:<handle>` on one chain root | `ParticipantRow::{Genesis, Binding}`; read by `standing_human_pinned` (`elohim/epr-rea/src/participants.rs:1125`) | Built. Proves which handle a device speaks for. Proves nothing about distinctness |
| c | handle and chain root → Holochain agent | mishpat's `binds-identity` commitment (`device_enrollment.rs:84`) and imagodei's `AgentKeyToHuman` link | Built in the coordinator. Not authenticated in an integrity zome |
| d | agent → member of the space | qahal `Membership{member_cid, member_kind, collective_cid, sponsor_cid, withdrawn_at}` (`qahal.rs:37`) | Built. No reader uses it for counting |

Link a is the first refusal station F6-probe found. Of the two ways to close it, this design
rules the binding row and not a change of signing key: brit keeps one key per clone, and a clone's
key counts only while a row signed by both that key and the device key names it. A clone with no
row signs records that are valid and count zero. A signer named by more than one unrevoked binding
row resolves `contested` and counts zero. Records are deduplicated before principals are: one
record counts once, whoever's proofs point at it.

Links a and b are roster rows that travel by git, so no validator can fetch them by address. The
act carries them inline, and a row left out (a `Contest`, a revocation) is invisible to the
validator. S7 gives that check to the witness.

**S4 — Only `Person` and `Collective` members are counted.** A `Membership` whose `member_kind`
is `ElohimAgent` never counts, and neither does any agent, device or runner acting under a grant.
Their records resolve through the grant to the principal who issued it and count as that
principal. The grant is a fifth cited link: authored by the issuer, scoped to the kind of work the
record is, and current under S7. A record whose grant is absent, out of scope or not authored by
the claimed issuer resolves `grant-invalid`. A CI runner is a sponsored service agent with no
standing of its own; its record counts as its sponsor, once.

A signer resolves to exactly one member of the space. A `Collective` member counts only through an
act of its own steward under that collective's roster, never through any one of its members'
devices. A `Person` counted directly excludes every collective they belong to from the same
count, and a collective counted excludes its members.

**S5 — Distinctness is the space's admission and nothing else.** Two members are distinct because
the space admitted two members, each by a sponsor who is already a member and who authored or
countersigned the admission. Today `Membership` validation is pure data, requires a sponsor only
for the `Steward` role and accepts the literal `founder` (`imagodei_integrity/src/qahal.rs:91`), so
a membership naming a real member as sponsor can be authored by anyone; §5 item 3 is what makes
this ruling true, and until it lands a membership resolves `unauthenticated-membership`. The read does not
judge whether two members are two humans. That judgment is the space's, made at admission and
open to challenge there. A space that wants more (a humanness attestation, a witness in person)
states it in its own policy, and the graduation act checks the policy it cites.

**S6 — A standing proof is a set of records a validator re-reads for authenticity, never an
assertion.** For each counted principal the graduation act carries: the record's signer; links a
and b inline; links c and d, any grant, and the `Membership` by address; and the `Collective` entry
of the space, by address. An integrity validator re-derives the principal from them. It refuses
the act when any link is not authentic, when a `Membership.collective_cid` is not the space the
cited policy names, when two proofs resolve to one member, or when the builder's own signer does
not resolve (S9).

The validator checks authenticity and nothing about currency. A withdrawal is an update on the
`Membership` create (`qahal_coordinator.rs:716`), and an integrity zome has `must_get_*` only, so
the create it fetches always reads as never withdrawn. The same holds for a superseded device
binding and for a roster `Contest` the steward did not include. The acting steward chooses which
records to cite, so the validator alone would let the steward decide who counts.

**S7 — Currency is the witness's alone, and a provisional act confers no reach.** The space's
named witnessing body checks what the validator cannot: that each cited membership, binding and
grant is current when it witnesses, that no contest or revocation was left out, and that the
cited records are the whole of what the space holds for those signers. Until it has witnessed,
the act is provisional, readers ignore it, and the result's reach is unchanged. This is the
operator's ruling of 2026-10-04, "final by witness", applied to one act. A member who withdraws later
does not undo a graduation. A member shown to have been wrongly counted is a ground for the
space to narrow the result's reach, which D5 already allows in that direction only.

**S8 — One steward alone graduates nothing, and that is a correct answer.** With one roster and
one human, every signer resolves to one principal and k = 1. Native reuse between that steward's
devices is unaffected: it is labelled `self` or `sibling` and never needed a count.

**S9 — The builder's principal must resolve, or the act is refused.** The builder is excluded from
the count of their own work. Excluding "nothing" because the builder's signer did not resolve
would let the builder's second clone or device count as an independent voice, which is a failure
that raises k. So an unresolved builder refuses the graduation and is never treated as zero.

## 3. The read, as a contract

One pure function, with two implementors as the plan's §9 requires:

`resolve(signer, space, roster_head) → Counted(member) | Uncounted(reason)`

`Uncounted` reasons are closed: `unbound-signer`, `device-not-in-roster`, `contested`,
`no-agent-binding`, `not-a-member`, `unauthenticated-membership`, `space-mismatch`, `withdrawn`,
`agent-kind`, `grant-invalid`, `builder`. `count(records, space, head)`
returns the set of distinct members and the list of uncounted records with their reasons. A
caller receives both and may not drop the second.

- **Native implementor** (epr-rea, beside `standing_human_pinned`): links a and b only. It
  answers "which handle", and with one roster it answers k = 1. It is what station F6-k1's tests
  run against first.
- **DNA implementor**: links c and d, in the integrity validator of the graduation act. Nothing
  joins link b to link c today: the native side names `human:<handle>` and the DNA side an agent
  hash (§5 item 4). Native k = 1 therefore says nothing about the DNA side until that join exists.
- **Fake** for tests: a table.

`tally` in `elohim-storage/src/services/release_attestation.rs` counts distinct issuer strings
today. Under this design it counts `resolve` results, and `for_release`'s exclusion of the
builder becomes exclusion of the builder's principal, refused under S9 when it does not resolve.

## 4. P2P design gate

- **Class.** The standing proof is no entity. It is a set of addresses inside the graduation
  act's payload, which D5 already rules as one notarized attestation (A, on the DNA). The signer
  binding of link a is a private roster row (B), like `PeerEndpoint`, and travels by git.
- **Existing entry types.** `Membership`, `Collective`, the `binds-identity` commitment and the
  `AgentKeyToHuman` link exist. This design adds no entry type. It adds validation, which is an
  integrity change and moves the DNA hash, so it rides the batched DNA move D5 names.
- **Head-plane cost.** Zero new heads. One roster row per clone per device natively: tens a year.
- **Identity.** A member is its `member_cid`. A signer binding is content-addressed over (device,
  signer key, both signatures).
- **Coordinator and signal.** None new. The graduation verb is D5's.

## 5. What must be true in code before the first graduation

1. A roster row binds a brit clone's signer to a device, and `epr peer verdicts` and the gate's
   sibling reuse refuse a record whose signer has none. (Native; no DNA move.)
2. `resolve` and `count` exist natively with the closed reason list, and `tally` counts through
   them. Test: three keys of one human yield one voice. (Station F6-k1; no DNA move.)
3. Links c and d are authenticated in an integrity zome. A `Membership` of any role is valid only
   when it is authored or countersigned by its sponsor's agent and cites the sponsor's own
   `Membership` by address; `founder` is accepted only from the author of the `Collective` entry.
   An agent binding is valid only when the human's chain root signed it. (Batched DNA move.)
5. A witnessing body is named for a space, and readers of reach ignore an unwitnessed act (S7).
4. `AgentRef` is documented as a Holochain agent hash (`epr-rea/src/model.rs:22`) and
   `standing_human` returns `human:<handle>` in it. One of the two is corrected before a
   validator compares them.

## 6. What this design does not prevent

- **One human admitted twice.** A space whose members sponsor a second membership for a person
  already in it has k inflated by one. S5 places that control at admission, where people who know
  each other decide. The attestation floors that could help are accept-all today
  (`attestation_validator.rs:65-80`). This is a Sybil-class residual and is named, not closed.
- **A captured sponsor chain.** Every member traces to the collective's founder. A founder who
  admits their own second identity first has seeded the roster. A space's charter is where a
  founding quorum is required; this read cannot see it.
- **A stolen clone key before its binding is revoked.** Its records count as the device's
  principal until the row is contested. It cannot raise k, by S2.

## 7. Left to the general standing redraft

How standing is first made, recovered and changed; who the witnessing body of a space is and how
it is named; the case of one person alone; the council roster
(`2026-09-03-holochain-evolution-epic-design.md:208`), which has no implementation. S7 depends on
the witnessing body being named for a space before that space graduates anything, and until then
no space can.

## 8. Adversarial review, 2026-11-22

One review by a fresh reader (Claude, red-team brief: raise k above the number of real people, or
get an act accepted that should not be). Every finding was accepted on the evidence and cured
above by the session model.

| Finding | What it showed | Cure |
|---|---|---|
| B1 | A withdrawn member still counts: withdrawal is an update a validator cannot see, and the steward picks the records. This repeated the earlier design's "authentic is not current" | S6 limited to authenticity; S7 gives currency to the witness, and a provisional act confers no reach |
| B2 | A membership can be forged by naming a real sponsor; non-steward roles need no sponsor | S5 states today's gap; §5 item 3 requires the sponsor's authorship |
| B3 | An unresolved builder excludes nothing, so a failure raises k | S9 |
| M1 | One person counted directly and again through a collective | S4, last paragraph |
| M2 | A grant was asserted, not cited | S4: the grant is a fifth link; `grant-invalid` |
| M3 | One clone key bound under two devices counts one machine's work twice | S3: such a signer is `contested`; records deduplicate first |
| M4 | Nothing tied a cited membership to the policy's space | S6: `space-mismatch`, the `Collective` fetched by address |
| minor | Reason list incomplete; native and DNA implementors do not join; git rows cannot be fetched by address | §3 and S3 |

The review found that S2 held on the day only because no witnessing body is named and so nothing
can graduate. That is recorded here as the design's condition of safety: it is sound to build
§5 items 1 and 2, and unsound to graduate anything before items 3 and 5.

Still open after this review, and for the next one: whether a witnessing body can itself read
currency completely; how a collective's steward act is told apart from that steward acting for
themselves; and one review is not the two rounds the earlier design received.
