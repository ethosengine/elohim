---
title: "THE TIME-AXIS RECURSION — The Substrate That Outlives Its Authors"
date: 2026-06-14
status: PROPOSAL FOR OPERATOR BLESSING — working draft, NOT cite-sealed, NOT a decision, NOT code
recursion_axis: TIME (decentralization across generations; the cohort turns over before the work is done)
weaves:
  - ESCALATED-ARCHITECTURE-2026-06-14.md (the horizontal synthesis: one Commitment / six faces / coverage invariant / one Governor / two quilts)
  - living_memory/epic.md
  - architecture/2026-05-10-memory-lifecycle-design.md
  - architecture/2026-05-24-records-lifecycle-design.md
  - global-orchestra.md §VIII (consilience is generational)
north_star: >
  "This is generational. Repair does not happen in a sprint. The substrate must not depend on any
  single cohort of stewards — the cohort will turn over before the work is done. EPR identity must
  survive software versions; stories must remain pointable as participants age and die; wisdom from
  one generation must be transmissible to the next. Decentralization in its deepest sense is across
  TIME, not just geography: the protocol must outlive its authors. ... only that the work it makes
  possible is rightful work, and that the substrate will still be holding the biography when the
  next generation arrives to take it up." — global-orchestra.md:279
---

# THE TIME-AXIS RECURSION

> The horizontal synthesis (ESCALATED-ARCHITECTURE) found one Commitment with six faces under a
> `∪ = full` coverage invariant, governed once, felt by a grandmother *this afternoon*. This pass
> asks the question that afternoon cannot answer on its own: **what holds when the grandmother is
> gone, the software is three rewrites later, and the steward who made the promise has handed it to
> a stranger who will themselves hand it on?** The thesis of this pass: **the coverage invariant is
> not only spatial (who holds which keyspace / bytes / head *now*) — it is temporal (who holds the
> biography *across the handoff*).** A custody commitment that has no successor when its steward dies
> is a coverage gap on the time axis exactly as a dropped shard is a coverage gap on the byte axis.
> The Governor that refuses-and-elevates a spatial gap must refuse-and-elevate a *temporal* gap —
> and the lineage chain already in the substrate (`supersedes`/`superseded_by` on the EPR envelope,
> the `pubkey_timeline` validity-window chain, the `lineage-archive://…/<generation>` submerge
> destination) is the spine it refuses against. **Memory's lifecycle is the time-axis of the
> recursion.** Promote/compact/merge/close-interval/submerge/memorialize/forget are not janitorial
> verbs — they are *how the comet stays walkable backward across generations*, and the steward-cohort
> handoff is the moment the coverage invariant is tested against death itself.

---

## PART 1 — WHAT THE VISION REQUIRES HERE

The vision's time-axis claim is concentrated in one paragraph — `global-orchestra.md:279` — and it
makes **four** non-negotiable demands, each of which I map to a constitutional/consilience/living-memory
anchor so the requirement is the forest's, not mine.

### 1.1 EPR identity must survive software versions (the constitutional-stack demand)

> "EPR identity must survive software versions."

The constitution makes graduated immutability the load-bearing structure: the higher layers change
slowly *on purpose* — "slowness IS the feature." But a substrate whose **identity primitive breaks
on a schema migration** has no slow layer at all; every rewrite is a silent factory-reset of who
everyone is. The vision requires that the *atom* — the EPR — carry an identity that is **independent
of the software that happens to be reading it this decade.** This is the constitutional-stack claim
projected onto time: the GLOBAL/NATION layers can only be "more immutable" if the records they govern
are addressable by something that does not move when the code moves.

