---
title: "Seamless groupware — UX addendum: open with, the install ladder, the device view, and the dev-workspace link, bound to the browser chrome, the EPR atom home, and what the protocol must natively support"
id: seamless-groupware-ux-addendum
status: Draft
class: ui-truth-layer
context-tier: disclosed
steward: angular-architect
graduation-trigger: on the household mesh a person opens one object with an epr-app served by another peer from the atom home's "open with" entry, installs it to their device with one click whose receipt shows the four installer acts composed and distinct, sees the runtime facet and the custody facet on the same chip, and reads the object's custody rows + hosting commitments + governance holders in Shefa's device view (one a2o receipt under genesis/a2o/reports/, delta on operator-runtime-surface), AND the operator records acceptance of §7's decision register
actor: agent:orchestrator@fable-5.1
written: 2026-09-08
habits: [operator-runtime-surface, epr-atom-home, dataplane-convergence, runtime-upgrade-propagation]
cites:
  - "package-composition-at-the-holochain-seam | the rules beneath this surface; D6 four acts never equated; A1 classes; A4 install-to-hub is adoption by election | sha256:e417e740ce2212ce | path: genesis/docs/superpowers/specs/2026-09-08-package-composition-at-the-holochain-seam.md"
  - "epr-atom-home-shell-component | §2 the frame stable for every atom; §4 ownership and seams; the shell-owned /epr/{id} this addendum extends with the runtime facet and \"open with\" | sha256:97bcb3c9d81b741a | path: genesis/docs/superpowers/specs/2026-09-02-epr-atom-home-shell-component-design.md"
  - "omnibar-consolidation-epr-native-links-design | §4 EPR-native cross-bundle links and the interceptor; §5 the serving-context segment of the EPR group in the browser chrome this addendum widens into custody + runtime | sha256:3b018cf87bf8a809 | path: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md"
  - "resilience-facings-select-fold-aggregate-design | §4 the five facings: Resiliency per-household, REA per-agent, Operational per-node — the device view is these facings joined per device | sha256:738c9220d105e9e4 | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md"
  - "subject-routing-locus-graph-design | shefa is the native CMS: authoring + value flow + exchange, the CMS owning vs projecting the EPRs — the device view is a CMS view, not a monitoring page | sha256:a884cdf639a04699 | path: genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md"
  - "rea-economic-facing-lens-design | the shefa dashboard's commitment-ledger lens: intent vs observed, mutual-compute — runtime offers and hosting land here | sha256:b83ead21be13bbaa | path: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md"
  - "durability-topology-felt-resilience | asserted → attested → ambient; the felt-resilience gradient the chip renders | sha256:935b1dd7d8121267 | path: genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md"
  - "epr-route-claims-link-conformance-design | §3 the claims contract: how \"open with X\" is minted without a pillar literal; §5 visitor-tiered dispatch | sha256:1d9969399472335d | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md"
  - "epr-resolution-provider-design | the resolver \"open with\" consults for a runtime offer | sha256:bc1f1cbcae739c4a | path: genesis/docs/superpowers/specs/2026-07-02-epr-resolution-provider-design.md"
  - "ai-stewarded-commons-reimplementation-plan | §10.1 the product unit is a capability in a relationship; §10.3 four installer acts; §10.4 Jessica's install experience and the uninstall rule; §10.5 pillar boundaries | sha256:bf2f1a4c94e70670 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md"
  - "compute-envelope-tevah | the envelope an installed executable runs under | sha256:6c9f84bc831ac1bc | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - elohim/lvi/docs/specs/2026-07-20-elohim-native-devspace-design.md
  - elohim/rakia/docs/specs/2026-04-27-rakia-as-brit-attestation-executor-design.md
  - genesis/data/timeline/backlog/object-open-with-runtime-facet-and-install-ladder.md
  - "sprint-velocity-quiescence-holochain-close | T10 peer-executed stage is the first instance of the runtime edge; T13 the cell-qualifier contract this surface binds to | sha256:1e2863ecb8af29e6 | path: genesis/docs/superpowers/plans/2026-09-08-sprint-velocity-quiescence-holochain-close.md"
---

# Seamless groupware — UX addendum

**The bar.** Selecting an object on the p2p dataplane should feel like Google Drive's "open with"
and one-click install, and at times be faster, because the bytes are already near and no central
round trip stands between a person and their document. Underneath, the four installer acts, the
custody and runtime facets, the grant, and the election stay distinct and composed through seams
(surface principle, 2026-09-08). This addendum specifies the surface and names, for each surface
element, what the browser chrome, the EPR atom home, and the protocol must natively provide.

