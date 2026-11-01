---
title: Content apps wear chrome — a doorway never serves an html5-app bare to a visitor
id: content-apps-wear-chrome-design
status: Draft
class: architecture
serves: doorway-failover
date: 2026-10-14
context-tier: disclosed
steward: human:matthew
graduation-trigger: >
  On the household mesh, opening `/apps/<slug>/<entryPoint>` of an html5-app content node as a
  top-level page in Chromium shows the app itself under a thin protocol chrome whose resiliency
  badge links to `/epr/<id>`; the same address requested as an iframe (the lamad renderer, the EPR
  focal) returns the bundle's bytes unchanged, with no chrome added; and an app bundle that is
  itself a chromed EPR app (spa-bundle, e.g. lamad-spa, imagodei-portal) is served unchanged.
cites:
  - "elohim-seam-map-concern-routing | places app-serving and render runtimes peer-side and keeps the doorway to web2 delivery; this spec's navigation shell is delivery behaviour in that seam | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
---

# Content apps wear chrome

## The principle

Nothing should be seen through a doorway on the Elohim Protocol without the minimal chrome that
qualifies it: what it is, who holds it, at what reach, who stewards it and how it is governed.
A reader who follows a link to a piece of content meets it inside that frame, never as a bare
page that could have come from anywhere.

The raw address still has a job. Embeddings inside the protocol's own surfaces (the lamad
iframe renderer, the EPR page's focal frame) load the bundle from `/apps/<slug>/<entryPoint>`
and rely on it being available. So the raw bytes stay reachable as an embedded resource; what
changes is what a visitor gets when they open that address directly as a page.

## What happens today

- `GET /apps/<slug>/<path>` (doorway `routes/apps.rs`) serves the file from the html5-app zip
  for every caller, after the reach check. Opened directly, an html5-app renders bare.
- `/epr/<id>` (elohim-app `app.routes.ts`, the universal route) renders the chrome and frames the
  bundle through the lamad iframe renderer.
- Found 2026-10-14 and fixed in the same change as this spec: the renderer treated the
  same-origin doorway base (the empty string) as "no doorway configured" and framed the node's
  `fallbackUrl` instead of the bundle. Every html5-app with a fallback framed an outside copy
  (`evolution-of-trust` showed ncase.me; the DTPR letter showed a claude.ai page that refuses
  framing, so the frame was blank). The fallback now only feeds the "open elsewhere" link.

## Design

The doorway decides by the browser's own declaration of intent, which it already receives on
every request (Fetch Metadata), and never changes the content's bytes:

1. A request to `/apps/<slug>/<entryPoint>` with `Sec-Fetch-Dest: document` and
   `Sec-Fetch-Mode: navigate` is a person opening the content as a page. For an **html5-app**
   content node, the doorway answers with a minimal chrome shell instead of the bare entry
   file: `<elohim-page-chrome>` with its omnibar, a **resiliency badge** for the node (how many
   households hold it, from the same resilience view the EPR page reads) that **links to
   `/epr/<id>`**, and the app itself filling the rest of the viewport in a frame whose source
   is the same `/apps/<slug>/<entryPoint>` address. The person stays on the app; the chrome is
   one thin bar; the full EPR page (reach, holders, steward, governance) is one click away.
2. Requests with `Sec-Fetch-Dest: iframe`, `frame`, `embed` or any subresource destination,
   and every request for a non-entry path (scripts, styles, fonts, images), are served as today,
   byte for byte. That includes the shell's own frame, so the app inside it loads raw. The
   doorway never adds chrome to an embedded response: a surface that embeds content (lamad, the
   EPR page) already implements the chrome's primitives and links itself, and a second frame of
   chrome inside it would be noise.
3. An app bundle that carries its own chrome (`spa-bundle` content such as lamad-spa and
   imagodei-portal, whose root renders `<elohim-page-chrome>`) is served unchanged: it is already
   a chromed surface.
4. A request without Fetch Metadata (older clients, tools) is served as today. The rule
   qualifies what people see; it is not an access control, and the reach check stays the gate.

The content id and the badge's figures come from the storage lookups the `/apps/` arm already
performs (slug index, reach check, distribution summary), so the doorway learns nothing new and
holds no policy of its own: whether an app is "content" or "chromed shell" is its declared
`contentFormat`. Because the shell frames the bundle rather than rewriting it, the bytes a
person runs are still exactly the bytes addressed by the node's `blobHash`.

