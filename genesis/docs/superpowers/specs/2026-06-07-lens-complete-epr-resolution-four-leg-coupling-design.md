---
title: "Lens-Complete EPR Resolution + the Four-Leg Coupling Law — closing the compose→resolve loop"
id: lens-complete-epr-resolution-four-leg-coupling-design
status: Draft
class: protocol-canonical
domain: D1
topic: [epr, resolution, coupling-law, typed-relation-graph, lens, cid, routing, doorway, rea-process]
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md
refines:
  - genesis/docs/content/elohim-protocol/architecture/2026-04-21-elohim-core-graph-substrate-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-04-18-experience-story-epr-design.md
cites:
  - experience-story-epr-design | defines EPR = EntityPortalReference (the content-addressed Resource this spec resolves) + contentFormat on ContentNode (the focal-render dispatch key) | sha256:b1dc5838ffab2e5d | path: genesis/docs/content/elohim-protocol/architecture/2026-04-18-experience-story-epr-design.md
  - records-lifecycle-design | the typed-relation vocabulary + lifecycle this spec walks as the lens-edges; the graph substrate the resolver traverses | sha256:3ebe9ccf2611bc02 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - pillar-epr-decomposition-design | §12.1 is the single-leg 302 behavior this spec corrects; §12.6 the universal /epr address being made lens-complete | sha256:8029079cea758380 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - epr-route-claims-link-conformance-design | the claims-302 (Model A) this spec demotes to a leg-preserving context lens; §5.1 classifier is the compose target for resolver dispatch | sha256:30b7cd1baf222922 | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md
  - epr-acquisition-pull-queue-design | §5.1 ClusterClosure is the bounded typed-relation closure-walk the resolver reuses; the dual-pin offline floor the focal render must satisfy | sha256:fc4a0cdd9828a377 | path: genesis/docs/superpowers/specs/2026-06-07-epr-acquisition-pull-queue-design.md
  - genesis/data/timeline/backlog/epr-routing-complementary-captures.md
  - .claude/skills/epr-content-addressing/SKILL.md
---

# Lens-Complete EPR Resolution + the Four-Leg Coupling Law

> **One-line:** the protocol already refuses to let an EPR be *composed* without its
> knowledge·value·governance legs — but *resolution* (`/epr/{id}`) currently drops them by
> collapsing an EPR to a single pillar mount. This design closes the loop: one typed-relation
> graph, a **four-leg** coupling-law (knowledge · value · governance · **process**) enforced at
> compose, and a **lens-complete resolver** that honors it at resolve. Pillar mounts, the
> claims-302, and the raw-node view all become *projections of the one graph.*

## 0. Provenance

Surfaced 2026-06-07 from an operator click: `https://alpha.elohim.host/epr/foundations-christian-technology`
302s to `/lamad/path/foundations-christian-technology` (a path; `contentType=path`,
`contentFormat=epr-composite`). The operator's framing — *"shouldn't it resolve to what the EPR
defines as its root, not the path itself?"* and *"every EPR has story+value+governance (and process)
… non-negotiable … EPRs should never be decoupled from these lenses"* — is the load-bearing insight
this spec formalizes. It is **not** a regression of the 2026-06-07 EprRouter fix (that fix repopulated
the router; this surfaced the *designed* §12.1 single-leg behavior that was simply invisible while the
router was empty).

## 1. The gap: the coupling-law is enforced at COMPOSE, dropped at RESOLVE

The three-legged coupling is **already canonical and structurally enforced at authoring**
(`.claude/skills/epr-content-addressing/SKILL.md` §"links carry three things"):

> 1. **Knowledge** — what the content is, how it connects.
> 2. **Value** — who stewards it, what value flows (via REA economic events).
> 3. **Governance** — what access rules apply, which community ratified it.
> **"You cannot create a link without all three. This is structural — the system enforces it."**
> *Information without governance is propaganda; value without attribution is extraction.*

Compose-side enforcement lives at `elohim/elohim-storage/src/services/epr_compose.rs`
(`reach_earning::evaluate` → `ReachVerdict`). **Reach is earned, never asserted** — the attestation
chain *is* the coupling.

**The defect:** resolution does not honor this law. `/epr/{id}` (pillar-epr-decomposition §12.1)
classifies *claimed commons-reach → 302 to the pretty mount; else → shell viewer*. A 302 to
`/lamad/path/{id}` collapses the EPR to its **knowledge leg alone** — value (who stewards it, the REA
flows) and governance (which community ratified it, the reach class) vanish from the lived surface.
The coupling-law is honored when the link is *born* and broken when the link is *walked*.

