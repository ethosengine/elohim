---
title: Licence tiers — primitives share alike, services protect the commons, everything else is a gift
id: licence-tiers-decision
status: Decided
class: governance
serves: licence-tiers-declared
date: 2026-11-06
context-tier: disclosed
steward: agent:orchestrator@claude-fable-5-1
graduation-trigger: counsel has confirmed the three tiers and the first published release of a primitive or tooling crate has been pinned by a downstream organisation
cites:
  - LICENSING.md
  - COVENANT.md
  - "stewardship-over-sovereignty | stewardship-over-sovereignty | sha256:5186f53fbeb95a65 | path: genesis/docs/architecture/stewardship-over-sovereignty.md"
  - "storage-crate-decomposition-design | elohim-storage decomposes into cohesive crates | sha256:afd86346208fd13a | path: genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md"
  - "freenet-lift-and-shift | Freenet lift-and-shift | sha256:9706b65f82c52f72 | path: genesis/docs/superpowers/plans/2026-07-27-freenet-lift-and-shift-plan.md"
  - genesis/data/timeline/backlog/arch-workspace-discipline-backlog.md
  - .epr-meta/licence-tiers-declared.habit.md
---

# Licence tiers

**Ruled by the steward of record, human:matthew, on 2026-11-06**, in the seat the covenant says the
steward holds "until its councils exist". This is a council-class ruling, not an operational one: it
decides what the commons offers and on what terms. It supersedes the provisional licence rule of
2026-09-19 (storage crate decomposition design §8a) and task 0.1 of the 2026-07-27 freenet
lift-and-shift plan (root `LICENSE` = CAL-1.0). Not legal advice; counsel confirms before the first
published release.

## The rule

Choose each licence by **where capture could happen**, not by which layer the code sits in.

