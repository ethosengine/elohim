---
title: EPR Route Claims, Redirect Governance & Link-Integrity Conformance — Design
id: epr-route-claims-link-conformance-design
status: Draft
class: protocol-canonical
domain: D8
topic: [epr, routing, routeClaims, redirect, alias, link-integrity, conformance, sitemap, doorway, dispatch, reach, attention-tending, discovery]
refines: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md
  - genesis/docs/content/elohim-protocol/architecture/2026-06-02-doorway-ssr-runtime.md
cites:
  - pillar-epr-decomposition-design | THE parent: §12 URL & Routing Contract whose §12.3/§12.8 routeClaims sketch this spec elevates; refines edge | sha256:8029079cea758380 | path: genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md
  - epr-slice2-universal-address-plan | landed Slice-2 plan: locked decisions (universal address, bridge rationale, /auth vocabulary) this design builds on | sha256:78644191dd11bf3d | path: genesis/docs/superpowers/plans/2026-06-06-epr-slice2-universal-address-plan.md
  - omnibar-consolidation-epr-native-links-design | locked cross-bundle link mechanics (plain href + interceptor, ServingContext) this spec must not contradict | sha256:92df16eea8d9bcf8 | path: genesis/docs/superpowers/specs/2026-06-05-omnibar-consolidation-epr-native-links-design.md
  - semantic-computable-links-design | docs-corpus content-addressed link integrity (slug/fingerprint/status) — the model extended here to runtime links (claims-stale isomorphism) | sha256:1460bc102580ab0d | path: genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md
  - trust-compute-gradient-brainstorm | 2026-04-30-trust-compute-gradient-brainstorm | sha256:89c493c73ff6b06b | path: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
  - doorway-stewardship-chain-design | grant/snapshot precedent: Commitment+Attestation chain, JWT fast-path, supersession — the visitor-reach and grant mechanics template | sha256:f90729e7a9887de8 | path: genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - records-lifecycle-design | link lifecycle precedent: intentionally-degraded vs maintained, closure rejection, redaction markers — the alias retirement lens | sha256:2b5f54d20108bcf0 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - rea-compute-commitment-primitive | rea-compute-commitment-primitive | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md
  - stewardship-over-sovereignty | stewardship-over-sovereignty | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - genesis/docs/architecture/pillar-bundle-split-runbook.md
---

# EPR Route Claims, Redirect Governance & Link-Integrity Conformance

**This spec elevates §12.3/§12.8 of the pillar-EPR decomposition design from a one-paragraph
sketch into the canonical Slice-3 routing contract.** It answers three entangled questions the
2026-06-06 handoff posed: (1) route-level redirect/aliasing as governed bridge-work, (2)
routeClaims as a steward-governed routing substrate, (3) link-integrity conformance — "the
`<a>` tag of the elohim protocol, applied everywhere." It composes from the parent spec's §12
URL & Routing Contract (Slices 1–2 landed 2026-06-04/06), the omnibar spec's locked cross-bundle
mechanics, the semantic-links system (the docs corpus's content-addressed link integrity,
extended here to the running web), and the trust-compute-gradient brainstorm (the inverted
compute gradient + AttentionTending).

**What this spec does NOT redraw:** the §12.1 three-surface URL model, the §12.2 ROUTE/ASSET
rule, the locked omnibar decisions (plain href + capture-phase interceptor cross-bundle;
routerLink same-bundle only), the `EprRouteResolution` client contract, or the §12.6 slice
boundaries. Slice 3 is the open slice; this is its design.

---

## §1 The two-plane model — one claims vocabulary, three consumers

Routing exists on two planes that Slice 2 left as disjoint layers:

| Plane | Resolver | Surface | Who sees it |
|---|---|---|---|
| **Static** (doorway, request-time) | `is_service_path` → `EprRouter` longest-prefix → projection dispatch → ROUTE/ASSET → `/epr/{id}` resolver | The *served* URL surface — cold loads, shares, crawlers, curl, SEO | The web |
| **EPR-SPA** (client, post-boot) | Angular router + `BUNDLE_ROUTE_CONTEXT` claims + `eprToRoute` minting + epr-link interceptor | The *minted* URL surface — in-app navigation | The person inside a booted bundle |

The contract that binds them: **a URL minted on the SPA plane MUST dispatch identically on the
static plane when cold-loaded.** One claims vocabulary feeds three consumers:

1. **Doorway dispatch** (static plane) — the granted-claims table drives `/epr/{id}`
   302-to-pretty-mount and alias redirects.
2. **Client minting** (SPA plane) — bundles derive `BUNDLE_ROUTE_CONTEXT` from the same
   declaration the grant acknowledges.
3. **The enumerable route-manifest** — the doorway-generated `sitemap.xml` and the conformance
   crawler's expected-set are *derived projections of the routing table* (Operational C,
   reconstructable by construction).

```
            BUNDLE MANIFEST (build-time, CID-addressed)
              routeClaims: DECLARED        ← the request; static lint checks against this
                      │
               grant ceremony (steward, write-time)
               conflict resolution happens HERE
                      ▼
      PROJECT-EPR COMMITMENT (notarized, Category A)
        routeClaims: GRANTED + claimsManifestCid
        redirectsFrom + redirectTemplates (alias law)
        spa_fallback · reach · gateHints · previewEprRef · deadEnd
                      │
        boot fetch + SSE + periodic replace_all   (existing flow — unchanged)
                      ▼
   ┌──────────────────┴───────────────────┐
   │ STATIC PLANE (doorway dispatch)      │ SPA PLANE (client minting)
   └──────── two-layer drift fixtures ────┘
                      │
                      ▼
      DERIVED: sitemap.xml · crawler expected-set
```