It adds to existing specs; it does not replace them. The atom home keeps its frame; the chrome
keeps its EPR group; the facings keep their selections. What is new is one facet, one ladder, one
view, and one link.

## 1. One facet: the chip reads custody AND runtime

Today the chip in the browser chrome's EPR group renders the custody plane (where the bytes are,
how many custodians, asserted → attested → ambient). Add a second facet on the same chip:

| Facet | Question it answers | Source | Reach/staleness |
|---|---|---|---|
| Custody | Where are the bytes, held by whom, how fresh | Resiliency facing (per-content → per-household) | its own |
| Runtime | Which epr-apps can act on this object, where each runs, who offers it | REA facing (`delegates-compute` and the new *runtime-offer* commitment kind) joined to the object's witness context | its own |

Rendering rules:
- Two glyph slots, one chip. Neither facet is hidden because the other is healthy. A document with
  five custodians and no reachable runtime reads as "held, not openable here"; one whose only
  runtime is another person's hub reads as "openable via <hub>" with that dependency visible.
- The runtime facet is computed from **offers the selector's reach can see** (route-claims §5
  visitor-tiered dispatch). A hosted session sees hosted-reachable runtimes only; it never implies
  a local runtime the device does not have (§10.4).
- Staleness is per facet. A runtime offer past its declared heartbeat greys that facet alone.

**Protocol must natively support:** the object names its witness context (DNA hash + cell id) on
every projected row (D6 contract, sprint T13); a *runtime offer* is a discoverable, reach-scoped
Class-A commitment (compute-grant flow, one more grant kind) resolvable by object CID + epr-app
id + witness context.

## 2. "Open with" on the atom home

The atom home (`/epr/{id}`) frame gains an **Open with** entry in its action row, minted through
the claims contract (route-claims §3), never by a pillar literal. Entries are ordered:

1. Runtimes already enabled on this device (zero-latency, offline-capable).
2. Runtimes offered by peers within the selector's reach, nearest custodian first (LAN before
   relay), each labelled with the offering steward's household name.
3. "More…" collapses everything else, including runtimes the person could install (§3).

Each entry resolves through the EPR resolution provider with a *runtime hint*; the provider
returns the bundle head to open and, for a remote runtime, the peer to open it on. Opening never
copies the object into another app's silo; the app renders the same subject over the permitted
subgraph (§10.5).

**Protocol must natively support:** resolution provider accepts `runtime` as a hint and consults
offers; route claims can declare an *acts-on* set (content types + formats) so "open with" is
computed, not configured per app.

## 3. The install ladder, one click deep

```text
Open with IronCalc (peer: jessica's hub)                    [Open]

Install IronCalc
  ● On this device — works offline, keeps a copy of the app     [Install]
  ○ On my hub — always on, can offer it to others               [Install to hub]
  Learn what this changes ▸
```

Surface: two radio rows and one button. The system composes beneath each button:

| Click | Acts composed (never equated) | Receipt shows |
|---|---|---|
| Install on this device | obtain the bundle head · enable declared runtime capabilities · replicate the executable's store if the device meets the budget · run under the ark envelope | four lines, one per act, each with its own status; a refused act names why (budget, reach, authority) |
| Install to my hub | the same on the hub · plus a separate **offer runtime to peers** commitment, defaulting to *not offered* until the steward flips it | the four lines plus one commitment line, with the reach the offer covers |

"Learn what this changes ▸" expands to the four acts in household words (§10.4 copy discipline):
"Keep a copy of this app here", "Let it read your shared ingredient list", "Join the meal
board's community" (only if not already joined), "Nothing else changes". A capability already
granted is shown, never re-asked.

Rules:
- The device branch is offered only when the chosen budget (sprint T12) says the device can hold
  the executable's store; otherwise the row reads "This device can't host it; your hub can" or
  "a nearby community hub can", with the nearest offering steward named.
- Installing never joins a community or provisions a witness context as a side effect (seam spec
  D6). If the object's context is one the person is not in, the ladder shows the join act as its
  own line with its own consent.
- Uninstall removes the interface and its active capabilities only; commitments, others'
  evidence, and promised custody stay (§10.4). If the bundle also supplied a still-needed
  background service, the uninstall dialog shows that dependency and offers placement or
  replacement, not silent removal.

**Protocol must natively support:** install-to-hub is the hub adopting the bundle head by
election (`services/release_adoption/`, seam spec A4) with "runs the target bytes" counting as
adopted (sprint T4); the resolved composition is recorded and recoverable (seam spec A8); the
offer is a compute-grant commitment with reach.

## 4. Shefa's device view: what is stored and hosted, with whom