This is the routing analog of two settled protocol stances:
- *"Storage is projection, not truth"* — the pillar mount is a projection; the content-addressed
  resource is the truth.
- *"Doorway is OPTIONAL, not architectural"* (MAP D8) — a peer functions with zero doorway, so the
  resolver's **floor must be peer-native**, which a claims-registry 302 cannot satisfy offline.

## 2. The long-term pattern (operator-ratified 2026-06-07)

**One typed-relation graph over content-addressed Resources, with a coupling-law at every edge, and a
single lens-complete resolver that everything else projects from.**

- **Node** = an EPR = `EntityPortalReference` = a content-addressed Resource (CID; `EntryHash = CIDv1`),
  Category-A notarized over an existing `ContentNode` entry. (`experience-story-epr-design.md:39`.)
- **Edges** = **typed relations** (the vocabulary + lifecycle in `records-lifecycle-design.md`; the
  bounded closure-walk in `epr-acquisition-pull-queue-design.md` §5.1 `ClusterClosure`).
- **The lenses ARE typed-relation classes** on the resource. "story/value/governance/process" are not
  renderers you might offer — they are the coupled legs the graph guarantees.
- **Resolution = a closure-walk** that renders the **focal content** (the EPR's self-described format
  root) *wrapped in its coupled legs*. The focal render is the **offline floor**; the legs are drawn
  from the resource's typed-relation neighborhood.

Everything becomes a projection of this one resolver:

| Surface | What it is | Projection of |
|---|---|---|
| `/epr/{cid}` | **lens-complete home** — focal content + all coupled legs | the whole resolver |
| `/lamad/path/{slug}` (pillar mount) | **single-leg deep-dive** (knowledge, rich pillar chrome) | the knowledge leg, projected |
| `/epr/{cid}/raw` | **typed-relation neighborhood inspector** (resource + every anchored relation + provenance) | the graph neighborhood, projected |
| claims-302 | a **context-adaptive lens** (online + claimed) that **never drops the other legs** | the knowledge leg, redirected |

## 3. The Four-Leg Coupling Law (extends the canonical three)

The canonical coupling is **three** legs. This spec adds **process** as a **fourth coupled leg,
enforced symmetrically at compose** (operator decision 2026-06-07):

| Leg | Lens | REA mapping | Pillar (deep-dive) | Enforced at compose? |
|---|---|---|---|---|
| **Knowledge** | story / what it is + connects | Resource + experience | lamad | yes (canonical) |
| **Value** | who stewards it, flows | REA Economic Event / Commitment | shefa / hREA | yes (canonical) |
| **Governance** | reach, ratification, access | Commitment / Agreement | qahal / mishpat | yes (canonical) |
| **Process** | how it is *enacted* / practiced | **REA Process** (input→output events) | (process pillar) | **NEW — yes** |

**Process leg (new):** REA already makes *Process* first-class (a Process transforms Resources via
input/output Events). The fourth leg makes "how this EPR is enacted/practiced/operated" a
non-negotiable coupled relation, not an afterthought view. Enforcing it at compose (symmetry with the
other three) is a **substrate-law extension**: a new typed-relation *class* (Category A2 — a link on
the existing resource entry, **no new DHT entry type**) plus a compose-gate rule in `epr_compose.rs`.

> **Open sub-question for D7/D9 review:** does the process leg ever require a *new entry type*, or is it
> always a typed-relation (A2) over existing `Process`/`Event` entries? Working assumption: A2 only.

## 4. Identity & addressing — CID canonical, slug as edge alias

The `{id}` is the knot. The EPR's true identity is **content-derived (CIDv1)** — and crucially,
`contentFormat` is *part of what the CID hashes*, so **a CID self-certifies what it is**
(epr-composite / html5-app / video / markdown). Today `/epr/{slug}` uses the human slug; the canonical
identity is the CID. This is the anti-pattern *"three address formats left undefined."*

- **Canonical:** `/epr/{cid}` — version-pinned, self-certifying, the share-forever address.
- **Edge alias:** `/epr/{slug}` → resolves (slug→current CID) at the edge. Tracks "latest."
- The raw-node view surfaces the CID prominently so a person can choose **pin-a-version (CID)** vs
  **track-latest (slug)**.

Because `contentFormat` is in the CID, **Model B (render the self-described root) is the
content-addressed reading; the claims-302 (Model A) is a mutable governance overlay** that must
*degrade to* the self-described render offline/unclaimed — never be the floor.