## Decisions taken while building (2026-10-14)

Three points the design left open were settled when the shell was cut into work, each the
narrowest reading that keeps the doorway's existing contracts:

- **The shell's omnibar is the native EPR omni element the doorway already serves**
  (`GET /chrome/omni-element.js`, injected with the same `inject_element` helper the SSR shell
  uses), not the Lit `<elohim-page-chrome>`. The doorway serves no Lit bundle at runtime, and
  the omni element is the chrome every doorway-painted surface already wears. The resiliency
  badge in the shell is a server-rendered link; the reusable `<elohim-resiliency-badge>`
  element in `elohim-core` carries the same wording for the surfaces that bundle Lit (the EPR
  page), so the two never drift in what they say.
- **The doorway learns a slug's content id, format and entry point from its own content
  projection only** (the same Mongo documents its slug index is built from). When that
  projection is cold or absent, the request is served raw, as today. No storage endpoint
  changes, and `_capability` / `X-Deliverability` keep describing bundle readiness, never
  shell success.
- **The chrome branch fires only when the resolved file equals the node's declared
  `entryPoint`** (`index.html` when none is declared). A trailing slash still maps to
  `index.html`; a node whose entry is another file wears chrome at that file's address.

Two consequences that are part of the shell, not afterthoughts: the shell is `no-store` and
both representations of the entry address carry `Vary: Sec-Fetch-Dest, Sec-Fetch-Mode`, so no
cache on the path (the doorway's file cache, the warm shell, a CDN) can hand a shell where
bytes are expected or the reverse; and the client apps service worker, which intercepts every
`/apps/` request under its scope, lets a top-level document navigation reach the network first
and falls back to its cached raw entry only when the network is away, so a direct visit meets
the shell online and the app still opens offline.

## Revision (2026-10-15): the chrome is the protocol's page-info surface

The operator, on seeing the first shell live on alpha: the chrome should do for EPR content what a
browser's own chrome does for a site. In Chrome, the icon beside the address opens page info,
which lets you inspect whether the connection is secure, read the certificate, see data and
cookies, and change site settings (clipboard, fonts, automatic downloads). This chrome stands in
for a browser built for the protocol, so it provides that kind of native inspection and those
settings for any EPR content. The bar itself stays as quiet as Chrome's omnibar. The thin
"Held by N of M households" bar shipped on 2026-10-14 is a second bar beside the omni chip, and
it goes.

### Three levels, each paid for only when asked

1. **The chip.** Collapsed, present by default, quiet. Its light or dark theme complements the
   content it frames.
2. **The omnibar.** Clicking the chip opens it. It is the existing omni element and stays
   minimal: one row, one word or icon per entry. Nothing is added at this level.
3. **Page info.** One glyph on the omnibar opens a single popover. Each row is a one-line
   reading, and opening a row turns it into a card with a back arrow, like a HyperCard stack.
   Every card ends in a link to the matching section of the EPR page, `/epr/<id>`, so nothing
   shown is a dead end.

| Row | One-line reading | The card holds |
|---|---|---|
| Source | Verified bytes, the content address | the record id, format, entry file, version and build stamp, receipt |
| Holders | Held by N of M households | the resilience tier, the recorded holders, the EPR page's holders section |
| Steward | the steward and the reach | governance, the steward's standing |
| Claims | how many claims back the record | each claim, who made it, its attestation |
| Permissions | what the content may do | each capability the content asks for and what this person granted |
| Yours (signed in only) | your relation to the content | your bookmark, progress, whether your household holds it, your claims |

A visitor who is not signed in sees the commons rows, and the permissions they set live in this
browser only. A signed-in person also sees **Yours**, and their permissions follow them across
devices. The omni context island already carries `authenticated`.

### Where each reading comes from (p2p design gate, 2026-10-15)

Seven of the eight sources exist and the omni element already reads most of them:

- the record: Notarized (A), `GET /api/v1/epr/{cid}`;
- claims and nav neighbours: `GET /api/v1/epr/{cid}/raw` and `/nav-context`;
- the resilience reading: Ephemeral (C), `GET /api/v1/resilience/{slug}`;
- byte verification and build stamp: Ephemeral (C), recomputed from the bytes.

New:

- **What the content asks to do.** A `capabilities` list in the html5-app record's `contentBody`
  (storage, downloads, clipboard, popups, forms). `contentBody` is opaque JSON, so this is
  DNA-hash neutral and needs no coordinator change. The doorway reads it as it reads `entryPoint`.
- **What a person grants.** The one new entity. Private (B), addressed by the agent-scoped tuple
  (agent, EPR id, `app-permissions`); for a visitor who is not signed in, the same choice is held
  in this browser only (C). No peer validates a grant, so it adds nothing to the head plane.
  SDO/RWA test: grants stay on the person's chain (or in this browser, for a visitor); a doorway sees what
  content asks for, which is public, and not what anyone granted.
- **The ambient theme hint.** Ephemeral (C): derived from the bundle's `color-scheme` and
  `theme-color` when the doorway resolves the descriptor. Derived, not read from the rendered
  content, because the chrome must not be able to read the content (next section).

The shell applies grants when it builds the frame: the frame's `sandbox` and `allow` attributes
are what the person granted from what the record asks. That is why the chrome loads first and
the content loads inside it.

### Decided (2026-10-15): a trustful network, two modes, and a separate content origin

**The network is trustful, and the chrome leans on it.** Content reaches people through
witnessed, inspectable gates run by agents tied to human operators on stewarded infrastructure.
VSM feedback withdraws reach, and agents can be dispatched to inspect content and send signals
back. The chrome therefore does not treat gated content as hostile.

**Authority comes from the network, not the doorway.** Every reading the chrome shows (the
record, its reach and declared capabilities, its witness, the holders, whether a wrapper fulfils
the claims) is derived from the p2p dataplane and the holons that govern the content. The doorway
projects those readings and enforces them at the web2 edge; it decides none of them. This is
what keeps the web from capturing or flattening trust at the doorway or the address: the same
content through any doorway, under any name, shows the same readings, so delivery stays plural.

**Two modes.** The chrome has two modes, switched by the record's gate standing, which is carried
in the context island and not inferred from the bytes:

- **Trustful.** Protocol-gated content, which is nearly everything authored at genesis. It gets
  the capabilities its record declares by default, and the chrome shows its standing and claims.
- **Trustless.** Content that is not Elohim Protocol content at all, such as legacy web projected
  inward. It runs in an opaque-origin sandbox with no capabilities by default, under web2
  trustless criteria. A protocol record whose reach is narrowed is not demoted to trustless:
  people outside the narrower reach are refused it, with the reason.

**Trustful wins.** The trustless mode does not dictate the trustful architecture. It is an edge
profile, and on a conflict it is the one that adapts.

**Trust lags delivery.** Content is served as soon as its bytes verify, and the chrome shows
amber. It turns green once the DHT has witnessed the record. The state is derived from
`dht_anchor_hash`; it is not written, and delivery does not wait for it.

**A separate content origin per public name.** Content is served on the public name's apps
origin, for example `apps.alpha.elohim.host`, `apps.elohim.host` and
`apps.doorway-alpha.elohim.host`. The pairing is derived from the public name, not from a
doorway instance: a request on `<name>` pairs with `apps.<name>`, and the reverse. Any doorway
answering a name computes the same pairing from the Host header alone, keeping no per-instance
state, so the design holds when doorways federate delivery under DNS anycast and balance load
over many peers on the p2p dataplane. The apps name is declared with the public name's
membership. Cache behaviour comes from the doorway's own headers, not a third-party CDN rule,
because doorways own the CDN projection. This
keeps the chrome and the person's session apart from every piece of content, so the chrome is
robust in both modes. That separation is a matter of robustness, not a defence against gated
content. The 2026-10-15 probe showed why it is needed: content served on the doorway's own
origin could rewrite the badge.

Requests that reach the apps origin are routed by Fetch Metadata:

| Request | Answer |
|---|---|
| A page navigation (`Sec-Fetch-Dest: document`) | Redirect to the chromed shell |
| A frame from another site (`Sec-Fetch-Site: cross-site`) | Redirect to the chromed shell |
| A same-site frame under a wrapper | The raw bytes |
| Raw HTML | Carries `frame-ancestors` naming the paired public name, and `no-store` |

This makes the rule hold: **the chrome comes off only inside an EPR wrapper that owns the context
and fulfils the claims**. A wrapper is recognised by a `presents` claim on its own record.

**Capabilities.** The frame's capabilities are the ones the record declares, within the ceiling
its standing allows, as the person grants them. Device capabilities (camera, microphone,
location) need the person's own grant, or their steward's under a stewardship grant.

**Signals back.** The doorway relays CSP violation reports to the content's holding peers, where
they are recorded as algedonic signals against the content's address; the governing community's
reviewers act on them through gates.

**Operator steps.** Three DNS records. Certificates are ordinary HTTP-01 ones.

Delivery order: (1) the content origin and the rule for when the chrome comes off; (2) chip, omnibar and page info; (3) capabilities follow standing, and signals flow back; (4) proof on the household, then alpha.

### The chrome makes the viable system visible (2026-10-15)

The operator's reading: the chain the chrome exposes is the protocol's own viable-system
recursion (Stafford Beer's VSM), made visible and discoverable to a person in an ordinary
browser. In the protocol, every holon (a household, a collective, a region) is a viable system,
and the elohim at each node is that system's Systems 3, 4 and 5
(`2026-06-14-recursive-architecture-design.md` §1.2). The algedonic bypass and the System-3*
audit channel are named in `2026-07-15-sense-respond-governance-classifier-design.md` §10.3. Page
info maps onto those channels:

| Page info | VSM | What the person sees |
|---|---|---|
| The content running in the frame; Delivery's peer; Holders | **System 1**, operations | the operational units that hold and serve the content: peers, households, a collective's pool |
| Delivery's doorway and name; amber turning green; the plural-delivery invariant | **System 2**, coordination | the anti-oscillation layer: doorways and the dataplane keep many deliveries describing one record the same way, and the DHT witness damps the interval before agreement |
| Permissions (declared ∩ admitted ∩ granted); reach | **System 3**, control | resources admitted inside the bounds the governing layer set |
| Projection inspection, from the backlog: loading directly from a holder and checking it against the content address | **System 3\***, audit | the sporadic direct look past the projection, by an admitted visit |
| Claims, including inspection attestations from reviewing agents | **System 4**, intelligence | what the system's scouts and reviewers learned about the content and its surroundings |
| Steward and governance; the EPR page's governs-layer chain | **System 5**, identity and policy | who governs the content, and under which layer's bounds |
| The signal relayed when content does what its record did not declare | **algedonic channel** | the pain signal that bypasses the hierarchy to the peers and the governing community's reviewers |

**Recursion.** The holder in a card can itself be a viable system: a household, or a
collective's commons pool with its own door sign and governance. Each EPR link descends into the
next level's account, so the person can climb the recursion as far as they choose, and every
level answers in the same shape.

### Returning visitors (2026-10-15)

The apps worker re-sent page openings itself, which Chrome labels `Sec-Fetch-Dest: empty`, so
everyone who had opened a protocol page before got the bare file. The worker now uses navigation
preload, so the browser sends the person's own request. Station 5 of
`epr-app-deliverability.feature` opens the entry address in a browser holding the worker built
from source.

## Links

Either address is a fine link once the shell exists: `/apps/<slug>/` opens the content itself
under thin chrome, `/epr/<id>` opens the full page about it. Until the shell is built, docs and
letters link to `/epr/<id>`, as the DTPR letter's README does.

## Debt

- The shell is not built. Homes: doorway `routes/apps.rs` (the navigation branch and the shell
  document), a resiliency-badge element in `app/elohim-elements/elohim-core` that renders a
  holders figure and an `href` (today the badge exists only inside the EPR page), and the a2o
  station named in the graduation trigger in `features/dataplane/epr-app-deliverability.feature`.
- `elohim-host-landing` is seeded as `html5-app` (genesis/data/lamad/content/elohim-host-landing.json;
  landing-page-dogfood.feature asserts it) although its root is a chromed shell. People meet it at
  `/`, which the doorway renders itself, never at its raw `/apps/` address, so the rule above only
  double-chromes a visit nobody is pointed to; still, the honest declaration is `spa-bundle`, and
  reseeding it with the dogfood story updated is open.
- `fallbackUrl` remains a field on html5-app content; whether a protocol-served node should
  carry an outside fallback at all is open.
