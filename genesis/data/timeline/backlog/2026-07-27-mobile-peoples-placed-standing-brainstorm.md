---
id: "backlog-mobile-peoples-placed-standing"
kind: "backlog"
contentType: "backlog-entry"
contentFormat: "markdown"
title: "Mobile peoples and placed standing — how the itinerant hold mutuality without a parcel"
slug: "mobile-peoples-placed-standing"
written: "2026-07-27"
author: "Matthew Dowell + Claude Opus 5"
status: "backlog"
domain: D-ontology
severity: medium
source: "sidebar during the Freenet peer-confrontation survey (2026-07-27) — reach/place ontology thread"
relatedNodeIds: []
tags: [reach, place, mishpat, imago-dei, valueflows, ontology, brainstorm, h3, subsidiarity]
---

**This is a brainstorm seed, not a task.** It wants its own session. Captured because the
distinctions below were worked out live and are easy to lose.

## Why it needs its own session

The protocol's commons standing is currently **placed** (Place-scoped, `governing_collective_id`),
while reach is **relational** (`intimate / trusted / familiar / community` resolve through
relationships, not coordinates). Those two facts are individually good and jointly incomplete:
a person who is mobile carries their reach with them and loses their standing at every border.

If the protocol is for everyone, itinerant people and groups are imago dei, and a design where
standing silently requires a fixed address excludes them by construction — quietly, without anyone
deciding to.

## The four-layer distinction (the part worth preserving)

The hazard here is **category**, not vocabulary. Collapsing these four into "itinerant" gets the
first wrong by erasing ethnicity and the fourth wrong by treating coercion as preference:

1. **Ethnic peoples with mobile traditions** — Roma, Sinti, Irish Travellers, Yenish, Sámi,
   Bedouin, Fulani, Maasai. Protected characteristics. Naming is **self-determined**.
2. **Livelihood mobility** — transhumant pastoralists, seasonal agricultural workers, fisher and
   riverine peoples, fairground and circus families.
3. **Chosen lifestyle mobility** — van-dwellers, liveaboards, digital nomads, intentional communities.
4. **Forced displacement** — refugees, IDPs, unhoused people, migrant labourers. **Mobility is
   imposed.** A design that treats this like (3) will be cruel.

### Terminology reference

- **Roma** (noun), **Romani** (adjective and the language). *Gypsy* is an exonym widely rejected —
  though some communities self-identify with cognates (*Gitano* in Spain, *Romanichal* in the UK),
  and Scotland officially uses *Gypsy/Traveller* because that community chose it.
  **Self-ascription governs.**
- **Irish Travellers** — a distinct ethnic group, **not** Roma. Capital T. Endonyms *Mincéirí*, *Pavee*.
- **Sinti** — distinct from Roma. European umbrella when one is needed: *"Roma, Sinti and Travellers"*
  or *"Romani peoples."*
- **"Vagrant"** — drop it; a criminalising legal term. Use *people experiencing homelessness*,
  *unhoused*, or *rough sleeping* (UK).
- Neutral umbrellas that don't flatten: **mobile peoples**, **nomadic and semi-nomadic peoples**,
  **transhumant**, **peripatetic peoples** (anthropological term for itinerant service-providing groups).
- Avoid as loose metaphor: *wanderers, drifters, nomads* for anyone unsettled.

## The architectural rule this implies

**Peoplehood does not belong in a DNA-notarized enum.** `reach` belongs there — a protocol primitive
with fixed semantics. Ethnic and cultural categories are the opposite: self-ascribed, contested,
evolving. A notarized enum is effectively immutable (changing it moves the DNA hash), so hard-coding
a category would make an outside party the permanent authority on who someone is — the
identity-sovereignty failure in a different costume.

The corpus already holds the right pattern: `place-type` carries `custom = "community-defined"`.
Extend that principle — **the vocabulary is extensible by the community it describes**, and the
notarized fact is the group's own *witnessed self-ascription* (`elohim/epr/src/witness.rs`), never a
category chosen for them.

## The design question, stated well