## 5. The resolver (focal content + coupled legs)

`resolve(epr_ref) ->`:
1. **Address**: CID (canonical) or slug→CID (edge alias). Verify bytes ⊢ hash (self-certifying).
2. **Focal render** — dispatch on the resource's own `contentFormat` (the **offline floor**, no
   doorway/claims dependency): `video`→player, `html5-app`→app, `epr-composite`→composite root,
   `markdown`→reader, unknown→raw. *Today the doorway HEAD-fetch returns only `contentType`
   (`doorway http.rs ~1745`); it must also surface `contentFormat`.*
3. **Coupled legs** — walk the bounded typed-relation closure (records-lifecycle vocab; §5.1 closure
   rule: typed-relation set + depth cap + reach boundary) to assemble the value / governance /
   process legs, each rendered as an always-present lens-affordance around the focal content.
4. **Pillar lens (optional, context-adaptive)** — when online **and** a claim is granted **and** the
   richer single-leg experience is wanted, offer/redirect to the pillar mount **without dropping the
   other legs**; degrade to step 2+3 otherwise.

## 6. P2P Design Gate output

Run 2026-06-07. **Verdict: clean — no new DHT entry type, no new identity, no new commitment.**

### Entity: EPR resolution behavior (`/epr/{id}` dispatch)
- **Classification:** Operational (C) — doorway/peer dispatch policy over existing notarized content.
- **Content Address Strategy:** **Content-Derived (CID)** canonical; slug = edge alias resolved at the edge.
- **Source of Truth:** Holochain DHT (`ContentNode` @ `EntryHash=CIDv1`). Router + claim are projections.
- **Anti-pattern caught & corrected:** *three address formats undefined* → CID canonical, slug = display alias.

### Entity: `contentFormat` at resolution time
- **Classification:** Operational (C) — expose an **existing** field (`ContentNode.contentFormat`,
  experience-story-epr §4.3) at `/epr/{id}` resolution. No new entity.

### Entity: raw-node surface (`/epr/{cid}/raw`)
- **Classification:** Operational (C) — read-view of the resource + typed-relation neighborhood +
  provenance. Reconstructable; no `dht_anchor_hash`, no new type.

### Entity: process leg (typed relation)
- **Classification:** Derived (A2) — a typed-relation **class** (link on the existing resource entry),
  carrying a small tag; **no new entry type**. Compose-gate enforcement added in `epr_compose.rs`.
- **Content Address Strategy:** anchored via Holochain Link on the resource `EntryHash`.

### Design constraints discovered
- The resolver floor must be **doorway-free** (D8 "doorway is optional") → focal render dispatches on
  `contentFormat` from locally-held bytes; the legs degrade gracefully when the closure can't be walked
  offline (show what's pinned).
- The 4th leg touches the **coupling-LAW** (compose enforcement), so it is a D1 substrate change, not a
  D8 routing tweak — sequence it behind the resolver.

## 7. Slices (decomposition seeds — sequence)

- **Slice 0 — resolve the click-bug now (bounded, shippable):** add `/epr/{cid}/raw` (neighborhood
  inspector) + surface `contentFormat` at `/epr/{id}` resolution + repoint "View as Content" on claimed
  types at `/raw` (kills the §12.1 round-trip captured in `epr-routing-complementary-captures.md:41`).
  No coupling-law change.
- **Slice 1 — lens-complete `/epr/{cid}`:** focal `contentFormat` dispatch + walk the closure to surface
  the **three** existing legs (knowledge/value/governance) as always-present affordances; claims-302
  demoted to a leg-preserving context lens.
- **Slice 2 — the 4th leg (process):** new typed-relation class + `epr_compose.rs` enforcement
  (symmetry); surface the process lens in the resolver.
- **Slice 3 — CID-canonical addressing:** slug→CID edge alias + version-pinned (CID) vs latest (slug)
  link minting across the EPR-link surface.
- **Slice 4 — offline floor hardening:** focal render + pinned-leg degradation with zero doorway
  (ties to the acquisition dual-pin floor).

## 8. Non-goals / boundaries

- Does **not** change what an EPR *is* (D1 `core-graph-substrate`) — it extends the coupling-law and
  adds the resolver projection.
- Does **not** retire pillar mounts — they remain the rich single-leg deep-dives (projections).
- Does **not** put granular data on the DHT — legs are typed relations (A2) + existing REA events.
- Chain-layer consensus for process enactment is out of scope (D7 Gap-Ledger item).
