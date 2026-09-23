# ActivityPub Federation — Prior Art for the Doorway

> The doorway's federation leg has an ActivityPub bridge filed as *planned*. Distributed Press
> (Hypha Co-op) has already shipped it from the publishing side. This note captures their design
> as prior art for that leg — to revisit when we brainstorm doorway federation.
>
> Full cross-cutting survey (incl. the Hypercore substrate thread):
> [`genesis/research/hypha-distributed-press-cross-pollination-2026-06-23.md`](../../genesis/research/hypha-distributed-press-cross-pollination-2026-06-23.md).
> Sibling projection leg already specced: [`atproto-lexicon-projection-doorway`](../../genesis/docs/superpowers/specs/2026-05-01-atproto-lexicon-projection-doorway-design.md).
> Live doorway federation surfaces this would extend: [doorway/CLAUDE.md §Federation](../CLAUDE.md), [`FEDERATION.md`](../doorway-service/FEDERATION.md), and the [doorway-consolidation-federation-arc history](../../genesis/docs/content/elohim-protocol/history/2026-06-11-doorway-consolidation-federation-arc.md) (live JWKS · DID-doc · `DoorwayRegistration` mechanisms).

---

## The prior art

[**Distributed Press**](https://distributed.press) publishes one site to HTTP + IPFS + Hypercore at
once and *seeds* it so anyone can co-host — resilience through many providers rather than one host.
Its [**Social Inbox**](https://github.com/hyphacoop/social.distributed.press) (1.0,
[Dec 2023](https://news.compost.digital/2023/12/04/announcing-distributed-press-social-inbox-1-0.html))
is the piece that mirrors the doorway: a *minimal ActivityPub server* that gives static /
distributed-web sites a live fediverse presence by splitting AP in two —

- **Static, published half** carries the durable objects: the **Actor**, **Outbox**, **Posts** (AP
  "notes"), and **WebFinger** discovery — plain JSON, servable from anywhere (incl. IPFS/Hypercore).
- **A small dynamic server** carries only what static files can't: inbound **follows, replies,
  likes, boosts**, and outbox fan-out to followers' inboxes.
- **Auth = HTTP Signatures over the site keypair. No login.** Register keys + actor location; the
  server verifies every inbound signature.
- **Moderation = allow/block lists at actor *and* instance level**; inbound activities queue to be
  accepted or deleted (manual or automated).
- **Interactions flow back to the static site**: approved replies become on-page comments; the
  follower list downloads back onto the site.

The keystone is [**FEP-1042 "Peer to Peer Fediverse Identities"**](https://codeberg.org/fediverse/fep/src/branch/main/fep/1042/fep-1042.md) —
a **draft** Fediverse Enhancement Proposal ([announced by Distributed Press](https://distributed.press/2024/08/14/our-shiny-new-bridge-between-peer-to-peer-protocols-and-activitypub-implementations/) as "FEP-1024" before the registry reassigned the number to 1042) that links a P2P document to an **HTTPS *alias* URL**, so a
Mastodon-class instance (which demands always-online HTTPS/DNS actors) can reference and interact
with content that actually lives on IPFS/Hypercore — *without the fediverse needing native P2P
support*, and surviving partial web outages. The companion client is
[**Agregore**](https://agregore.mauve.moe/) (Mauve / AgregoreWeb): a minimal browser speaking
`hyper://` / `ipfs://` natively — the "loyal client" of the
[three-legged stool](../../genesis/research/zuckerman-three-legged-stool-2023.md), made real.

## Why this fits the doorway exactly

The Social Inbox split mirrors the doorway's own **projection-vs-Operational-state** cut. Distributed
Press applies it to AP: the Actor/Outbox/Posts are *projections* of the DHT (the doorway already
serves content-addressed content from cache — the **swap test** passes), while the dynamic inbox
(follows/replies/likes) is *doorway-local Operational state* — the category the doorway CLAUDE.md
carves out for cache stats and the federation peer list. (This is **not** the doorway's Axis-1/Axis-2
scaling split: the inbox queue is Operational state, not conductor-bound identity hosting. And the
Social Inbox's outbox *fan-out to followers' inboxes* is AP delivery — unrelated to the doorway
**No Blob Fan-Out** rule, which forbids iterating storage peers to find bytes.)

## Design implications to carry into brainstorming

1. **The AP bridge should be a thin inbox leg, never a Mastodon instance.** The live inbox queue =
   *doorway-local Operational state* (same category the [doorway CLAUDE.md](../CLAUDE.md) carves out
   for the federation peer list and cache stats); Actor/Posts stay projections of the DHT. **Follower
   relationships are a separate, open question** — not assumed Operational (see Open questions below).
2. **Adopt FEP-1042 alias-bridging at the projection layer** *(highest-leverage learning)*. The
   doorway already serves DHT content at HTTPS paths (CDN-edge projection — the canonical address
   stays the content-derived CID) and already serves `/.well-known/did.json`. FEP-1042 (a *draft*
   proposal) declares "this HTTPS doc aliases this DHT doc," turning an agent-key identity into
   something **followable from Mastodon without the doorway owning it**. ActivityPub is the sibling
   *federation flavor* the committed `atproto-lexicon-projection-doorway` spec anticipates (that spec
   *defers* AP generalization until a driver appears); FEP-1042 is the identity-bridging mechanism
   that flavor brings — analogous to how the atproto spec handles `did:plc` via a `ProjectionClaim`,
   not to the whole spec.
3. **Federation auth rides the agent keypair, not a doorway account.** DP signs inbox ops with the
   site keypair (HTTP Signatures, no login). The doorway's identity is already the Ed25519 agent
   key; AP activities should be signed with that (or a derived) key — keeps *identity = key, not
   account* intact through the federation surface.
4. **Publish-once / project-to-many, past HTTP.** DP emits HTTP + IPFS + Hypercore from one publish.
   The doorway projects the DHT to HTTP today; the pattern says surface `ipfs://` / `hyper://` (or
   FEP-1042 aliases) too, so distributed-web clients reach content directly — the doorway's own
   *"useful because it can't capture you"* thesis extended below HTTP.
5. **Inbound-federation moderation: queue-and-approve, allow/block at actor+instance.** A concrete
   minimal design for the moderation surface foreign replies/follows will need — and a natural seat
   for the three-legged stool's "friendly neighborhood algorithm store" shared Trust-&-Safety
   tooling. Doorway-local Operational state, legitimately.

## Divergences / cautions

- **Trust root differs.** DP is a *service* whose site identity is the publishing keypair and whose
  inbox server holds follower state; availability leans on the DP instance. A doorway is a
  *swappable projection of a validating DHT* — walk to the next doorway, keep everything. Lift the
  "split static from dynamic" shape; do **not** import the instance-as-home trust root.
- **FEP-1042 anchors identity in HTTPS/DNS aliases.** Adopt it as a *projection convenience at the
  doorway*, not as an identity primitive — the same stance the
  [dds-wg survey](../../genesis/research/dds-wg-cross-pollination-2026-05-01.md) took on AT-Proto
  lexicons. Content-addressed provenance stays the spine; web-DNS is a convenience surface.
- **p2p-vs-federation layering.** AP/FEP-1042 is a *federation*-layer concern (doorway over p2p).
  NAT traversal / discovery is a *p2p*-layer concern (steward/node) — see the
  [Hypercore prior-art note](../../steward/node/research/hypercore-holepunch-prior-art.md). Route
  doorway-selector/JWKS/AP gaps to federation; iroh/libp2p gaps to p2p.

## Open questions for the design gate

- Where does the FEP-1042 alias live — in the DID document the doorway already serves (it carries
  `elohim:capabilities`; identity is served at `/.well-known/did.json`), or a parallel
  `/.well-known` surface? Does the alias resolve to the content-derived CID (not a doorway-minted
  id), so a sibling doorway publishes an identical alias (swap test)?
- Inbox follower-store: which entity-class under `p2p-design-gate`? (Working hypothesis: doorway-local
  Operational (C) for the live inbox queue; followers/relationships may want a notarized form.)
- Can the AP Actor be served as a pure projection of an existing DHT identity/presence entry, so a
  sibling doorway serves the identical Actor (swap test)?
- One inbox server per doorway, or per hosted-agent? When a hosted human graduates to their own
  device, does their inbox leg travel with them (the identity-hosting load shrinking as it should)?