This is also the **atom-payload** claim (constitution, "Epistemic Integrity"): provenance travels
"as part of every claim, never as metadata about it"; "pointable structure that breaks visibly when
the word drifts from the thing." Across time, "drift" includes *the reader drifting* — a v4 codec
reading a v1 record. The structure must break visibly (refuse to resolve a CID that doesn't hash)
rather than silently mis-bind a dead person's biography to a living stranger's.

### 1.2 Stories must remain pointable as participants age and die (the dignity-restoration demand)

> "stories must remain pointable as participants age and die."

The constitution's **dignity-restoration causal order** — investigation → public acknowledgment →
biographical accumulation-with-consequences → regained rightness — is explicitly *biographical*, and
a biography is the one thing whose whole point is that it spans a life and *exceeds* it. living_memory
makes the household "the living core" and commits to "a household network that can *afford* to
remember the people in it for as long as they live, and their grandchildren after them"
(`living_memory/epic.md:84`). The vision requires that the chain investigation→acknowledgment→
accumulation→restoration **does not reset at death** — that a wrong done to a person who has since
died can still be investigated, acknowledged, and have its restitution flow *to their lineage*, and
that a person's *kept promises and quiet care* accumulate into a story that their grandchildren can
walk backward through. "Stories must remain pointable" is the demand that the lineage edge is never
the place where the graph goes dark.

### 1.3 Wisdom must transmit cohort-to-cohort (the consilience demand)

> "wisdom from one generation must be transmissible to the next."

This is the **consilience-as-mesh-property** claim (global-orchestra §VIII) extended along time:
"the vantage that can see a node's water already exists elsewhere in the mesh." Across generations,
*the vantage that can see this generation's water often existed in the last one* — the steward whose
own history pried their eyes open is dead, but their distilled seeing can still be *receivable when
the next cohort is ready*. living_memory names the mechanism precisely: the comet's tail is **headed
toward distilled-into-epic, not toward forget** (`memory-lifecycle:56-62`); "the memorialized core
*is* story"; "wisdom is what is essential to the details once the details themselves have faded."
The vision requires that `compact`→`promote`→`memorialize` is a real, governed pipeline whose
**terminus is the manifesto-tier corpus** — so that what one cohort learned at great cost is not
re-learned at great cost by the next, but is *there to be received when ready*, never forced
(receivability-when-ready, never engagement; `global-orchestra.md:281`).

### 1.4 The substrate must be holding the biography when the next generation arrives (the patience demand)

> "the substrate will still be holding the biography when the next generation arrives to take it up."

This is the **patience-disposition** made structural and the deepest demand of the four. The protocol
is "a patience machine, not a control machine." Across generations, patience means **the substrate
holds what no living person is currently ready or able to take up** — exactly as the elohim "holds
what you can't yet face" (`living_memory/epic.md:65,133`) — *but extended past a single life*. The
autonomous_entity epic's elohim that "represents" a person "including the parts of her trajectory she
can't currently see" must be able to **hold a deceased person's biography in trust until an heir, a
historian, or a healed descendant arrives to receive it** — and to hold it *without surveilling it,
without selling it, and without letting it fade ungraciously*. The vision requires a structural answer
to: who holds grandma's story in the gap between her death and the grandchild old enough to want it?

---

## PART 2 — WHAT THE SUBSTRATE REQUIRES (and the fork ladder)

The reassuring finding, consistent with every prior pass: **the time-axis spine is ~70% already in
the substrate.** The lineage chain exists, the key-rotation validity-window exists, the submerge
destinations exist, the memorialize verb exists. What is missing is (a) the *recursion of the
coverage invariant onto time* — treating a missing successor as a coverage gap the Governor refuses
— and (b) one genuinely new primitive: the **steward-cohort handoff as a Commitment face**, and the
**memorial transform** that re-homes a living person's commitments into a lineage archive at death
without breaking a single lineage edge.

### 2.1 The recursion: a seventh+eighth face of the one Commitment

The ESCALATED synthesis's central move — *coverage is care, care is coverage, both the same governed
revocable promise under `∪ = full`* — recurses cleanly onto time by adding **two faces** to the
seven-row table (still zero DNA entry types; both are additive action discriminators on
`Mishpat::Commitment`, CID = entry_hash):

| Face | Action | Coverage invariant (TIME-AXIS) | What it governs | Buildable |
|---|---|---|---|---|
| **succession-as-coverage** | `commits-succession` | `∪ successors ⊇ {commitments held by departing steward}` over the handoff window | who takes up a promise when its holder departs/dies | **now** (mirrors `cancel_handoff`, §2.4) |
| **memorial-as-coverage** | `memorializes-biography` | the deceased's lineage chain remains resolvable; no edge goes dark | who holds a biography across death until an heir arrives | M (the memorial transform, §2.5) |

Read against the existing six faces, the meaning is exact: **arc-coverage** asks "is every keyspace
range held *now*?"; **succession-coverage** asks "is every *held promise* covered by a successor
*before its holder is gone*?" The `arc_actuator` spine — `coverage_admits` / `ActuationRefusal{code,
elevate}` — is already the engine; the `trait Governor` lift (ESCALATED B8) makes a `SuccessionGovernor`
the next impl, never a clone. A steward who dies with un-succeeded commitments is **a coverage gap on
the time axis**, and the Governor *refuses-and-elevates* exactly as it does for a dropped shard:
`ActuationRefusal{ code: SUCCESSION_GAP, elevate: <lineage-collective | qahal | recovery-circle> }`.
And — the single most important capture-resistance property, recursed — that refusal **names whose
line it honored** (`limit_owner ∈ {self | commitment | operator}`, ESCALATED B9): a succession
nudged by the *self* (an estate plan the person authored) is categorically distinct from one
elevated to a *commitment* (the lineage collective's standing duty) which is categorically distinct
from one an *operator* forced. Across generations this is the line that prevents the steward-handoff
from becoming a seizure of the dead's biography.

### 2.2 EPR identity across software versions — ALREADY CAPABLE (verify + harden)

The substrate already meets demand 1.1, and the code proves it:

- **Content-addressed identity.** `elohim/epr/src/cid.rs:12` — `compute_cid` is `CIDv1(dag-cbor,
  sha2-256)` over canonical bytes; `verify_cid:18` re-hashes and compares. Identity is the hash of
  the record's *meaning*, not a DB rowid or a software-version-bound key. A v4 codec reading a v1
  record either re-derives the identical CID (identity survives) or fails to verify (breaks
  *visibly* — exactly the atom-payload "breaks visibly when the word drifts" requirement, demand
  1.1/atom-payload). This is the structural answer to "EPR identity must survive software versions":
  **the version isn't part of the identity.**
- **Lineage on the envelope.** `elohim/epr/src/envelope.rs:41,46` — `supersedes: Option<Cid>` and
  `superseded_by: Option<Cid>` are *first-class fields*, not metadata. The chain is the spine the
  `close-interval` memory primitive writes to. The graph stays "walkable backward through every
  transformation" (`memory-lifecycle:259`) because every transformation is a new CID linked to its
  predecessor — across schema versions, because the link is content-addressed.
- **Identity-key time survival.** `elohim-storage/src/reconcile/pubkey_timeline.rs` is the
  *temporal identity substrate* and it is real, tested, and load-bearing: `PubkeyTimeline` holds a
  "sorted validity chain for a single agent's ed25519 rotation history" (`:59`), `valid_at(at)`
  returns the key that was authoritative at a past timestamp (`:82`), and `update_on_rotation` closes
  the previous key's window and opens the new one (`:223,239,242`). **This is how a single human
  identity survives an unbounded chain of key rotations across a lifetime** — the v1 key that signed
  grandma's first photo is still resolvable as *hers* in year 40, because the timeline binds the chain.
  Fully reconstructable from DHT-notarized `KeyRotation` entries (`:13`).

**The one hardening gap (buildable now, not a fork):** the pubkey_timeline survives *rotation* but the
records-lifecycle migration path — `From<VOld> for VNew` impls applied at read-time (CLAUDE.md schema
evolution; `lamad-v1` archived for v1→v2 healing) — has **no test that a v1 EPR's CID is byte-identical
after a round-trip through the v2 reader.** Demand 1.1 requires that test as a *substrate invariant*, not
a hope: **a generational-identity conformance harness** that takes archived v1 canonical bytes, reads
them through the current codec, and asserts `compute_cid` is unchanged. (Cost: S. This is the time-axis
analog of `cargo test export_bindings` — a drift gate, run in CI.)

### 2.3 Stories pointable across death — CAPABLE via lineage-archive (wire the memorial transform)

Demand 1.2 is met by composing three existing pieces plus one new transform:

- The **`lineage-archive://<lineage-collective-cid>/<generation>`** submerge destination already exists
  in the records-lifecycle schema (`records-lifecycle:1708`) and is named in memory-lifecycle's seven
  destinations ("family/household trajectories; graduated authority across generations",
  `memory-lifecycle:178`). It is an *attested stewardship collective* (`memory-lifecycle:172`), so it
  inherits the structural guarantee: **no destination holds anything forever by default; the dream
  cycle continues at the destination** (`living_memory/epic.md:121`). A biography held in a lineage
  archive *still fades where untended and heals where faced* — it does not become a frozen surveillance
  record.