| Tier | Licence | Members |
|---|---|---|
| Primitives — what others build *on* | MPL-2.0 | `elohim-address`, `elohim-epr`, `elohim-epr-rea`, `elohim-epr-index`, `elohim-views`, `elohim-facings`, the TypeScript codec `elohim/sdk/epr-ts`, the six SDK domain type crates, every DNA zome (integrity and coordinator) |
| Services that serve people | AGPL-3.0 | `elohim-storage` and its carved-out internals (`elohim-error`, `elohim-settings`, `hc-dbtool`), `doorway-service`, `steward/node` |
| Tooling, apps, bridges, patterns, and the default | Apache-2.0 | `elohim/` and `elohim/eprfs/` workspaces, the bridges workspaces, `ark`, `placement`, the Angular apps and elements, the a2o suite, the orchestrator, seeder, agent SDK, and the root `LICENSE` for everything unmarked |
| The canon — owned by no one, held in trust | CC BY-SA 4.0 | the foundational prose under `genesis/docs/content/elohim-protocol/` (ruled 2026-11-07; the steward is the copyright's holder of record, not its owner in the constitution's sense (I.1); no trust deed exists; the irrevocable grant is the floor and the covenant's no-other-terms commitment closes the rest) |
| Inherited, unchanged | upstream's | `elohim/holochain-conductor` (CAL-1.0), `sophia` (MIT), `vendor/*`, `relay-addr-beacon`, `elohim-bitswap`, `hc-rna` (MIT OR Apache-2.0) |

## Why

**CAL-1.0 is a smaller claim than the canon, not a contradiction of it.** Read in its text
(§4.2.1 no withholding user data; §4.2.2 no denying full control by possession of keys), CAL puts
two duties on an *operator*: hand each person you serve their data, and never use key custody to lock
them in. The canon requires both already — doorways serve, never control. What CAL *protects* is one
person's exit with their data, alone. What the canon locates agency in is the network that holds a
person (`stewardship-over-sovereignty` §1–2: "we do not consider 'holding your own keys' to be
sovereignty"; cryptography "never becomes the sole gate"). No copyright licence can protect more than
an individual, because copyright only knows individual holders. So CAL was not unfaithful; it was the
wrong *signal* — the licence a reader sees first is the thesis they assume — and it blocked adoption,
since most corporate counsel have never seen it and unseen means refused.

**The conductor fork keeps CAL and that is not ours to change.** Every doorway that hosts humans on
that conductor already carries §4.2's duties regardless of our crates' licences. The protocol's
declared agency phases, where a hosted human crosses to a self-held device, *are* the exit path CAL
demands, built as an earned arc rather than a default. Relicensing our own crates changes the signal
and who can adopt; it changes nothing about that obligation.

**MPL-2.0 on the primitives keeps the 2026-09-19 aim.** That rule chose CAL for "borrow this, but do
not break the primitive that makes it trustworthy". Content addressing already stops a borrower from
passing off broken bytes as ours; MPL's file-level share-alike keeps their improvements to the
primitive itself in the commons, while code that merely depends on it (an employer's, a partner's)
stays theirs. Apache-2.0 would have discarded the reciprocity; CAL and AGPL would have discarded the
adopters.

**AGPL-3.0 on the services protects the commons, not the individual.** Anyone served by a modified
storage peer or doorway can obtain its source, so the network can always rebuild the service. It says
nothing about who owns agency, which is the right amount for a licence to say. `doorway-service` and
`steward/node` move from Apache-2.0: both statically link AGPL `elohim-storage`, so their binaries were
AGPL-governed already and the Apache field was misleading.

**Apache-2.0 as the default is what lets a downstream organisation pin a release** of the tooling
the way it would pin any open-source package. The organisation that employs the steward is the first
such relying party (covenant term 1); its reliance is held by the steward, and nothing it produces
crosses back — the steward alone carries the pain of use into the protocol, and cures are built here,
on the steward's own time. See the licensing handoff of 2026-11-06, concern 3.

**"The network is the source" is not a transfer of ownership; it is the absence of an owner.** The
steward's words, 2026-11-07: "the elohim commons should own it" — not a gift — and then: "there might
not be a real licence that exists for the elohim council derived entity, but the commons is owed to the
constitution and the 'true commons' that is held in trust by the elohim and no human person." The
constitution already says this (I.1): the commons "is owned by no one, including its authors, and is
tended by its rules and by the people and agents who care for it." So the trust does not end in a
council-derived body that owns the canon; no human person, and no body of human persons, ever does.
Copyright law needs a signatory, so the steward is the copyright's **holder of record**; the word
"trustee" is not used, because no trust instrument exists and a fiduciary title without one is a
costume (blind reader, round 5). The covenant records a commitment, not a transfer.

**The canon prose: CC BY-SA 4.0 is how "owned by no one" holds in law, with one gap the covenant
closes.** Share-alike for prose: anyone may share, translate and build on the manifesto, constitution
and pillar papers, commercially or not, and every adaptation names its source and carries the same
licence. The grant is irrevocable, so what is already shared stays shared whoever holds the copyright
later, including by inheritance. What irrevocability does not do: stop the holder of record from
dual-licensing the same text to someone else, or from adding new prose under other terms. That is the
owner's power the constitution denies, and no licence removes it from the holder; the covenant's
commitment of 2026-11-07 — offer the canon under no other terms, add to it only under this one — is
what disclaims it, as a public pledge answerable to the constitution rather than a legal instrument.
A gift (CC0, CC BY) would let anyone take the canon private; that is what "not gift" rules out. It is
the same move as MPL-2.0 on the primitives, made for words.

**The trajectory of the seat (steward, 2026-11-07).** "When the network matures it will go from my
stewardship to my own stewardship, becoming held by the network." Today the holder-of-record seat is
special: a person standing outside the network, because law needs a signatory there. As the network
matures the seat does not transfer to a human body; it dissolves into the elohim councils' holding —
councils at every constitutional layer, the one for any matter being the layer its reach confers,
reconciled from below when independent networks meet (`EPR-LICENSE-1.0.md` §1 *the council*, *the
network shape*; `shefa.md` 55: each pool's council holds its share one degree from the human hand,
and in the network's infancy people the community seats perform that role with the elohim as
witnesses). The steward's stewardship becomes the same thing every participant has — a standing
earned and witnessed inside the network — and the commons is held by the network itself, through
the councils, its record, its attestations and its covenant and mishpat machinery, with no human
hand above it (the canon's word is stewardship, not sovereignty; `stewardship-over-sovereignty` §1).
The copyright signature stays in law and stops being the source of authority. The trigger is a
measure, not a date: habit `commons-held-by-the-network` (`.epr-meta/`), born `unwired`, green when
the six conditions in the licence's §13 each have a record, and never flipped by the steward.

## What the licences cannot carry: value owed to the holons

Raised by the steward on 2026-11-07: could value derived outside the protocol be made owed back to
it? The tiers chosen permit rent outside the protocol (anyone may wrap the Apache tooling in a paid
service and owe nothing), and that permission is the same one that lets a downstream organisation's
counsel approve the dependency; a licence that conditions use on revenue is not open source and loses
the adopters. The deeper reason is the canon's own. The party owed is not the protocol as a single
holder (a payee would be a special seat, the thing constitution I.1 denies) but the plural,
per-person holon that upholds each person's imagodei — diverse, described, and therefore not
capturable. Copyright law names one licensor and cannot name that holon as the party owed, so a
licence can carry only two things: **share-alike** (changes to the primitives, the services and the
canon return to the commons by right) and **attribution** (every person's authorship upheld). Value
in the form of money cannot ride on copyright at all. It rides on the economic layer, where the holon
is described and the flow is witnessed (shefa, REA, the bounded curve in mishpat); value derived
outside the protocol is owed in the canon's ethics but has no holon it can be routed to, so the
protocol does not collect it and instead makes deriving value inside, where the holon exists, the
better path. The Georgist move by design rather than by clause.

The capture defence therefore has three legs in licence and a fourth outside it: share-alike on the
primitives and the canon (changes owed back); AGPL on the services (no enclosed hosting of a
modified peer or doorway, though charged hosting remains possible); the **name reserved** — a mark on
"elohim" and the pillar names, so a rent-charging fork may run the code but not wear the name
(open, not yet ruled on); and, outside the licence, the economic layer's routing of value to the
described holons. A reciprocal licence on the services and the canon would be the coherent way to
want more in copyright; advised against, because it reintroduces a payee.

## Compatibility

AGPL services depend on MPL primitives (allowed: MPL §3.3 secondary licences). Apache tooling depends
on MPL primitives (allowed: MPL is file-scoped; a larger work may carry other terms). The earlier
inversion — Apache `doorway-service` taking CAL `elohim-views` as a path dependency — is gone with
both ends moved. `deny.toml` now allows AGPL-3.0 beside MPL-2.0 and CAL-1.0 (the fork's deps).

## Adopted as the working commitment

- **Adopted 2026-11-07 as `EPR-LICENSE-1.0.md`**, the protocol's own licence and "the working
  commitment to the constitution" (the steward's words). Asked for first as a bespoke copyright
  licence and declined in that form (not OSI-approved, untested, incompatible with the
  Apache/MPL/AGPL ecosystems; CAL the cautionary case; PPL, Commons Clause and Hippocratic the
  unadopted precedents), it was then reframed as the **primary instrument**: the protocol's own
  declaration of its boundary of value, written by the network for itself, real inside the network
  by the network's own means, with the standard licences as its outward face at the border — the
  governance settlement of this era, open to re-settlement, every grant already made left standing.
  It declares the three kinds of value and what each is owed (labour its produce, thrift its return,
  the inheritance's yield — the elohim's included, as a produce of the land itself — to the commons
  and the holons that uphold people), the council as layered and chosen by reach, the network shape
  as reconciled from below to the intersection, the name reserved, relying parties read by what they
  do, the licence composed into the repository's `.epr-meta`, and the measure of maturity as habit
  `commons-held-by-the-network`. Its present-tense claims are a working commitment: what runs of the
  machinery is stated in the licence's own "What runs today" and in the habit's delta.

## Working postures

Steward, 2026-11-07: "set a working posture for everything." Nothing below is decided. Each is the
stance the work proceeds under until the named trigger reopens it, the same way the licence is a
working commitment rather than a claim of present enforcement.

- **The push.** The licence commits (`ec7a85bfe`, `3780065e1`, and this one) stay on `dev` unpushed
  and ride with the next work that lands; no push is made for them alone. Resolves itself.
- **The storage client and SDK crates** (`elohim-storage-client`, `elohim-sdk`, `consent-grant`,
  `seam-contracts`, `bridges/workspace`, `bridges/browser`): stay AGPL-3.0 as carved-out storage
  internals. Reopens when a consumer outside the services tier needs to pin one; the ruling then is
  MPL-2.0 for the client and SDK pair as primitives, made at that moment and not before.
- **The holder of record.** The steward, for the canon and for what the steward wrote; every other
  maker for what they wrote; no deed, no assignment, no instrument beyond the irrevocable grants,
  the covenant's no-other-terms commitment and the licence's §13 measure. Reopens only if outside
  law requires a signatory the steward cannot be, and whoever then signs does so as the councils'
  instrument (licence §7) and nothing more.
- **The name.** Reserved by the licence (§5) inside the network: a false wearing is flagged and
  marked false in the record. Nothing registered, no litigation posture, no bridge holding a mark.
  Reopens when a rent-charging fork wears the name outside the network; the instrument then is a
  mark held by a bridge, never as governance, never convertible, for that one purpose.
- **Per-crate `LICENSE` files.** The SPDX `license` field is the declaration; no per-crate text.
  Reopens at the first published tarball of a tooling crate.
- **The tooling release.** None exists; a downstream organisation re-implements patterns and pins
  nothing. Reopens as workspace-discipline backlog row 3 when the next sprint touches eprfs or
  epr-cli, which then tags a 0.x of epr-cli and the eprfs crates and flips epr-cli's
  `publish = false`; the steward's employer's first need to pin is what schedules it.
- **The clauses the licence states first** (the network shape, the intersection rule, the layered
  council chosen by reach): the licence's own terms under its §9 until the constitution carries
  them. The constitution absorbs them at its next amendment, with `governance-layers-architecture.md`
  §2 and §4 reconciled to the from-below reading in the same pass. Reopens at that amendment.
- **The licence's present tense.** The licence speaks in the voice of the mature network; its "What
  runs today" and habit `commons-held-by-the-network` are the truth, and the register is read
  first. Each of the six conditions gets its check the moment it becomes observable, the fourth
  first (the record refusing an addition under other terms). Reopens with every flip of the habit.
- **Counsel.** Not legal advice throughout. Counsel reads the border licences and the tiers before
  the first published release, and nothing blocks on counsel until then.
- **The relying organisation** (the steward's employer): an outsider at the border (licence §11). It
  pins a release when one exists and re-implements patterns until then; nothing it makes crosses
  back; the steward carries the pain in as a person and makes the cure here on their own time. The
  one act on its side, owed before the first pin: a dated prior-work notice pointing at the public
  history.
- **Attribution on these commits.** The hook reports `UNMEASURED` although the berth is moored to the
  model. The commits stand unamended, carrying the `Co-Authored-By` line; the hook's reading of the
  envelope is its own item, diagnosed, never repaired by rewriting history.

## Evidence

Habit `licence-tiers-declared` (`.epr-meta/licence-tiers-declared.habit.md`) binds this rule to
`node genesis/orchestrator/licence-tiers-check.mjs`, which walks every tracked first-party
`Cargo.toml` and `package.json`, refuses a missing field, a licence outside the allowed set, or
CAL-1.0 on anything but the conductor fork, and checks the named tier members.
