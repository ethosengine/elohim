# Stewardship — the custodial account class

**The concern in one line:** a **party** acting on **another party's** account, under a grant that
names its basis, bounds its dimension, expires, and can be contested — with **many such grants
coexisting** over one person, socially composed rather than ranked.

A party is not necessarily a person. The steward may be:

| Steward shape | Example | Natural `authority_basis` |
|---|---|---|
| a person | a legal guardian; a named custodian | `minor_guardianship`, `court_order` |
| a **pair or household** | a husband and wife stewarding their child | `minor_guardianship`, `mutual_consent` |
| an **institution** | a correctional facility; a care home; a school | `organizational_role`, `court_order` |
| a **court** | a court holding authority over a ward | `court_order` |
| a **community body** | a qahal exercising community intervention | `community_consensus` |
| a **care relationship** | family supporting an adult with IDD, or a senior | `medical_necessity` |

Kids. Legal wards. Court-appointed custody. Adults with IDD receiving medically-necessary support.
Seniors whose family holds authority for them. Organizations managing a device they own. These are
not edge cases bolted onto a normal account — they are a **class** of account whose defining
property is that **the acting party and the subject are two different parties**.

This directory is the entrypoint for modeling and validating that class.

---

## Why this directory exists (read this before writing code)

**The substrate already models most of this. Nothing above the DNA does.** That asymmetry is the
whole problem, and it is why the first deliverable here is a scenario rather than an endpoint.

### What exists

The imagodei integrity zome carries `StewardshipGrant` — a far more careful model than "an admin
console over a managed account":

| Field group | What it carries | Why it matters |
|---|---|---|
| `steward_id` + `subject_id` | **Two parties**, explicitly separated | The one thing every layer above forgets |
| `authority_basis` | Closed set: `minor_guardianship`, `court_order`, `medical_necessity`, `community_consensus`, `organizational_role`, `mutual_consent` — with `evidence_hash`, `verified_by` | Authority must *say what it rests on*, and that claim is checkable |
| capability scope | `content_filtering`, `time_limits`, `feature_restrictions`, `activity_monitoring`, `policy_delegation` — separate booleans | Authority over **named surfaces**, never a rank |
| lifecycle | Mandatory `expires_at` **and** `review_at`; `status` incl. `revoked`; bounded `delegation_depth` | A grant that never expires is a transfer of personhood |
| `appeal_id` | Links to `StewardshipAppeal` (`scope`, `excessive`, `invalid_evidence`, `capability_request`) | **The subject has standing to contest** |