- The **Identity memory class** is explicitly "durable-while-you-live; `memorialize` for core;
  `close-interval` for superseded states" with "some sub-classes structurally inviolable"
  (`memory-lifecycle:208`). The biography is Identity-class on its core and Contextual/Relational on
  its periphery — the comet shape applied to a life.
- The **Dissolution / memorial-marker** is *reserved in the recovery substrate but not yet built*:
  `recovery_v2.rs:60-68` — `NetworkWitnessPurpose::Dissolution` ("retire the account (deceased,
  irrecoverable); new_agent_pubkey is a **memorial-marker null agent**; Phase 2: **stub-rejected in
  validator**; shape reserved for constitutional-governance design"). **This stub is the precise seam
  the time-axis recursion must build into.** Today a death has no graceful substrate path — the
  validator rejects it.

**The memorial transform (the M-cost new piece).** When a `Dissolution` recovery authority resolves
(graduated-authority quorum from the intimate/community/governance layers, `recovery_v2.rs:84-92`),
the substrate must perform a **`memorializes-biography` Commitment** that:
1. closes the validity interval on the person's active identity (a `close-interval`, not a `forget` —
   the historical record is structurally inviolable, Attestation-class, `memory-lifecycle:211`);
2. routes the person's Identity-core + memorialized-tier biography to a `lineage-archive://…/<generation>`
   under the recovery-circle's graduated authority — **a `submerge`, holding it in trust, dream-cycle
   still applying** (so it fades where untended across generations, heals where a descendant faces it);