## §2 The inverted-gradient invariants (normative)

The reach epic inverts the compute gradient: cost is paid at **authorship and grant time**, not
read time (trust-compute-gradient brainstorm §2.1 — "distribution cost scales with trust at
every hop").

- **R1 — The Commons Fast Path (MUST).** For `reach=commons`, dispatch performs no per-request
  governance computation beyond the routing-table lookup: no DHT reads, no commitment walks, no
  auth roundtrips, cache-friendly responses. Latency parity with static HTTP serving is a
  *measurable conformance property* (§7), not an aspiration. All claim validation, conflict
  resolution, alias checking, and manifest fingerprinting happen at declare/grant/refresh time.
- **R2 — Gradient placement (MUST).** Gate/boundary compute exists only on non-commons arms.
  Conformance/audit compute (lint, crawler, sweeps) runs entirely off the request path.
- **R3 — The AttentionTending seam (MUST preserve).** The only read-time mediation is
  consumption-side and negotiated: the person's tending agent (trust-compute-gradient §6.1 —
  "not a perimeter filter; a discernment signal"), the protocol-native answer to ad blockers /
  CovenantEyes-class filtering. The doorway never filters commons content. The routing layer's
  duty is to feed the tender: links and boundaries carry the **epr-summary-hint** envelope
  (§5.3) so a tending agent can decide follow / skip / wrap *before* fetching. The
  tending-immune floor (constitutional, mishpat-notarized) is inherited: a tending filter may
  wrap, but the resolver guarantees the address still resolves.

## §3 The claims contract — declare + grant

A routeClaim's force at a doorway is **two-level**: the bundle *declares*, the projecting
steward *grants*.

### §3.1 Declared claims (bundle manifest — build-time, CID-addressed)

```jsonc
"routeClaims": {
  "schemaVersion": 1,
  "claims": [
    { "contentType": "path",
      "template": "path/{id}",
      "fragments": { "step": "path/{id}/step/{n}" } }
  ]
}
```

Declarations ride the bundle EPR's manifest content (§12.3 "like §3 element manifests") and
version with the bundle's blob hash — a redeploy IS the declaration-version bump. The
declaration is (a) the **request** input to the grant ceremony and (b) the **build-time truth**
the static lint (§7.1) and the bundle's own `BUNDLE_ROUTE_CONTEXT` derivation (§8.3) check
against. Per the gate record (§12): claims need no standalone notarization because they cannot
be silently changed — editing claims changes the bundle CID, which is a visible `blob_hash`
change on the Category-A commitment.

### §3.2 Granted claims (commitment metadata — steward-authored, write-time)

```jsonc
"routeClaims": {
  "schemaVersion": 1,
  "claimsManifestCid": "bafkrei…",   // fingerprint of the manifest version acknowledged
  "claims": [ /* the granted subset, same shape as declared */ ]
}
```

The grant rides the existing `project-epr` commitment metadata (like `spa_fallback` and
`redirects_from`) and flows through the unchanged `rea_commitments` → `EprProjectionView` →
boot-fetch + SSE + `replace_all` path. **Dispatch reads only the table** (R1). The granted set
is the operative routing law: steward-authored, stable across bundle redeploys until
re-granted, revocable by commitment supersession (the existing predecessor-link pattern; no new
recovery story needed — stewardship-over-sovereignty §6 Rule 1 is satisfied by commitment
update/supersession).

### §3.3 Conflict resolution is grant-time uniqueness

The grant validator REJECTS (or supersedes, operator-curated) a grant whose
`(doorway, contentType)` binding is already held by another projection on that doorway.
Dispatch inherits a table that *cannot* have runtime conflicts — no nondeterministic
tie-breaking (today's `max_by_key` over HashMap values has no stable winner for equal-length
prefixes; claims never inherit this), no first-wins races. Vocabulary discipline: a grant binds
a *steward's projection*; nothing is "owned" — routes are stewarded under the nearest-authority
frame, and the binding is walkable to its constituting commitment.

### §3.4 Drift is governed, cite-style

Bundle redeploy → new manifest CID ≠ `claimsManifestCid` → the conformance sweep (§7.5) stamps
**`claims-stale`** — the STALE-CANDIDATE mechanism from the semantic-links system applied to
runtime routing (grant = envelope, `claimsManifestCid` = fingerprint, sweep stamps status).
Granted claims keep serving (stable routing law) until the steward re-grants — drift is
visible, never silent, and never breaks the serving path.

## §4 Alias law — redirect governance on the commitment

Two alias mechanisms, both on the notarized commitment, both consumed by doorway dispatch:

1. **`redirectsFrom: Vec<String>`** (mount-level, bare paths) — already notarized, currently
   dead-wired (`projection.rs:45-46`; nothing consumes it). This spec wires it: `EprRouter`
   gains an alias→canonical index; dispatch issues 301/302 to the same sub-path under the
   current `url_path`. The §12.4 sanction ("`redirects_from` on the commitment is the
   sanctioned migration tool") becomes real behavior.
2. **`redirectTemplates: [{from, to}]`** (route-level, NEW) — e.g.
   `{ "from": "/lamad/resource/{id}", "to": "/epr/{id}" }`. The promise "this old address
   stays alive" becomes notarized, steward-granted, revocable-by-supersession, and auditable.
   Who committed: the projecting steward. For whom: holders of monolith-era shares. At what
   cost: one dispatch-table row. Revocation: a visible commitment update.

**Validation (grant-time, closing the gap the p2p gate caught):** `validate_project_epr_commitment`
extends §12.1's rules to aliases — (a) no alias may collide with a reserved prefix
(`/epr|/db|/api|/blob|/apps|/auth|/status|/health`); (b) no alias may shadow another
projection's live mount on the same doorway; (c) alias chains are forbidden — an alias targets
a canonical address (`/epr/{id}` or a current mount), exactly one hop; (d) templates use the
same placeholder grammar as claims (§8.1).

**The bridge story completes (Q1 adjudicated):** the Slice-2 `LegacyResourceRedirectComponent`
was correct bridge-work without a story. With `redirectTemplates`, a cold
`GET /lamad/resource/{id}` matches at the doorway and 302s to `/epr/{id}` *before any bundle
boots*. The component remains as the SPA-plane twin (in-bundle hits on the legacy route) until
legacy in-app hits age out, then retires; the a2o `@regression` anchor flips from "bridge
component traverses" to "doorway honors the notarized promise" — same scenario intent, upgraded
mechanism. Lifecycle frame: a *named transitional bridge with a retirement phase* (dissolution
taxonomy: Interface → Subsumption → Absorption → Retirement); records-lifecycle's
"intentionally degraded vs maintained" lens applies — the old address is degraded access to a
still-verifiable target, not lost truth.

## §5 Dispatch semantics — the visitor-tiered resolver

### §5.1 The classifier

`classify_epr_universal(id, visitor)` is a **pure classifier** returning a Disposition
(mirroring `classify_dispatch` — unit-testable without HTTP, table-data only; path-prefix
guards in dispatch arms remain forbidden per server/CLAUDE.md):

| Target state | Anon visitor | Authed visitor (DHT identity) |
|---|---|---|
| commons + claimed | **302 → pretty mount** (cacheable) | same |
| commons + unclaimed | serve shell → universal viewer | same |
| gated + claimed + visitor reach passes | — (anon never passes) | **302 → pretty mount** |
| gated + unclaimed + visitor reach passes | — (anon never passes) | serve shell → universal viewer |
| gated + reach fails | **head-edge boundary** (§6) | same, plus visitor's progress toward the gate |
| no projection / unknown id | generic designed boundary (no head disclosed) | same |
| alias template match | one-hop 302 to canonical target first | same |

302-to-pretty-mount happens **only when reach passes** — the resolver owns the gate; mounts
never see unauthorized traffic; the gate experience is uniform protocol-wide. The anon tier's
reach-passes set is the substrate's anon-readable rule **{commons, public}** (mirrors storage's
unauthenticated list filter; reach hierarchy public=6 < commons=7) — one definition per side,
`anon_reach_readable` doorway-side. (Found live 2026-06-06: commons-only pinning denied public
content its pretty mount; the wider 3-vocabulary reach reconciliation remains tracked debt.) The `/epr/{id}`
resolution requires the target's contentType + reach: a **local projection head lookup only**
(storage `/epr-head/{id}` family — never a DHT walk on the dispatch path), cacheable for
commons.

Visitor reach resolution is **snapshot-based** (the stewardship-chain JWT/session fast-path
precedent): anon = commons-only with zero identity compute; authed = reach from the session
snapshot, never a per-request chain walk. Today's hardcoded `401` (reach≠commons) and `501`
(steward-direct) in `dispatch_to_projected_epr` are replaced by the gate-face arm; steward-direct
mode remains out of scope (501) for this spec.

### §5.2 Fragment passthrough — one place per side, resolved

URL fragments never reach the doorway (HTTP), and RFC 7231 semantics make browsers re-attach
the original fragment when a 302 `Location` carries none. Therefore:

- **Doorway**: path-level 302s only; needs no fragment templates at all.
- **Landed bundle**: upgrades `#step/2` to its pretty step route using the *same claim's*
  `fragments` template after boot.
- **Minting client**: uses `fragments` templates via `eprToRoute` exactly as today.

"Fragment mapping lives in exactly one place per side" (§12.3) resolves to: doorway = none
needed; bundle = fragment-upgrade; minter = `eprToRoute`.

### §5.3 The epr-summary-hint (named wire surface)

The ~500-byte head-edge envelope — id, contentType, reach, title, preview ref, gate-relation
summary — served on the existing `/epr-head/{id}` family. One envelope, three readers, all
pre-fetch:

1. the anon/gated **boundary face** (§6),
2. **link affordance disclosure** — `<elohim-epr-link>` reveals the edge before the click (a
   link's visual grammar tells you commons vs gated),
3. **AttentionTending agents** (R3) — the tender decides follow / skip / wrap from the hint
   without fetching, the same epr-head-edge mechanism the cite system's `status:` field
   dogfoods.

Commons-visible by construction (§6.1); cacheable.

## §6 The default experience — head-edges and the inclusive path

### §6.1 Head-edge disclosure is governed at grant time

Anon traffic explores the public commons at full speed and sees **EPR-head edges** of the
gated graph — and *only* what stewards committed as the outward face. Parent §2.4 validation
Rule 1 already guarantees this: a non-commons projection **cannot exist** without
`previewEprRef` (commons-reach by design) OR non-empty `gateHints` OR an explicit
`deadEnd=true`. The protocol cannot have an undesigned wall — *by validation, not by UX
diligence*. Undisclosed existence is expressed by having no projection at all (the resolver
renders the generic boundary with no head).

### §6.2 The discovery-RPG is a conformance class

Every gated boundary renders the head-edge plus the **inclusive path**: the ordered
`gateHints` graph (PersonWhoCanGrant · MembershipPrerequisite · ContentToSync · PlaceToVisit ·
CapabilityToEarn · PaymentToOffer · WitnessToInvolve — each hint itself an EPR, recursively
explorable). For an authed visitor, the face additionally reflects *their* progress toward the
gate (which hints they already satisfy). This is the default daily-drive experience: the
boundary is the invitation — a quest surface showing the connections and steps that would earn
reach to well-reasonedly gated content. A boundary without a face is a validation failure
upstream and a rendering failure downstream (§7.4).

**Rendering homes:** the shell's universal viewer renders the gate face at `/epr/{id}`;
`LamadNotFoundComponent` adopts the same §6 outward-face pattern for the bundle's `**` route
(absorbing the backlog capture explicitly gated on Slice 3).

## §7 Link-integrity conformance — the `<a>` tag of the protocol

Conformance classes for a minted link, measured honestly (counts and timestamps, never
absolute claims — the epistemic-integrity lesson: report "N of M anchors verified at T," never
assert an unprovable "zero dead links"):

### §7.1 Author time (MUST)
Links are **minted, never literal** — `eprToRoute` / `eprToUniversalHref` / claims templates
are the only legal sources. A CI lint generalizes the pillar-bundle-split runbook §4.4
router-literal canary (the Slice-2 hand-run greps — `'/resource'`, `'/content'`, pillar
prefixes — become a permanent gate). Documented keepers: `<base href>`, SEO canonical
generation, doc comments describing the public URL surface.

### §7.2 Build time (SHOULD)
Code/fixture-referenced EPR ids verified against seed/content at build — cite-gen's
fingerprint discipline applied to code-minted refs.

### §7.3 Serve time (MUST)
Every well-formed link resolves to a rendered surface or a designed boundary. Commons resolves
on the fast path (R1 — latency-parity assertion is part of the suite). Asset misses stay
honest 404s (§12.2 discipline preserved).

### §7.4 Boundary time (MUST)
Every gated boundary renders the head-edge + inclusive path (§6.2).

### §7.5 Continuous
- **`sitemap.xml`** — doorway-generated from projections × granted claims × commons-reach
  enumeration (Operational C, derived-by-construction; the static plane becomes enumerable).
  **Materialization model**: an event-materialized projection, never request-time computation —
  every sitemap change source is a notarized write event (grant, commons content registration,
  alias change) already flowing gossip → post-commit signal → SSE → doorway refresh; sitemap
  regeneration rides those same events (debounced alongside `replace_all`) and `GET /sitemap.xml`
  is a static cached read with `ETag`/`<lastmod>` from commitment/content timestamps (R1 applied
  to crawlers; eventual consistency at gossip latency is over-fresh for sitemap consumers).
  Derived *from* DHT entries, never notarized *as* one — reconstructable by construction, and
  the swap test holds: any doorway projecting the same commitments materializes the identical
  sitemap.
- **The conformance crawler** (a2o) — walks every rendered anchor from the seeded surface set
  against the sitemap expected-set; asserts rendered-surface-or-designed-boundary,
  render-verified (never HTML-shell).
- **The sweep** — stamps `claims-stale` (§3.4) and `DEAD-ALIAS` (alias targeting a retired
  mount/address) statuses, cite-style: tool-managed, reviewable, never hand-written.
- **Two-layer fixtures** (§8.4) pin static-plane/SPA-plane agreement continuously.

## §8 Schema & code homes

### §8.1 Claims schema
`elohim/sdk/schemas/v1/route-claims.schema.json` — declared-claims and granted-claims shapes,
the placeholder grammar (`{id}`, `{n}` — segment-safe substitution only), `schemaVersion`
discipline. TS codegen via the existing `schema:codegen:ts` pipeline (added to
`INTERFACE_FILES`).

### §8.2 EprProjectionView extension
`routeClaims` (granted form) + `redirectTemplates` fields follow the standard view-schema
contract: views schema → Rust struct (`elohim/elohim-views/src/projection.rs`,
`#[serde(rename_all = "camelCase")]`, `#[derive(TS)]`) → schema contract test → codegen →
`cargo test export_bindings`. snake_case never leaves the Rust boundary.

### §8.3 Client derivation
Bundles derive `BUNDLE_ROUTE_CONTEXT` from their **own manifest declaration** (build-time
constant generated from the manifest — single authoring home), replacing hand-wired
composition-root `useValue` providers. The doorway consumes the *granted* set; the client
mints from the *declared* set; §3.4's `claims-stale` is the governed gap between them.

### §8.4 Two-layer drift fixtures
`elohim/sdk/fixtures/route-claims.vectors.json` — (claims table × EPR refs → expected
commands / href / 302 Location) consumed by doorway Rust dispatch tests AND client TS
`eprToRoute` tests, generalizing `spa-route-discrimination.vectors.json` ("edit here, never
fork per-crate copies").

### §8.5 Dispatch implementation
`EprRouter` gains the granted-claims index (contentType → mount binding) and the alias index
(exact + template) populated in `replace_all` — compiled at table-load, never evaluated against
the DHT per-request. The `/epr/{id}` arm calls the §5.1 pure classifier.

## §9 Error handling & edge cases

| Case | Behavior |
|---|---|
| Grant references unknown contentType | Grant-time validation against the lamad-manifest vocabulary; reject with reason |
| Grant without matching manifest declaration | Permitted but stamped `claims-unverified` by the sweep (steward may pre-grant; the lint catches the bundle side) |
| Alias chain (alias → alias) | Rejected at grant validation (one hop, canonical targets only) |
| Alias shadows reserved prefix or live mount | Rejected at grant validation (§4) |
| Bundle redeploy changes declared claims | Granted claims keep serving; `claims-stale` stamped; re-grant ceremony clears it |
| Claimed mount's projection revoked | SSE `projection.revoked` → `replace_all` drops claims + aliases atomically; targets degrade to `/epr/{id}` universal floor (safe-by-default preserved) |
| `/epr/{unknown-id}` | Generic designed boundary; no head disclosed; never raw 404 JSON on a ROUTE |
| Head lookup unavailable (storage down) | Degrade to serve-shell (Slice-2 semantics); never block dispatch on the head — fail open like the interceptor |
| Template grammar abuse (e.g. `{id}` matching across segments) | Placeholder substitution is segment-bounded by grammar (§8.1); vectors pin it |

## §10 Testing & a2o scenarios

- **Unit**: classifier matrix (target state × visitor tier × claimed/aliased) — pure function,
  no HTTP; grant-validator cases (conflicts, reserved prefixes, chains).
- **Contract**: route-claims vectors consumed by both planes (§8.4); schema contract test for
  the view extension; codegen-freshness via the existing pre-push gate.
- **a2o** (`genesis/a2o/features/lamad/deep-link-delivery.feature` extends; doorway subject for
  sitemap/conformance):
  1. Cold `/epr/{id}` for a claimed commons EPR 302s to the pretty mount and renders.
  2. Cold legacy share `GET /lamad/resource/{id}` 302s at the doorway (no bundle boot) —
     the upgraded `@regression` bridge anchor.
  3. Fragment survival: `/epr/{pathId}#step/2` → 302 → mount → bundle upgrades to the step.
  4. Gated EPR for anon renders head-edge + inclusive path (never a wall, never a 302).
  5. Authed visitor with sufficient reach gets the mount; without it, the face shows progress.
  6. `sitemap.xml` enumerates exactly the granted claims × commons surface.
  7. Commons fast-path latency parity smoke (R1 measurable).
- **Conformance crawler**: a2o suite walking rendered anchors against the sitemap expected-set
  (implementation may trail as its own plan task; the suite design is normative here).

## §11 Slicing & migration

- **Slice 3a — claims grant + dispatch**: schema + view extension + grant validation + 
  classifier + 302-to-pretty-mount + client derivation + vectors. (The §12.6 slice table's
  "routeClaims in manifests" row, realized as declare+grant.)
- **Slice 3b — alias law**: `redirectsFrom` consumption + `redirectTemplates` + bridge
  retirement + scenario flip.
- **Slice 3c — conformance**: CI lint gate + sitemap + crawler + sweep statuses.
- **Parent-spec amendments** (small, forward-pointing): §12.3 routeClaims paragraph and §12.8
  classification note point here; §12.6 slice table marks Slice 3 as designed-by this spec.
- **Explicitly after claims** (unchanged ordering from the handoff): card-flip + pushState
  mechanics (§7 + §12.3 last bullet) — flips push "mount URL if claimed, else `/epr/{id}`".
- **§12.7 vision tier stays designed-for** (federated anycast, toll access via
  `delegates-compute`, capability load balancing): BLOCKED-BY-ENV (alpha degraded, shem off);
  only the resolution policy inside `/epr/{id}` evolves, never URL shapes — nothing here
  forecloses it.

## §12 P2P design gate record

| Entity | Class | Identity | Source of truth | Notes |
|---|---|---|---|---|
| RouteClaim (declared) | C (operational manifest content) | Content-derived (bundle CID) | Bundle EPR manifest (notarized content row) | §12.8 classification upheld; authority inherited from the commitment that projects the bundle |
| RouteClaim (granted) | rides Category-A commitment metadata (like `spa_fallback`) | n/a (attribute of the commitment) | DHT (project-epr commitment) | Steward-authored; revocation = supersession |
| `redirectsFrom` consumption | A (existing field) | n/a | DHT | Wiring, not new entity; validator gap closed (§4) |
| `redirectTemplates` | A (new commitment metadata field) | n/a | DHT | The bridge's substrate story |
| epr-summary-hint | C (projection of head data) | EPR id | derived | Existing `/epr-head/` family; cacheable |
| sitemap / route-manifest | C (derived-by-construction) | n/a | none (projection of the routing table) | Reconstructable at any moment |
| link-audit record | CI artifact (C); Attestation form is a captured follow-up | — | — | Needs the crawler first; attest only what's measured |

No new DHT entry types. No new identity schemes. No new link types. Anti-pattern checks: no
UUID identities; no REST-first design (claims flow DHT-commitment → projection → dispatch); no
prefix guards in dispatch arms; no granular data on the DHT.

## §13 Captured follow-ups (not this spec)

- **Conformance crawler + sweep statuses** (Slice 3c remainder, gap item `#7-5`): the a2o
  crawler walking rendered anchors against the sitemap expected-set, and the sweep stamping
  `claims-stale` (§3.4) / `DEAD-ALIAS` statuses. Backlog:
  `genesis/data/timeline/backlog/epr-routing-complementary-captures.md`.
- **Substrate-visible link-audit Attestation** (`kind=link-audit` on the existing Attestation
  entry type, attached to the projection commitment) — after the crawler exists.
- **ServingContext provenance headers** (omnibar §9.7) — `X-Bundle-Address`/`X-Variant`;
  re-runs the gate when picked up.
- **X-Cache passthrough** at the doorway proxy (observability for link-serving provenance).
- **Alpha ingress `/lamad/path` SSR-intent rules** reconcile (§12.2 seam).
- **Slice-2 review-debt tail**: step-def consolidation, `EprNavService` shared-lib collapse,
  `updateForProfile` canonical oddity, shared `eprUniversalHref` template helper (encoding
  consistency).

## Appendix A — Decision log (operator-adjudicated, 2026-06-06)

1. **Claim force**: declare + grant (bundle declares in manifest; steward's commitment grants;
   conflicts resolve at grant time).
2. **Bridge story**: alias templates on the commitment — the promise is notarized, revocable,
   auditable; the component retires by substrate.
3. **Reach × claims**: the resolver owns the gate — 302 only when reach passes; gated visitors
   get the §6 face at `/epr/{id}`.
4. **Conformance scope**: static gate + contract fixtures + crawler/sitemap in this spec;
   substrate-visible attestation captured as follow-up.
5. **Grant mechanics**: grant-on-commitment, denormalized, with `claimsManifestCid` fingerprint
   (drift governed cite-style).
6. **Paradigm constraints** (operator): the inverted compute gradient is normative (R1–R3);
   commons loads at HTTP speed with AttentionTending as the only — consumption-side,
   negotiated — read-time mediation; anon explores commons + head-edges; the gated boundary is
   the discovery-RPG inclusive path, the protocol's default daily-drive experience.

## Appendix B — HTTP ↔ EPR translation (informative, for web developers)

How the status codes and artifacts you already know translate when routing is a projection of
notarized substrate rather than server state. The deepest shift: **a response is never "what
this server decided" — it is what *any* doorway projecting the same commitments would resolve**
(the swap test). Nothing in this appendix adds normative requirements; sections cited are the
normative homes.

| The web you know | The EPR world | Where |
|---|---|---|
| **200 OK** — the server has the file | A **projection served you**: a notarized `project-epr` commitment authorized this mount; provenance headers (`X-Content-Address`, `X-Reach`) say which substrate facts backed the response. Commons content rides the fast path — table lookup, no governance compute (R1). | §2, §5.1 |
| **301/302** — the admin moved a file | A **notarized routing decision**: a granted claim 302s `/epr/{id}` to its pretty mount, or a steward's alias promise (`redirectsFrom` / `redirectTemplates`) keeps an old address alive. Every redirect is walkable to the commitment that authorized it — revocable by supersession, never silent config. Browsers carry your `#fragment` across (RFC 7231); the landed bundle upgrades it. | §4, §5.1–5.2 |
| **404 Not Found** — dead end, your problem | **Narrowed to assets only.** A missing hashed bundle file is a deploy bug and MUST surface as an honest 404 (§12.2 discipline). A *content route* never raw-404s: an unknown `/epr/{id}` renders a designed boundary. 404-as-user-experience is a conformance failure. | §7.3, §9 |
| **401/403** — a wall | **Never a wall for content.** A gated target renders the head-edge boundary: the steward-authored outward face (preview + `gateHints` — the inclusive path to earning reach). The face is itself commons content, served as a real page; machine clients read the state from the epr-summary-hint envelope, not the status code. Parent §2.4 Rule 1 means a gated projection *cannot exist* without this face. | §5.1, §6 |
| **410 Gone** — deleted, tough luck | **Intentionally degraded, never lost truth**: a closed EPR's address resolves to a designed terminus; redaction leaves a `redaction-applied` provenance marker; validation rejects *minting* new links to closed EPRs (the failure is prevented at write time, not discovered at read time). | records-lifecycle; §7.2 |
| **304 / ETag / Cache-Control** — hope the cache is right | **Content addressing makes caching exact**: CID-addressed content is immutable — cache it forever; a new version is a new address, not an invalidation. The content address IS the strongest possible ETag. | §3.1, R1 |
| **429 / paywalls** — rate-limit the stranger | The **trust-compute gradient**: cost scales with distance from commons, paid at authorship/grant time — never imposed on commons reads. (Toll access for un-contracted content is the §12.7 vision tier, via `delegates-compute` commitments.) | §2, §11 |
| **5xx** — the server is sad | Operational failure stays honest and **fails open toward the floor**: head-lookup down → degrade to serve-shell (Slice-2 semantics); interceptor errors → default browser navigation. Substrate truth is unaffected; any other doorway resolves identically. | §9 |
| **sitemap.xml** — hand-maintained, drifts | **Derived by construction** from projections × granted claims × commons enumeration — the static plane is enumerable because routing is data. It cannot drift from reality; it *is* a read of the routing table. | §7.5 |
| **rel=canonical** — SEO guesswork | The universal `/epr/{id}` is the durable canonical for content (bundle-agnostic, survives remounts); pretty mounts are claimed presentation addresses. | §5.1; slice2 plan |
| **.htaccess / nginx.conf** — config nobody audits | **The commitment table**: routing law is notarized, steward-authored, witnessed, revocable — legible governance artifacts instead of config files. | §3–§4 |
| **robots.txt / ad blockers / content filters** — adversarial scraping & blocking | **AttentionTending**: consumption-side, negotiated mediation. Content declares legibly (the epr-summary-hint); the person's tending agent decides follow/skip/wrap before fetching; the doorway never filters commons. The tending-immune floor protects what may never be silenced. | §2 R3, §5.3 |

## Appendix C — Learning-path template: *The Elohim Protocol for Web Developers* (NOT seeder-ingested)

**Status: template only.** This appendix scaffolds a future lamad learning path bridging from
what W3C / LAMP / MEAN-stack developers already know into the protocol's way. It is
deliberately **not** in seed-data form (no `contentFormat`/seed JSON — transformation happens
via `elohim-import` in a future content sprint); its job today is to pin the path's shape and
**link each step to the canonical spec it teaches**, so the curriculum stays born-linked to
this sprint's artifacts. Do not ingest.

**The recursion is the capstone**: the path will itself be served at `/epr/{id}`, claimed by
lamad's grant, with later steps reach-gated — so the learner *experiences* the inclusive-path
boundary at the moment the curriculum explains it. The medium demonstrates the message.

| # | Step (working title) | The bridge anchor (what you already know) | Protocol concept | Canonical source |
|---|---|---|---|---|
| 1 | Addresses that cannot lie | URLs, permalinks, ETags | Content addressing: CID as identity; slug vs fingerprint; new version = new address | semantic-computable-links-design; records-lifecycle §A.1 |
| 2 | The server that isn't the truth | LAMP vhosts / document root; Express `app` | Projection-of-substrate; the swap test; doorway as optional projection, never host | pillar-epr-decomposition §2, §12.1; stewardship-over-sovereignty §3 |
| 3 | .htaccess, notarized | .htaccess / nginx.conf / Express middleware | Routing law as commitments: declare + grant; aliases as revocable promises | this spec §3–§4 |
| 4 | 404 is a deploy bug, not a user experience | 404 pages; SPA fallback (`try_files`) | ROUTE/ASSET discipline; the universal address floor; designed boundaries | pillar-epr-decomposition §12.2; this spec §5, §7.3 |
| 5 | Walls become inclusive paths | 401/403, paywalls, login walls | Reach; head-edges; gateHints as the discovery-RPG quest surface | this spec §6; trust-compute-gradient §1–§3 |
| 6 | Pay at write time | CDN caching, rate limits, ad blockers | The inverted compute gradient; Commons Fast Path; AttentionTending as negotiated mediation | trust-compute-gradient; this spec §2 |
| 7 | Sitemaps that cannot drift | sitemap.xml, link checkers, W3C validators | The enumerable static plane; minted-never-literal; conformance classes | this spec §7 |
| 8 | Capstone: the address of this path | view-source | *This path's own URL resolves through the claims you just learned* — walk the commitment | this spec §5.1; epr-slice2-universal-address-plan |

Each step, when authored, carries: 2–4 atomic concepts (one per bridge-anchor row of
Appendix B where applicable), a `sophia-quiz-json` assessment translating a familiar-web
scenario into protocol terms, and `relatedNodeIds` pointing at the canonical sources above.
Audience variants (W3C / LAMP / MEAN) differ only in the anchor column's framing, not the
concept sequence.

## Appendix D — Navigation flows: mechanism → glue → scenario (traceability, informative)

The canonical map from each navigation flow to the mechanism that owns it, the glue code that
realizes it, and the scenario that validates it. A flow with an empty scenario cell is an
honest gap (tracked below or in the backlog).

| Flow | Mechanism | Glue code | Validating scenario(s) |
|---|---|---|---|
| Cold deep link to a mount route | §12.2 ROUTE/ASSET + `spa_fallback` | doorway `derive_app_subpath` · storage safety-net · `spa-route-discrimination.vectors.json` (two-layer) | deep-link: *shared path URL cold*, *deep link straight to a step* |
| Asset miss stays honest | §12.2 | same | deep-link: *asset miss stays an honest 404* |
| Universal address, unclaimed → shell viewer | §5.1 ServeShell | `dispatch_epr_universal`/`classify_epr_universal` · shell `epr/:resourceId` → lamad `ContentViewerComponent` · shell cross-pillar DI bridge (app.config) | deep-link: *renders unclaimed types in the shell viewer* |
| Universal address, claimed + anon-readable → pretty mount | §3 grant + §5.1 + `anon_reach_readable` | `EprRouter` claims index · grant flow (seeder→commitment→SSE) · 302 arm | deep-link: *302s a claimed type to its pretty mount* |
| Alias promises (mount + route-template) | §4 | `EprRouter::resolve_alias` · `redirectsFrom`/`redirectTemplates` on the commitment | deep-link: *monolith-era share honored by a notarized alias promise* (+ browser twin) |
| In-bundle minted links | §7.1 + §8.3 | `eprToRoute`/`eprToUniversalHref` · `claimsFromDeclaration(LAMAD_ROUTE_CLAIMS)` · `lint-route-literals` gate (in `just gate`) · `route-claims.vectors.json` | pinned by vectors + lint; exercised by every render scenario |
| Cross-bundle anchor + interceptor handoff | omnibar §4.2 | `epr-link-interceptor.ts` (capture-phase, fails open) · `EprNavService` | *View Resource Details* (@wip → un-wip with #6-2) — **nav-stack handoff has NO scenario (gap)** |
| Fragment upgrade (`#step/n`) | §5.2 (RFC 7231 passthrough + bundle template) | claim `fragments` + bundle router | **NO post-302 fragment-survival scenario (gap)** |
| Rendered-anchor conformance | §7.5 crawler | NOT BUILT (#7-5) | seed scenario: *View as Content honors the path claim* (operator click-path) |
| Gate face (gated boundary) | §6 | NOT BUILT (#6-2) | *View Resource Details* @wip-pinned |
| Sitemap | §7.5 | `render_sitemap` + generation invalidation | deep-link: *sitemap enumerates the claimed static plane* |
| SEO canonicals / og URLs | §7.1 keepers | seo.service via `eprToUniversalHref` | **NO canonical-correctness scenario (gap; `updateForProfile` bug open)** |
| EPR-head relationship navigation | §7 (HyperCard) | relationship boxes ← EPR Head | `epr-link-navigation.feature` (authed, 2 scenarios) |
| Context menu "Open in {pillar}" | parent §7.5 (claims-derived targets) | designed; rides the claims table | NOT BUILT (Slice-3-adjacent) |

## Appendix E — Reference-bearing surfaces & acquisition affordances (the `<script src>`/`<link href>` analog audit, informative)

`<a href>` is one of MANY reference-carrying surfaces. The audit below maps each web surface to
its protocol analog, its integrity story (the SRI analog), and its gap state.

| Web surface | Protocol analog | Integrity today | State |
|---|---|---|---|
| `<script src>` (app code) | bundle blob via projection (`/apps/{epr}/…`, hashed chunks inside) | blob hash verified at INGEST (`blob_store` PUT + reassembly) — **not re-verified at load**; apps-sw cache keyed by bundle hash (v1→v2 white-screen lesson; deterministic-zip + auto-invalidation = known Sprint-2 debt) | partial — load-time verification is an open design question (pair with the #13 link-audit attestation era) |
| `<script src>` (protocol-native components) | **element-registry manifest** (`tagName→cid`) + elohim-core `Loader` | **CID-verified client-side on every load, hard-fail on mismatch** (`verifyCid` default-on) — a real SRI-by-construction | ✅ strongest story in the stack — **no a2o scenario pins it (gap)** |
| WASM modules | in-bundle asset (oras-staged at build) | inherits bundle integrity; 404 → TS fallback (observed live) | ✅ acceptable |
| `<link href>` (styles/fonts/icons) | in-bundle assets | inherits bundle integrity | ✅ |
| `<link rel=canonical>` / og:meta | minted via `eprToUniversalHref` (§7.1 keeper) | lint-gated minting; correctness un-asserted | gap: canonical-correctness scenario; `updateForProfile` |
| `<img>/<video src>` in content | `/blob/{hash}` (content-addressed) | ingest-verified; **GET is reach-ungated — capability-by-hash semantics are an UNDOCUMENTED design decision** (a gated EPR's blob hash leaking = access leak; commons replication is the dataplane's job) | **design decision to make explicit (captured)** |
| `<a download>` / offline | — none — | — | see affordance ladder below |
| service worker | `apps-sw` (scope `/apps/`): peer-scored delivery + per-file cache | cache keyed by blob hash; Sprint-2 invalidation debt | partial ✅ |
| ES modules / importmap | Angular build internal | bundle integrity | ✅ |
| `epr:` URIs in content bodies | `EprResolver.resolveInContext` + relationship boxes | resolver-owned | ✅ scenarios exist |
| prefetch/preload hints | **epr-summary-hint** envelope (§5.3) — also the AttentionTending decision surface | designed | #5-3 deferred |

**The acquisition-affordance ladder (operator dimensions, 2026-06-07).** A link today offers
only *browse*. The protocol's link should offer a ladder, each rung composing existing
substrate primitives:

1. **Browse** — today's click (§5.1 resolution).
2. **Open in {pillar}** — the §7.5 context menu over the claims table (designed, unbuilt).
3. **Download / offline** — save locally (Tauri-direct already serves `:8090` doorway-free;
   browser offline = apps-sw cache + automerge offline-first for content). No UI affordance
   exists.
4. **Pin as peer ("save and replicate")** — downloading BECOMES provisioning: a REA *provide*
   commitment (`content:<reach>` provide rows — the Epic-B seam) + quilt custody
   (tiered-quilt, D5). The affordance turns a reader into a replication peer.
5. **Sync a cluster (parent EPR)** — an album / course / module is an EPR-head graph walk
   (relationships + path steps; `GateHintRelation::ContentToSync` is the existing vocabulary
   hook): fetch the closure, pin it as one custody unit.

**Multipeer transport (the torrent question, evidence-based 2026-06-07):**

- **Today**: apps-sw fetches a *scored* peer list (`/api/v1/peers/delivery`) and walks it in
  **sequential failover** — peer-aware, single-stream, whole-zip. No striping.
- **Substrate primitives already present**: `sharding.rs` + `p2p/blob_protocol.rs` +
  reassembly-verified `blob_store` (storage); **`elohim-bitswap`** (libp2p 0.54 port) — wired
  into **steward/node only**, not storage/browser delivery; RS(N,K) erasure quilt designed
  (tiered-quilt, D5).
- **Doorway stays single-target by gospel** (no fan-out) — swarm assembly belongs to the
  substrate (storage P2P) and the client delivery layer (apps-sw striping across scored
  peers), never the doorway.
- **Gap**: composing these into torrent-style parallel chunk striping (scored peers ×
  shard-ranged requests × hash-verified assembly) is designed-for but unbuilt — captured with
  the affordance ladder as one brainstorm seed (it is the bandwidth story FOR rungs 3–5).

**The async PULL queue (operator framing, 2026-06-07).** The substrate already has the WRITE
half: the publish **drain** queue (`status.drain {total, published, pending}`, watched by the
seeder's `wait-for-drain`) — local writes reconcile outward to peers. Rungs 3–5 are its mirror:
an **async pull queue** — a declared *desired-content set* (a pin, a cluster closure, an
offline subscription) reconciling INWARD with the same shape: `{total, fetched, pending}`,
resumable, prioritized, bandwidth-aware (the multipeer striping above is its transport),
hash-verified per item, and observable with the same wait-for semantics. This is the
P1 reconciliation-controller pattern pointed at the local node: the desired-set is the
manifest, the pull queue is the controller, `ContentToSync` gate hints and parent-EPR walks
feed it. Completing a pull at reach=commons naturally flips the node to *providing* (rung 4 —
the REA provide commitment), closing the read→host loop the protocol's economics expect.
One brainstorm seed covers the family: affordance ladder + pull queue + multipeer striping.