Siblings: `StewardshipAppeal`, `DevicePolicy` (composes **one-way** — each layer may only ADD
restrictions, never remove a parent's), `ActivityLog`, `RelationshipRenewal`, `HumanRelationship`.

The collective primitives exist in the **same integrity zome** (qahal substrate), which is why the
generalization below composes rather than needing new entry types:

- `Collective { founder_agent_cid, charter, display_name, anchor_agreement_cid }`
- `Membership { member_cid, member_kind, collective_cid, role, sponsor_cid, withdrawn_at_block_height }`
- `MemberKind::{ Person, Collective, ElohimAgent }` — **collectives can contain collectives**, so a
  court→facility→unit chain is expressible
- `MembershipRole::{ Steward, Contributor, Observer }` — and `sponsor_cid` is documented as *"set when
  role == Steward and pending counter-attestation"*, so steward-of-a-collective **already requires a
  second party to attest**
- `CollabAgreement { participants: Vec<String> /* Collective CIDs */, scope }` — multi-collective
  co-stewardship (a court **and** a family) is expressible

### The inconsistency to resolve (the design question this lane opens)

`authority_basis` is **already collective-shaped**: `court_order`, `organizational_role`, and
`community_consensus` do not describe individuals. A court is not a person. A correctional
institution is not a person.

But **`steward_id` is a single `String`.**

So the existing model names institutional authority in its vocabulary while typing its holder as
one identifier. Resolving that is this lane's first architectural question, and the shape is
already available: let `steward_id` denote **a Human OR a Collective CID**, and when it denotes a
Collective, authority reaches an acting human over a **two-hop path**:

```
StewardshipGrant(steward_id = <Collective CID>, subject_id = <Human>)
    └─ Membership(member_cid = <Human>, collective_cid = <same>, role = Steward, sponsor_cid = <attester>)
         └─ the acting human
```

`delegation_depth` already exists to bound the hops. **Do not invent a parallel mechanism** —
p2p-design-gate this before proposing any new entry type, because the honest answer may well be
*zero new entry types*.

### The questions the generalization opens (name them; don't answer them here)

1. **Who acts for the collective, and is that recorded?** When a facility restricts someone, *which
   human* did it? `ActivityLog` exists; whether it captures the acting member behind a collective
   grant is unverified.
2. **Who hears the appeal when the steward is the institution?** Appealing to your parents and
   appealing to a correctional facility are not the same act. `StewardshipAppeal` has appeal types
   but no forum. A court as steward makes this sharp.
3. **Does a collective steward need more than one attester?** `Membership.sponsor_cid` suggests the
   substrate's instinct is yes. A two-person household stewarding their own child probably should
   not need external counter-attestation; a facility probably should.
4. **Can a collective be the SUBJECT?** (An org account stewarded by a board.) Out of scope for the
   first chapters — but do not write a scenario that makes it *impossible*.

---

## Plural stewardship — slots, dimensions, and social composition

**A person is not stewarded by "their steward." They are stewarded by many parties at once, each
over a different dimension of life, and those relationships compose socially rather than by rank.**

A child: parents day-to-day, a school during school hours, a doctor for health data, possibly a
court if custody is contested. A senior: an adult child for finances, a care home for daily care, a
physician for medical decisions. A person in a correctional institution: the institution for custody,
a legal advocate, family for contact. **None of these is "the" steward, and none should be able to
overwrite another's dimension.**

### What already supports this

More than it first appears — this is a composition problem, not an entity-modeling problem:

- **Plural grants are already structurally possible.** `StewardshipGrant` is per-*relationship*, not
  per-person; there is no "steward" field on `Human` and nothing enforces one grant per `subject_id`.
  N concurrent grants over one subject already type-check.
- **`DevicePolicy.inherits_from: Option<String>` is a general parent pointer.** The doc comment's
  "Organization → Guardian → Elohim → Subject" is an *example ladder*, not the mechanism. The field
  composes arbitrary chains already.
- **`reach_level_max: Option<u8>` is already in the policy model** — the Reach vocabulary is the
  existing language for "how far does this extend." Do **not** invent a parallel scope vocabulary
  for stewardship dimension; compose on Reach and say where it falls short.
- **Monotone composition makes peer stewards order-independent.** Because each layer may only ADD
  restrictions, the composition of a *set* of policies is their **union** — well-defined without any
  precedence order. Two peer stewards (a mother and a father; a family and a school) need no
  "who comes first" answer for restrictions to compose deterministically. **This is the property
  that makes social composition tractable; protect it.**

### The three real gaps

1. **A grant has no dimension of applicability.** `authority_basis` says *why*, the capability
   booleans say *what*, but nothing says *when or over which domain*. A school's grant should not
   reach at-home hours; a physician's should not reach reading history. `DevicePolicy` has
   `time_windows_json` and `reach_level_max`, but those live on the **policy**, not on the **grant** —
   so the scope of *authority* is unbounded even when the scope of a *rule* is bounded.
2. **`inherits_from` is a chain, and peers are not a chain.** Linearizing forces someone to be
   "first," which is a **social claim the substrate should not make on its own**. Peer grants want a
   set/lattice, not a parent pointer. The monotone-union property above means this is mostly a typing
   and traversal question, not a semantics rewrite.
3. **Restrictions only ever accumulate, and no steward can relax another's.** With plural stewards
   the union can over-restrict a person with no party able to release it — the failure mode is
   *quiet*, and the subject feels it as an unexplained shrinking of their own life. The relief valve
   is the **appeal path** (`StewardshipAppeal`) and, plausibly, a `role: ceiling` lens bounding total
   composed restriction. Name it in a scenario before designing it.

### Postures to hold

- **No primary-steward field.** The moment one exists, every other relationship becomes secondary by
  construction, and the social composition collapses back into a rank.
- **A grant that cannot say what it does *not* cover is too broad.** Dimension is a property of the
  grant, not a convention of the UI.
- **The subject is a party to their own composition.** `DevicePolicy.subject_can_view` already exists
  ("Transparency — subject sees logs"). Extend that instinct: a person should be able to see *who
  holds what over them*, in one place. That view is probably this lane's most humane deliverable.

---

## Concern taxonomy

Each `.feature` carries exactly one `@concern:` tag. That tag is the only join between the story,
the CI check, and `genesis/manifests/habits.yaml` — reuse it verbatim, never paraphrase it.

| `@concern:` tag | Chapter | Status |
|---|---|---|
| `custodial-stewardship-01-grant-established` | A grant is created with a named `authority_basis` and verifiable evidence; it expires and has a review date | unwritten |
| `custodial-stewardship-02-steward-acts` | A steward exercises a granted capability on the subject's account; an ungranted capability is refused | unwritten |
| `custodial-stewardship-03-subject-appeals` | The subject files a `StewardshipAppeal`; the grant is narrowed or the appeal is answered | unwritten |
| `custodial-stewardship-04-policy-composes-one-way` | A nested layer adds a restriction; an attempt to *remove* a parent's restriction is refused | unwritten |
| `custodial-stewardship-05-grant-expires-and-revokes` | A grant lapses at `expires_at`; a revoked grant stops conferring authority immediately | unwritten |
| `custodial-stewardship-06-collective-stewards` | The steward is a **Collective** (a household pair; an institution). Authority reaches an acting human via `Membership`, and the acting human is identified | unwritten |
| `custodial-stewardship-07-institutional-accountability` | A collective steward acts; `ActivityLog` records **which member** acted, and the subject can see it | unwritten |
| `custodial-stewardship-08-graduation` | Ward → **self-steward**: the subject takes over their own account | unwritten |
| `custodial-stewardship-09-plural-stewards-compose` | **Two or more concurrent grants** over one subject (parents + school; family + care home). Restrictions compose as a **union**; neither steward can relax the other's; neither is primary | unwritten |
| `custodial-stewardship-10-dimension-bounded` | A grant applies **only within its dimension** — the school's authority does not reach at-home hours; the physician's does not reach reading history | unwritten |
| `custodial-stewardship-11-subject-sees-who-holds-what` | The subject can enumerate **every** party holding authority over them, with each grant's basis, dimension, and expiry | unwritten |

Chapter order is narrative, not execution order. Pick up 01 and 03 together — a grant scenario
without its appeal is the exact drift this directory's `.epr-meta` guards against. Chapters 06–07
are the collective generalization and should be written as a pair; an institution that can act
without naming the human who acted is the failure mode they exist to prevent.

**Prior art, and a distinction worth keeping:** `features/dataplane/resiliency-saga/05-co-steward-agreement.feature`
already models *co-stewardship* — but of **content** (replication custody, via a Mishpat
`replicates-commons` Commitment). Co-stewarding blobs is not co-stewarding a **person**. Reuse its
carrier pattern (Mishpat Commitment → projected row → HTTP probe); do not reuse its semantics.

---

## The three things NOT to do

Recorded as **question 8** ("whose account is this caller acting on — their own, or someone
else's?") in [the doorway auth-posture canon](../../../docs/content/elohim-protocol/architecture/2026-08-25-doorway-auth-posture-declared-stage.md):

1. **Do not add a `PermissionLevel` tier.** It is a total order (`Public < Authenticated < Admin`).
   `Steward = 3` would make a steward *globally* more powerful rather than powerful over **one
   relationship**. Custodial authority is relational; it does not belong on a scalar axis. This gets
   *worse* with collectives — an institution granted a global rank is precisely the shape to avoid.
2. **Do not overload `is_steward` or `custodial`.** Three names in `doorway-service` already mean
   the *opposite* thing — `is_steward`, `Claims.session_id` "(custodial mode)", and
   `src/custodial_keys/` all describe *becoming a self-steward* (the doorway holds **your own** key
   until you graduate). A custodial-session field must be named for the relationship
   (`acting_on_behalf_of` / `subject_id`), and must be able to carry a Collective.
3. **Do not build a doorway stub.** A steward relationship is a witnessed, revocable, DHT-notarized
   fact. A placeholder in the projection layer puts it in the wrong seam and invites (1).

---

## The frame to defend

The zome's own module header states it:

> *"This is NOT external control — it's about identity and self-knowledge… Power scales with
> responsibility, not role assignment."*

The G Suite superadmin analogy is **right about the controls and wrong about the accountability**. A
steward is accountable *to* the subject, not merely *over* them — and that obligation does not
dilute when the steward is an institution. **It intensifies**, because the asymmetry is larger and
the subject is often least able to assert their own standing. Every scenario here should be readable
by the person being stewarded without insult.

---

## Trajectory

- **Ward → self-steward is a graduation event** — structurally the same unbuilt source-chain
  migration that `admin_conductors.rs` already tracks as MongoDB flag-state for hosted humans
  graduating to their own device. One migration, not two.
- The bounded-authority primitive is `Mishpat::Commitment` / delegates-compute. The constraint that
  must give for custodial delegation is **`performer == recipient`** — this is precisely the case
  that relaxes it, in a controlled way, and the collective case is the same relaxation one hop
  further out.
- Social recovery (`RecoveryVote` / `RecoveryHint`, `RECOVERY-PROTOCOL.md`) is the nearest existing
  several-parties-act-for-one machinery. **Relate to it; do not build a parallel one.**

---

## Lane status — what is real and what is not

Honest inventory, so nobody reads this directory as further along than it is:

- **Entry types:** `StewardshipGrant` + siblings live in the imagodei integrity zome; `Collective` /
  `Membership` / `CollabAgreement` live in the same zome. ✅
- **The two-hop collective path:** **not modeled** — `steward_id` is a single `String` while
  `authority_basis` already names institutional bases. This is the lane's first design question.
- **Plural grants:** structurally possible today (nothing enforces one grant per subject), but the
  **dimension of applicability** is unmodeled and peer composition is typed as a chain
  (`inherits_from`) rather than a set. The monotone-union property already holds and should be
  protected. No "who holds what over me" view exists anywhere.
- **Consumers above the DNA:** none. Zero `StewardshipGrant` references in `doorway-service`.
- **Scenarios:** none yet — every row in the taxonomy above is `unwritten`. This README is the
  specification for them, not a record of them.
- **Habit:** none. `genesis/manifests/habits.yaml` is at its **12/12 cap**, so this lane gets a habit
  only when an existing one graduates. Do not add a 13th to make this lane look measured.
- **⚠ Pruning risk (time-sensitive):** `imagodei_integrity/src/lib.rs` is being pruned on a
  *"zero callers → removed C.2"* heuristic. `StewardshipGrant`/`StewardshipAppeal` are marked
  *deferred to Stage G* with 3 and 1 live `create_entry` callers. Measured from above — no consumer,
  no scenario — an honest Stage G pass would prune the most carefully-designed part of this class.
  **Before pruning, ask: does anything else encode legal-guardianship authority basis with appeal
  rights?** Writing chapter 01 is the cheapest way to make this model visibly load-bearing.

Cluster row: `commons-holonic-stewardship-backlog` row 26 (this is row 2a's `subjectStanding`
question at its sharpest — the subject may be the party least able to assert their own standing).
Valueflow lane: recipe `custodial-stewardship` in `.claude/epr-meta/recipes.yaml`.