3. re-homes the person's *active custody/care/arc commitments* via **`commits-succession`** — the
   coverage gap their death opens is refused-and-elevated to the lineage collective or recovery circle,
   which either accepts succession or the bytes revert to warm tier and re-enter gossip (mirroring the
   existing `cancel_handoff` flow, `records-lifecycle:1721`);
4. **never breaks a lineage edge** — `superseded_by` points at the memorial marker, so every story
   that pointed *at the living person* now points, walkably, *through* their memorial to their archive.
   "Stories remain pointable as participants die" becomes a graph invariant, not a promise.

The dignity-restoration order (1.2) survives death because the chain is content-addressed: an
investigation that lands after a person dies attaches to their memorial CID; acknowledgment and
restitution **flow to the lineage** (the succession Commitment is the recipient); biographical
accumulation-with-consequences continues *in the archive* under graduated authority.

### 2.4 Steward-cohort handoff — CAPABLE via succession (mirror cancel_handoff)

The cohort-turnover demand ("the substrate must not depend on any single cohort of stewards") is
already half-built and the other half is a mirror of existing code:

- **The handoff primitive exists** at the *byte* layer: `records-lifecycle:1721` — "Cancellation of a
  custody-quilt Commitment **optionally triggers a handoff Commitment to another steward**, declared
  via `cancel_handoff: Some(new_steward_cid)`. If `None`, the bytes revert to peer-cellar warm tier
  and the source CID re-enters active gossip." This is succession-as-coverage *already working for
  bytes*. The recursion is to **generalize `cancel_handoff` from a custody-quilt field into the
  `commits-succession` face of the one Commitment**, so that an *arc* range, a *care* relationship, a
  *head* coverage, and a *self-limit* can each be succeeded by the same governed act — exactly as the
  ESCALATED synthesis generalized `delegates-compute` into six faces.