> **Reach travels; standing is placed. The session's work is the translation function between a
> portable relational footprint and a non-portable placed standing.**

### Answer sketch already latent in the ontology

The H3 ladder may answer it with no new primitive. `objects/h3-cell` documents the mapping:
*"res 0-2=global, res 3=bioregional, res 5=municipal, res 7=neighborhood, res 9=parcel"*, and
`Place.parent_place_id` already models nesting (*"parcel → community → bioregion → global"*).

So standing can be held at a **coarser resolution**: you belong to a bioregion (res 3) without
holding a parcel (res 9). The settled householder holds standing at res 9; the traveller, the
seasonal worker, the hermit hold it up-ladder. **Not a lesser standing — a differently-scoped one.**
Mutuality without a fixed address.

## Valueflow legibility — what elohim would negotiate

Mobile peoples' contributions are usually invisible to *sedentary* accounting rather than absent
from it: seasonal labour that registers as someone else's harvest, route and corridor maintenance,
seed and livestock genetic stewardship, knowledge and repertoire transmission, market-making between
settled communities that never trade directly, ecological work of grazing rotation.

ValueFlows can express all of it; the ontology is not the blocker. The blocker is that a contribution
made across four localities currently registers in none. The res-ladder sketch reframes it: **a
contribution made along a route accrues at the resolution that contains the route.** That is the
thing elohim argues on their behalf — not special-casing, but arguing the correct *resolution* for a
contribution whose shape a rooted counterparty cannot see from a parcel.

## Guard rails for the session

- **No-conscription, applied to place.** The same `imago-dei` principle that forbids conscripting a
  node into hosting forbids regularising a person into an address. Hovering is a legitimate mode,
  not an exception carrying a penalty.
- **Attested location attaches to things, never to people.** `Place` is safe because it attests
  parcels and boundaries. An attested geofence on a *thing* is governance; the same attestation on
  an *agent* is precisely what reach exists to prevent. Write this as a rule before anyone adds a
  location field to a Human.
- **(4) is not (3).** Forced displacement needs a different obligation shape than chosen mobility.
- **Free movement is the assumed good.** Design for how a footprint *translates* on leaving a
  locality, not for how it is retained by staying.

## Anchors in-tree (verified 2026-07-27)

- `Place` entry + `validate_place` — `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:161,586`.
  Carries `h3_index`/`h3_resolution`, `geometry_json`, `parent_place_id`, `governing_collective_id`,
  `constitutional_layer`, `carrying_capacity_json`, `status` (`active|proposed|disputed|dissolved`).
- Spatial schemas — `elohim/sdk/schemas/v1/objects/{geo-point,h3-cell}.schema.json`,
  `enums/{place-type,place-status,spatial-context-type,osm-element-type}.schema.json`.
  **All carry `_dna: NONE`** — validated as entries, but not notarized protocol vocabulary like `reach`.
- Reach — `elohim/sdk/schemas/v1/enums/reach.schema.json` (8-level ordinal, DNA-notarized
  `REACH_LEVELS`); `elohim/epr/src/reach.rs` (`openness()`, `Ord` deliberately not derived).
- Composition seams for place-governed reach (verified): `Place.governing_collective_id` → collective →
  `check_reach_authorization`'s `community` arm (works today, no schema change); and
  `bounds` is an open JSON map with a per-commitment `reach_ceiling`, so a `place_scope` can sit
  beside `epr_scope` with no Commitment entry change —
  `elohim/elohim-storage/src/services/bounds_validator.rs:187-215`.
- **Invariant to preserve:** place never becomes a reach level. Place selects *which collective or
  commitment governs*; reach stays the ordinal on openness. Two axes composed at the commitment,
  never merged in the enum. (This is what the 2026-07-22 reach-ontology split bought — had the
  geographic-8 stayed inside reach, this composition would be inexpressible.)

---

# RAW CAPTURE — in-flight findings, 2026-07-27