Shefa is the protocol's native CMS (subject-routing locus spec: authoring + value flow + exchange,
the CMS that owns vs projects the EPRs), and its dashboard already carries the commitment-ledger
lens (intent vs observed, mutual-compute). The device view is therefore a CMS view over the
person's own compute surface, not a monitoring page: every row is something they author, steward,
host, or accept, and every row is actionable in place. It is built by joining three existing
facings per device:

| Column | Facing | Row example |
|---|---|---|
| Stored here | Resiliency (holder relation) | "Supper recipe — copy on this phone, on Matthew's hub, on Jessica's hub" |
| Hosted / computed here | REA (`delegates-compute` + runtime offers) | "IronCalc runtime — offered to household, 3 opens this week" |
| Governed by | governance leg (reach + grant holder) | "Meal board — Jessica stewards; you can read and propose" |

The view answers "what does my social compute surface look like", device by device, and is the
place the elohim reasons from when it suggests moving a hosting commitment to a hub, retiring an
unused runtime, or accepting a neighbour's offer. Every row links to the atom home of the thing
it names.

**Protocol must natively support:** the REA facing exposes runtime offers and their consumption
counts per device; the composition record (A8) is readable per device; hosting shows in the
reciprocity fold so it counts as contribution.

## 5. The dev-workspace link: every app is one click from contributing

Every installed epr-app carries, in its "More…" menu and in its atom home, **Open a dev
workspace**. It resolves the app's source and build artifacts (rakia: content-addressed, attested
executables) into an lvi devspace on whichever capable runtime the person's reach can see: their
own device if the budget allows, their hub, or a nearby community hub offering compute. The
person lands in a workspace already holding the app's source at the running version, its
resolved composition, and a build grant; their elohim is a native participant (it can read the
same subgraph, propose a change, run the gate as a compute task on a peer, and attach the attested
result to a contribution).

This is the same install ladder with a fifth act, **contribute**, that composes: obtain the source
head · enable build capabilities · accept a compute grant from a provider · run under the envelope
· attach attested results. It is the social coding commons: the community that uses a tool is one
click from improving it, on hardware the community already stewards, with the elohim helping
drive the improvement as a participant in the substrate rather than a tool outside it.

**Protocol must natively support:** rakia artifacts resolvable by the same resolver (source head
→ build → executable); lvi accepts a composition record as its workspace seed; delegated compute
runs the gate as a stage with an attested result (sprint T10 is the first instance).

## 6. Performance felt at the surface

| Moment | Target feel | What makes it true |
|---|---|---|
| Select an object | chip facets render from the local projection, instantly | zero round trips; facets are folds over local relations (facings §5 determinism) |
| Open with a local runtime | as fast as a native app | the executable is local, the object is local |
| Open with a peer runtime on the LAN | faster than a hosted drive | blob from the nearest custodian over iroh, no central RTT (seam spec P2) |
| Install on this device | progress shown per act; usable before witnessing completes | witnessing is async; the projection is the working copy (seam spec D7) |
| A change by someone else | appears on notification, converges anyway | notification carries the act; sweep converges without it (seam spec D8, sprint T6) |

Budgets are numbers only once T12 chooses them; until then the targets are directions, not
claims.

## 7. Decision register (operator to accept or amend)

| # | Decision | Default |
|---|---|---|
| U1 | The runtime facet lives on the existing chip, not a second chip | accept |
| U2 | "Open with" entries are computed from route-claim acts-on sets plus offers, never configured per app | accept |
| U3 | Install-to-hub defaults to *not offered* until the steward flips the offer | accept |
| U4 | The device branch is hidden, not disabled, when the budget refuses; the hub or a nearby hub is named instead | accept |
| U5 | "Open a dev workspace" ships on every app's More… menu from the first slice, even before lvi can host it locally (it can name a community hub) | accept |

## 8. Slices and a2o homes

1. **Facet** (after T13): chip renders custody + runtime from local folds; a2o under
   `genesis/a2o/features/elohim/` with `@concern:operator-runtime-surface`.
2. **Open with** (after T10): atom home action row; one object opened via a peer's runtime on
   the household mesh, jessica offering, matthew selecting.
3. **Install on this device**: four-act receipt; ark envelope; offline open.
4. **Device view**: Shefa join of three facings per device.
5. **Install to hub + offer**: adoption by election; offer commitment; hub receipt (needs the Adam
   worker or a household hub).
6. **Dev-workspace link**: resolves into lvi on a capable runtime; first contribution with an
   attested gate result.

Each slice cites the seam spec rule it exercises and the receipt it produced. None is in sprint
2026-09-08; slice 1 follows T13, slice 2 follows T10.