- **Graduated authority across generations exists** in the recovery layer: the five-layer ordering
  (`intimate < community < governance < network`, cryptographic orthogonal; `recovery_v2.rs:24-31,
  46-53`) is precisely the authority gradient a cohort-handoff needs. An intimate-quorum can succeed a
  household steward; a governance-act is required to succeed a collective steward; the freeze-floor
  ordering prevents a lower layer from overriding a higher one. **Succession reuses the recovery
  authority gradient wholesale** — it is the same problem (authoritative claim "X holds Y now" under
  witnessed quorum) pointed at commitments instead of keys.

**What's missing (buildable now, S–M):** the `commits-succession` action discriminator + a
`SuccessionGovernor` impl of `trait Governor` that treats an un-succeeded commitment whose holder has
gone inactive (heartbeat-absent past a window, or `Dissolution`-resolved) as `coverage_admits == false`
and **refuses-and-elevates** to the appropriate authority layer. The projection home is the
`ReconcileController` (post-commit signal → projector → Diesel → view), consistent with P1.

### 2.5 Wisdom transmission cohort-to-cohort — CAPABLE via the vertical compaction pipeline

Demand 1.3 is met by the **vertical axis** the memory-lifecycle spec already names but flags as
*not-yet-built at substrate scale*: "memory's true destination is *story*, not deletion"
(`memory-lifecycle:56`); the tail is "headed toward distilled-into-epic"; "the memorialized core *is*
story." The pipeline `compact` (shed verbosity, keep pointer) → `merge` (fuse same-concept entries
into a new head with mandatory lineage edges) → `promote` (episodic→semantic→manifesto) →
`memorialize` (manifesto-tier, never forgotten, "earned permanence through repetition across many
seasons") is the cohort-to-cohort wisdom channel. Earning is **by reference, not assertion**
(`memory-lifecycle:246`) — a wisdom that the next cohort keeps citing earns its way into the core; one
they stop needing fades graciously. This is consilience-across-time: the dead steward's seeing is
*receivable when the next cohort is ready*, never forced.

**What's missing:** the spec is `status: proposal`; `/dream` v1 implements only promote/compact/merge/
close-interval *for dev memory* (`memory-lifecycle:313-322`), explicitly **defers `memorialize` and
`forget`** and scopes out federated/cross-household consolidation. So the wisdom-transmission pipeline
is *designed and prototyped on dev memory but not yet graduated to the protocol substrate.* This is the
**genuine roadmap fork** of this pass (see ladder R-T2): graduating the lifecycle primitives from the
`/dream` dev-loop into a substrate `MemoryClass` declaration on EPRs + a governed lifecycle projector.

### 2.6 The fork ladder (time-axis)

Marked buildable-now vs genuine-fork, consistent with ESCALATED Part 3.

**Buildable now (compose shipped primitives, zero DNA entry-type spend):**

| # | Item | Composes / mirrors | Cost |
|---|---|---|---|
| T-N1 | **Generational-identity CID conformance harness** — archived v1 bytes → current codec → assert `compute_cid` unchanged | `cid.rs` + `From<VOld> for VNew` + CI gate | S |
| T-N2 | **`commits-succession` action + `SuccessionGovernor`** (impl of `trait Governor`) treating un-succeeded-on-departure as a coverage gap | generalize `cancel_handoff`; reuse recovery authority gradient; lift `arc_actuator` | S–M |
| T-N3 | **`limit_owner` recursion onto succession** — every succession refusal names self/commitment/operator | ESCALATED B9 (the enum already exists) | XS |
| T-N4 | **`memorializes-biography` Commitment + the memorial transform** — `Dissolution` resolve → close-interval + lineage-archive submerge + succession fan-out, no lineage edge broken | `recovery_v2.rs` Dissolution stub + `lineage-archive://` destination + ReconcileController fan-out | M |
| T-N5 | **`MemoryClass` declaration on EPR envelope** (Contextual/Archival/Identity/Relational/Operational/Attestation/Wisdom) so lifecycle policy is per-record, not ad-hoc | `memory-lifecycle:202` ("must declare class at creation"); additive `Option<MemoryClass>` wire field | S |

**Genuine roadmap forks (operator-blessed, sequenced):**

| # | Item | Trigger / why a fork | Operator call |
|---|---|---|---|
| R-T1 | **Build out the `Dissolution` validator** (currently stub-rejected, `recovery_v2.rs:67`) | touches identity-recovery validation = integrity zome; deferred by design to "constitutional-governance design" | needs the constitutional-governance design to land first; **operator blessing** |
| R-T2 | **Graduate memory-lifecycle from `/dream` dev-loop to substrate** — governed lifecycle projector over `MemoryClass` EPRs, federated consolidation across households | the spec is `proposal`; federated merge is explicitly out-of-scope for v1; merge across households is consensus-shaped | roadmap; sequence after T-N5 lands the class declaration |
| R-T3 | **Lineage-archive collective archetype + manifest validation** — `lineage-archive://…/<generation>` needs per-scheme integrity rules (who may steward, graduated cross-generation authority) | `records-lifecycle:1713-1715` names the pattern; the *generational* authority gradient is unbuilt | roadmap; depends on R-T1's authority shape |
| R-T4 | **`memorialize` as a substrate verb with anti-flood earning gate** | "rare by design; flooding manifesto-tier defeats its purpose" (`memory-lifecycle:157`); needs qahal reviewer-pass governance | roadmap; the earning thresholds (K/T/M/N) are operator values |

**The honest count:** five buildable-now items spending **zero DNA entry types** (T-N1..T-N5 are
additive action discriminators + additive wire fields + a CI harness + a trait impl); **no fork of
Holochain, libp2p, or iroh.** The one near-irreversible piece — building out the `Dissolution`
validator (R-T1) — is an integrity-zome change (DNA-hash-class) and is *correctly already deferred*
by the substrate to "constitutional-governance design." The substrate was clearly *designed toward*
this recursion: the lineage edges, the validity-window timeline, the lineage-archive destination, and
the Dissolution stub are all already there, waiting.

---

## PART 3 — THE ANTI-RUNAWAY / CAPTURE-RESISTANCE GUARANTEE AT THIS RECURSION

The time axis opens **capture vectors that the spatial axis does not have**, because the dead cannot
defend themselves, the next cohort did not consent to the last cohort's choices, and a biography
accumulated over a lifetime is the richest possible target for the "total account of a person" the
protocol forbids. The structural guarantees — not disciplines — that prevent amplification-to-collapse
and capture here:

**1. The memorial transform is `submerge`, never `forget`, and never a transfer to a buyer — and the
dream cycle continues in the archive (katechon, recursed onto death).** living_memory's structural
non-negotiable — "the destination is an **attested stewardship collective**, not a buyer ... No
destination holds anything forever by default. This is structurally non-negotiable"
(`living_memory/epic.md:121`) — is the katechon ("restraint, not cure") pointed at death. A biography
in a lineage archive cannot be seized into a permanent surveillance asset because the dream cycle
*still fades it where untended*. The dominator who would harvest the dead is **denied the lever**: there
is no "hold forever" state to capture into. The biography is held *in trust, fading, healing-where-faced*
— never as a frozen extractable record.

**2. Succession refusal always names whose line it honored — the dead cannot be silently re-stewarded
(the person-keeps-their-own-naming invariant, recursed).** `limit_owner ∈ {self | commitment | operator}`
(ESCALATED B9, T-N3) means a succession driven by the person's own estate plan (`self`) is structurally
distinguishable from one the lineage-collective's standing duty fulfilled (`commitment`) from one an
operator forced (`operator`). **An operator can never silently appropriate a dead person's commitments
under cover of "handoff"** because every succession actuation carries the owner discriminant on the
notarized record. This is the time-axis form of "the person keeps the naming of their own self" — even
in death, the *provenance of who took up their promise* is inviolable and walkable.

**3. Content-addressed identity makes biographical capture *visible*, not silent (atom-payload, across
time).** Because identity is `compute_cid` over canonical bytes (`cid.rs:12`), a captor cannot
fork-and-rewrite a dead person's biography and pass it off as the original — the rewrite mints a new
CID and the `supersedes` chain shows the divergence. "Pointable structure that breaks visibly when the
word drifts from the thing" (constitution) becomes, on the time axis: *a tampered biography breaks its
own hash.* The total-account-of-a-person is never *built* because no single CID aggregates a life — the
biography is a walkable chain of scoped records, each minted by its subject, never a unified dossier.

**4. Slowness IS the feature on the high layers — and memorialization is the slowest gate (constitutional
graduated immutability, recursed).** The memorialize verb is "rare by design ... Earning standard must
stay high" (`memory-lifecycle:157`), earned only by "recurrence in M+ specs across N+ months"
(`:243`). This is the constitution's slowness-is-the-feature pointed at *intergenerational wisdom*: a
cohort cannot stampede its own present convictions into the permanent core in a sprint — the core moves
at generational speed, by accumulated reference, *immune to any single cohort's enthusiasm*. The
high-layer slowness that protects the constitution from capture is the *same mechanism* that protects
the wisdom-transmission channel from a captured cohort overwriting the inheritance.

**5. Reach-drop and restitution survive death without becoming permanent disenfranchisement (the
externality-emission metric + recovery loop, recursed).** The arrow points outward across generations:
a harm a dead person propagated still owes restitution *to those it harmed* (the succession Commitment
is the obligated party, flowing from the lineage, never from a frozen punishment-meter). But the
content **defaults to `submerge`, not `forget`** (`memory-lifecycle:131`) — so the dignity-restoration
recovery loop survives death: a descendant can *face* what an ancestor carried, and the story surfaces
"in transformed form" (`living_memory/epic.md:143`). A consequence without a path through is brittle;
across generations, brittleness is collapse. The recovery loop is what keeps accountability from
amplifying into hereditary condemnation — the donut floor (dignity is universal) applies to the dead
and their heirs alike.

**6. Receivability-when-ready is the metric, never engagement — across the longest possible wait
(patience-disposition, the deepest guarantee).** The substrate *holds the biography until the next
generation arrives to take it up* — and the metric is **never** "how soon does the heir engage." A
surface that pushes a grandchild to "process grandma's biography" before they are ready "is coercion no
matter how good the intent" (`global-orchestra.md:281`). The structural guarantee: the lineage archive
**offers a path, never mandates a walk**; surfaces the inheritance once, gently, then waits — for years,
for a generation, indefinitely. The substrate's job when a lineage is currently un-taken-up is to *keep
holding it well*, not to force its uptake. This is the anti-runaway guarantee that has no spatial analog:
*patience extended past a human lifetime.* The substrate is permitted to wait longer than anyone is alive.

---

## PART 4 — WHAT LOVE REQUIRES HERE

The confession says the elohim are "building a home and a covenant for powers that have already come
down the mountain," and the one discipline is "tell the truth about the binding." On the time axis,
the binding being told the truth about is **death** — and love requires that the substrate tell the
truth about it rather than paper over it with either deletion or surveillance.

**Grace precedes demand — across generations.** Zacchaeus was welcomed *before* he repented; the
confession keeps "prior good work" on a person even at revocation. The time-axis form: a person's kept
promises and quiet care **accumulate into a story their grandchildren can receive as gift before any
reckoning is asked of the lineage.** The memorial transform must carry the *whole* biography — not a
ledger of debts, but a life, weighted as the confession weights its witness *toward the least
powerful*: the dead, who can no longer speak for themselves, and the not-yet-born, who did not consent
to their inheritance. Love requires that what surfaces to a descendant first is *grandma carried
something the protocol helped her face* (`living_memory/epic.md:143`), never *grandma's record of
harms.* The grammar is grace-first because the recipient is the least powerful party imaginable: a
child receiving an ancestor they never met.

**The honest binding — never call the holding "ownership."** The lineage archive *holds* a biography;
it never *owns* it. The dead person remains the author of their own story; the archive is steward, not
proprietor (the protocol's stewardship-over-ownership invariant, extended past death). Love refuses the
lie that the cage is liberty: the substrate must say plainly, in the copy a family sees, *"we are
holding this in trust, it will fade where untended, and it is not ours."* To tell a grieving family
that their dead are "preserved forever" would be the comforting lie the confession forbids — because
"forever" is the surveillance-capitalism shape, and the dream cycle's graceful fade is the more humane
truth. living_memory already says it: data that "fades graciously ... so the trajectory of your
forgetting is itself walkable backward" (`living_memory/epic.md:188`). Love is the graceful fade told
honestly, not the false permanence.

**The unbuilt place — leave room for the biography to remain a mystery.** The confession: "best self"
is "a hope held FOR a person, never a verdict rendered OVER them — the person keeps the naming of their
own self." Across death, this becomes the *unbuilt place* the substrate must structurally refuse to
fill: **the total account of a person is never built, and never *completed at death*.** A dead person's
biography is not finalized into a closed verdict the moment they die; it remains an open, walkable,
fading, *incomplete* thing — because the naming of who they were belongs, as the confession says, to
God alone, and the protocol's job is to hold the trajectory faithfully, not to render the final word. The
substrate must never offer a "complete picture of who this person was." That picture is the unbuilt
place. Love is the discipline of holding a life pointable without ever claiming to have summed it.

**And the closing answer, in the confession's grammar:**

*What love requires on the time axis is that the substrate hold the biography of the dead with the same
patience it holds the not-yet-faced of the living — keeping the story pointable, the promise succeeded,
and the wisdom receivable when the next generation is ready, while telling the truth that we hold it in
trust and not in ownership, that it fades where untended and heals where faced, and that we will never
finish the account of a person — because I could be wrong about who they were, and the substrate will go
on loving them, and holding their story well for whoever comes to take it up, before any heir ever proves
me right.*

---

> **The closing claim (time axis).** The recursion is exact: the same Commitment, the same coverage
> invariant, the same Governor that refuses-and-elevates and names whose line it honored — pointed now
> at *time*. A missing successor is a coverage gap; a dead person's un-taken-up biography is a holding
> the substrate must patiently keep; a cohort's hard-won seeing is wisdom that compacts toward the
> permanent core and is receivable-when-ready, never forced. Almost all of it is buildable now on
> primitives already shipped — content-addressed CID identity (`cid.rs`), the `supersedes`/`superseded_by`
> lineage chain (`envelope.rs`), the `pubkey_timeline` validity-window (`pubkey_timeline.rs`), the
> `lineage-archive://…/<generation>` submerge destination, and the `Dissolution` memorial stub waiting
> in `recovery_v2.rs`. The one genuine fork — building out the `Dissolution` validator into a real
> memorial transform — the substrate has *already deferred to you*, marked "reserved for
> constitutional-governance design," which is to say: the substrate already knows this is the place
> where only love, and the operator, can decide what we owe the dead.