**Status: unrefined.** A deep dive was started and deliberately stopped so it can be done properly
in its own session. What follows is what was already on the table — prior art located, one
self-correction, and extra in-tree anchors. **Nothing here is concluded.** Treat it as a warm start,
not as findings.

## Prior art located (four traditions, all directly on-point)

**1. "Open property regimes" — Moritz et al., *International Journal of the Commons*
([ijc.719](https://thecommonsjournal.org/articles/10.18352/ijc.719), and
[ijc.903](https://thecommonsjournal.org/articles/10.18352/ijc.903)).**
The single most relevant find. The standard four tenure categories (open access / common property /
private / state) **cannot describe mobile pastoral systems**, so they propose a fifth:

> *"open access does not mean the absence of rules; instead it refers to the right that every
> pastoralist has to common-pool grazing resources."*

Open access **is** the rule; informal institutions *facilitate* rather than restrict. No one can be
excluded. Reciprocity guides management without formal enforcement. The system is explicitly a
complex adaptive system — *"large networks of components with no central control and simple rules of
operation give rise to complex collective behavior"* — where autonomous mobility decisions produce an
**ideal free distribution** (grazing pressure matches resource distribution) with no central
coordination. Five success conditions: resource variability, user mobility, **point-centred land use
(overlapping radii, no territorial boundaries)**, household autonomy, disequilibrium rangelands.

*Cross-link worth noting:* this is structurally the same move as the Freenet **"emergent refusal"**
finding in the peer-confrontation survey — no admission machinery, ordering emerges from autonomous
local decisions. Two unrelated fields arriving at the same shape is a signal.

**2. Sámi *siida* + usufruct + *alders tids bruk*.** The right to reindeer husbandry is a
**usufruct** — a use-right that *"applies over certain land areas regardless of the ownership of
those lands."* The Norwegian Supreme Court (30 June 2021) upheld Swedish herders' cross-border rights
grounded in **time-immemorial use**, tracing to the 1751 Border Treaty addendum (Lapp Codicil).
The siida is a collective whose members hold individual resource rights but co-manage — and it is
**not territorially defined**. *The right is grounded in demonstrated recurring use, not in
registration or residence.* This is the strongest candidate lift: standing earned by **witnessed
pattern of use** — and `elohim/epr/src/witness.rs` is already the primitive.

**3. ECOWAS International Transhumance Certificate** — a portable credential certifying itinerary and
herd fitness, resting on the free-movement principle
([IOM](https://publications.iom.int/system/files/pdf/iom_ecowas_pastoralism.pdf),
[ECOWAS](https://ecpf.ecowas.int/wp-content/uploads/2016/01/CrossBorder-Transhumance-WA-Final-Report-1.pdf),
[Davies et al. review](https://pastoralismjournal.springeropen.com/articles/10.1186/s13570-020-00168-z)).
**The lesson is the failure mode:** twenty years of poor domestication; Nigeria never implemented the
certificate requirement. Issuing a portable standing credential is the easy half — **the hard half is
the receiving collective honouring it.** That is precisely the negotiation elohim would conduct, and
a warning against building the credential and calling it done.

**4. Negotiated Stopping (LeedsGATE, UK)** — the working model, and it is a *ceremony*, not a permit:
agree **which land**, **how long**, **which services** (water, waste, sanitation), and **contribution
to costs**, with community, landowner and authority all party
([Friends, Families and Travellers briefing](https://www.gypsy-traveller.org/wp-content/uploads/2021/02/Briefing-Accommodation-February-2021-1.pdf),
[GOV.UK traveller sites policy](https://www.gov.uk/government/publications/planning-policy-for-traveller-sites/planning-policy-for-traveller-sites)).
Reciprocal obligations on both sides. Critically, **it was authored by the Traveller organisation
itself** — matching the "vocabulary extensible by the community it describes" rule above. Context:
the Criminal Justice Act 1994 repealed the local-authority duty to provide sites, and campaigners
seek its reinstatement — i.e. the absence of an obligation is what produced the crisis.

**5. Portability of social protection**
([IZA World of Labor](https://wol.iza.org/articles/the-portability-of-social-benefits-across-borders/long),
[DCI](https://spdci.org/resource/making-social-security-portable-across-borders/),
[habitual residence test](https://en.wikipedia.org/wiki/Habitual_residence_test)).
Only ~23% of migrants are covered by bilateral agreements; habitual-residence tests block export by
design. The transferable technique: **separate risk pooling, pre-funding, and redistribution** in
benefit design, because *they port differently*. Direct analogue for decomposing commons standing
into components with different portability — contributed value (ports), residence-conditioned
provision (doesn't), redistribution (jurisdiction-bound).

## Self-correction — the H3 res-ladder sketch is not sufficient

The "hold standing at a coarser resolution (res 3 bioregional rather than res 9 parcel)" idea above
**imports the territorial assumption it was meant to escape.** H3 is a *tessellation*: coarsening it
yields bigger boundaries, not the absence of boundaries. Open property regimes are **point-centred
with overlapping radii and explicitly no territorial boundaries** — the opposite shape.

`Place` is territorial by construction (`geometry_json` polygon + H3 cell + `parent_place_id`
nesting). So the ontology as it stands may be able to express *where a mobile people is allowed*
but not *how they hold standing without a bounded territory at all*.

Encouraging: the non-territorial primitives already exist —
`enums/spatial-context-type` is `point | area | route` (**route** is already a first-class kind), and
`objects/geo-point` carries an `accuracy` field. The session's real question may be whether standing
can attach to a **route** or a **point-with-radius** rather than to a Place.

## Additional in-tree anchors found in flight

- A whole spatial service layer exists, unexamined: `elohim/elohim-storage/src/api/spatial.rs`,
  `services/spatial.rs`, `services/spatial_capacity.rs`, `services/resource_nature.rs`.
- `spatial_capacity.rs` header states the enforcement chain verbatim:
  *"Resource → SpatialContext → Place → CarryingCapacity → enforcement"*, aggregating
  *"up the H3 hierarchy: parcel → community → bioregion → global"*, with
  *"Source of truth: Place.carrying_capacity_json (DHT-projected, Category A)"*.
  So the flourishing-floor/capacity leg is further along than assumed — and it is
  **Place-scoped**, which is exactly the coupling this backlog item interrogates.
- `resource_nature.rs` classifies resources by **Rivalry / Excludability / Depletability /
  Fungibility / CapacityModel / Circularity**. Grazing-type commons are the canonical
  low-excludability, rivalrous, replenishing case — likely the bridge between the Ostrom literature
  and our REA ontology, and worth checking whether "open property" is expressible in these dimensions.
- `relationship-type` enum is currently **structural only**
  (`contains, belongs_to, describes, implements, validates, relates_to, references, depends_on,
  requires, follows, derived_from, source_of`) — no social/kinship/co-presence relations. Reach's
  relational tiers resolve elsewhere; worth confirming where, since mobile mutuality would live there.

## What the proper session should do with this

Not more literature. The prior art is sufficient to start. The session's job is to decide whether
standing attaches to **Place** (territorial), to **route/point** (non-territorial), or to a
**witnessed pattern of use** decoupled from geometry entirely (the Sámi usufruct shape) — and then
to design the recognition ceremony (the ECOWAS failure) and the portability decomposition (the social
-protection technique).

## Open questions for the session

1. What is the translation function on leaving a locality — does standing decay, transfer, or
   re-scope up-ladder? Who witnesses the move?
2. Can standing be held at multiple resolutions simultaneously (res 3 bioregional + episodic res 9)?
3. How does a res-3 holder participate in a res-9 governance decision that affects their route?
4. What does `Place.status: disputed` mean for standing held inside a contested boundary?
5. Does the counter-cyclical share principle (protocol carries more where public provision is
   weaker — measured by flourishing-floor adequacy via `carrying_capacity_json`, **not** tax rate,
   which inverts in kleptocracies) interact with mobility? A traveller crossing provision gradients
   is the hard case.
6. What is the minimum witnessed self-ascription that makes a community's own vocabulary
   protocol-legible without a notarized enum?
