---
title: Hypha / Distributed Press / Hypercore Cross-Pollination
status: Capture
date: 2026-06-23
---

# Hypha · Distributed Press · Hypercore Cross-Pollination — June 2026

Two adjacent traditions, surveyed together because the doorway sits exactly where they meet:

- **[Hypercore Protocol](https://github.com/hypercore-protocol)** — the append-only-log + holepunching-DHT substrate, now developed under [Holepunch](https://github.com/holepunchto) and the [Pear runtime](https://docs.pears.com/). A *substrate / data-plane* prior art.
- **[Hypha Worker Co-operative](https://github.com/hyphacoop)** (Toronto) and their **[Distributed Press](https://distributed.press)** publishing platform + **[Social Inbox](https://github.com/hyphacoop/social.distributed.press)**. A *projection / federation* prior art — the closest external mirror of the doorway's job we have found.

These map onto two of the README's standing problems: the Hypercore thread informs **The Networking / Content-Addressing Problem**; the Distributed Press thread informs **The Edge Problem** (`doorway/`). The doorway's own thesis — *"a projection of the DHT, not an authority over it,"* federation that *"rides p2p"* rather than instance-to-instance — is precisely what Distributed Press has been building toward from the publishing side. The ActivityPub bridge the doorway has filed as *planned* already exists in their codebase, shipped, in production.

---

## Subject 1 — Hypercore Protocol → Holepunch / Pear

**The approach.** A Hypercore is *a secure, distributed append-only log*: **single-writer** (one keypair owns write access), integrity-secured by a **signed BLAKE2b merkle tree**, and replicated **sparsely** — peers *"only download the data they are interested in."* On top of the single core sit the rest of the stack: **Hyperbee** (B-tree KV store), **Hyperdrive** (a filesystem = metadata core + content core), **Corestore** (manages many cores), and **Autobase** (linearizes *multiple* single-writer cores into a multi-writer structure). Discovery and transport are **Hyperswarm** over **HyperDHT**: peers find each other by a *topic / `discoveryKey`* (derived from the core key, so you find peers *without leaking the key*), and **holepunching is a first-class feature** — UDP holepunching across NATs/firewalls into Noise-encrypted streams. The end state is **Pear**: ship fully P2P desktop/mobile apps with *no servers at all*. (The `hypercore-protocol` org is now legacy — `hypercore-next`/`corestore-next` are archived; active work is in `holepunchto`; Hypercore 11 (current, RocksDB-backed for storage + atomicity) supersedes the v10 LTS line.)

**What's worth learning.**

1. **A Hypercore is structurally a Holochain source chain.** Single-writer, append-only, hash-linked, signed by the owning key. The instructive part is the *divergence*: Hypercore is *"trust the key, no shared validation, discovery-only DHT,"* whereas Holochain is *validation-intrinsic-to-data-type over a graph/validating DHT*. Hypercore shows how far you get with logs + holepunching alone (serverless apps) — and what you trade away (no cross-agent validation, no graph queries). That trade is exactly the pain named in `.claude/prompts/p2p-dataplane-sprint.md` ("DHT chokes at ~3000 entries, no query capability"). Hypercore is a candidate **data-plane**, never a **truth-plane**.
2. **Sparse replication = "replication follows relationship."** *"Download only the blocks you need"* is the off-the-shelf shape of the p2p-dataplane sprint's hardest requirement ("not everyone has everything") and a direct comparator for the self-healing dataplane's byte-mobility work — sitting beside iroh/libp2p, not above the Holochain DHT.
3. **Autobase is prior art for multi-writer convergence** — the "how do N agents converge a shared structure" problem we currently answer with Automerge. Worth reading for how it linearizes single-writer logs without a global consensus bottleneck.
4. **HyperDHT is one of the most mature holepunching-first DHTs in the JS ecosystem** (UDP holepunching is a first-class feature, deployed at scale via Holepunch/Keet). This is a *p2p-layer* learning (steward/node, the live WAN-NAT backlog), **not** a doorway/federation one — see the p2p-vs-federation layer split. Its discovery-by-topic model also rhymes with "steward topology emerges from content affinity": a `discoveryKey` per affinity-topic is one concrete shape for affinity-weighted peer discovery.

---

## Subject 2 — Hypha Co-op & Distributed Press (the doorway mirror)

**The approach.** Distributed Press *"automates publishing and hosting content to the web that it seeds to decentralized protocols."* One publish action emits the same site to **HTTP + IPFS + Hypercore**, and anyone can *"choose to seed and help co-host your content"* — resilience through many providers rather than one host. The platform is an **API** (Site / Publisher / Admin / Auth), a **CLI**, a **Site Indexer**, a **Social Reader**, and — the piece that matters most for us — the **Social Inbox**.

The **Social Inbox** (1.0, [Dec 2023](https://news.compost.digital/2023/12/04/announcing-distributed-press-social-inbox-1-0.html)) is a *minimal ActivityPub server* that gives static / distributed-web sites a live fediverse presence. Its design is a clean split:

- **Static, published content** carries the *durable* half of ActivityPub: the **Actor** profile, the **Outbox**, the **Posts** (as AP "notes"), and **WebFinger** for discovery — plain JSON files, servable from anywhere (including IPFS/Hypercore).
- **A small dynamic server** carries *only* what static files can't: receiving **follow requests, replies, likes, boosts**, and fanning **outbox** posts to followers' inboxes.
- **Auth is HTTP Signatures over the site's keypair — no login.** You register your keys + actor location; the server verifies every inbound request's signature.
- **Moderation is allow/block lists at actor *and* instance level**, with inbound activities *queued to be accepted or deleted* (manually or automated).
- **Interactions flow back to the static site**: approved replies become on-page comments; the follower list downloads back onto the static site.

And the keystone: **[FEP-1042 "Peer to Peer Fediverse Identities"](https://codeberg.org/fediverse/fep/src/branch/main/fep/1042/fep-1042.md)** — a **draft** Fediverse Enhancement Proposal that Distributed Press authored to bridge P2P and ActivityPub ([announced Aug 2024](https://distributed.press/2024/08/14/our-shiny-new-bridge-between-peer-to-peer-protocols-and-activitypub-implementations/) as "FEP-1024," before the FEP registry's content-addressed ID check reassigned the number to 1042; merged 2025-04-03). It links the **P2P version of a document to an HTTPS *alias* URL**, so a Mastodon-class instance (which requires always-online HTTPS/DNS actors) can reference and interact with content that actually lives on IPFS/Hypercore — *without the fediverse needing native P2P support*, and surviving partial web outages. The companion client is **[Agregore](https://agregore.mauve.moe/)** (Mauve / AgregoreWeb): a minimal browser that speaks `hyper://` / `ipfs://` natively, authors content locally via `fetch()`+`PUT`/`DELETE`, and auto-reshares P2P sites — the "loyal client" of the [three-legged stool](zuckerman-three-legged-stool-2023.md) made real.

---

## What we'd lift

1. **Split static-projection from dynamic-inbox — and keep the inbox *thin*.** The Social Inbox validates the doorway's whole federation stance: the Actor / Outbox / Posts are *projections* (the doorway already serves content-addressed content from cache; the swap test passes), and **only** the dynamic inbox (follows, replies, likes) needs a small stateful server. The doorway's planned ActivityPub bridge should be a **thin inbox leg, never a Mastodon instance**. The live **inbox queue** is legitimate *doorway-local Operational state* (the category the doorway CLAUDE.md carves out for the federation peer list and cache stats). The **follower relationships** themselves are an open `p2p-design-gate` question, *not* settled Operational state — a follow is an agent-to-agent relationship (the gate's Category A/A2: notarized, or a link on an existing identity/presence entry), and unlike a cache entry it isn't reconstructable if the doorway's table is wiped. Run the gate before assuming Operational.
2. **Adopt FEP-1042 alias-bridging at the doorway projection layer.** This is the single highest-leverage learning. The doorway already serves DHT-resident content at HTTPS paths (CDN-edge projection — the canonical address stays the content-derived CID) and already serves `/.well-known/did.json`. FEP-1042 is a *draft* proposal for a convention to declare *"this HTTPS doc aliases this P2P/DHT doc,"* turning an agent-key identity into something **followable from Mastodon without the doorway owning the identity**. ActivityPub is the sibling *federation flavor* the atproto-projection spec (`2026-05-01-atproto-lexicon-projection-doorway-design.md`) already anticipates — that spec explicitly *defers* ActivityPub generalization until a driver appears; FEP-1042 is the specific identity-bridging mechanism the flavor brings, analogous to how the atproto spec handles `did:plc` via a doorway `ProjectionClaim` rather than importing it as a peer identity.
3. **Federation auth rides the agent keypair, not a doorway account.** Distributed Press authenticates inbox ops with HTTP Signatures over the site's keypair, *no login*. The doorway's identity is already the Ed25519 agent key; AP activities should be signed with that (or a derived) key. Keeps the protocol's line — *identity = key, not account* — intact through the federation surface.
4. **Publish-once / project-to-many, past HTTP.** Distributed Press emits HTTP + IPFS + Hypercore from one publish. The doorway projects the DHT to HTTP today; the pattern says the projection layer could *also* surface `ipfs://` / `hyper://` (or FEP-1042 aliases) so distributed-web clients (Agregore-class) reach content directly. That is the doorway's own *"useful because it can't capture you"* thesis extended below HTTP — more providers, more escape hatches.
5. **Inbound-federation moderation: queue-and-approve, allow/block at actor+instance.** A concrete, minimal design for the moderation surface the doorway's inbox leg will need for foreign replies/follows — and a natural seat for the [three-legged stool's](zuckerman-three-legged-stool-2023.md) "friendly neighborhood algorithm store" / shared Trust-&-Safety tooling. Doorway-local Operational state, legitimately.

---

## Where our paths diverge

- **Trust root.** Distributed Press is a *service* whose site identity is the publishing keypair and whose inbox server holds follower state; availability leans on the DP instance. A doorway is a *swappable projection of a validating DHT* — walk to the next doorway and keep everything. The convergence on "split static from dynamic" is strong; the root of trust is not.
- **Hypercore is integrity, not validity.** It guarantees a log wasn't tampered with by anyone but the key-holder; it does **not** run shared validation rules. So it cannot be the protocol's truth layer — it's a candidate blob/data-plane substrate beside iroh/libp2p, never a replacement for the Holochain validating DHT.
- **FEP-1042 anchors identity in HTTPS/DNS aliases.** We adopt it as a *projection convenience at the doorway*, not as an identity primitive — the same stance the [dds-wg survey](dds-wg-cross-pollination-2026-05-01.md) took on AT-Proto lexicons (interop at the projection layer, content-addressed provenance stays the spine). Web-DNS is a convenience surface, never the source of truth.

---

## Outputs

Matching the pattern the [dds-wg survey](dds-wg-cross-pollination-2026-05-01.md) set, this engagement landed:

- **Manifest clones** (in [`research-manifest.json`](research-manifest.json), clonable via `research.sh`): `social-distributed-press` (pillar: doorway), `hypercore` (pillar: elohim), `hyperdht` (pillar: elohim).
- **Module-boundary pointer notes**: [`doorway/research/activitypub-federation-prior-art.md`](../../doorway/research/activitypub-federation-prior-art.md) (federation leg) and [`steward/node/research/hypercore-holepunch-prior-art.md`](../../steward/node/research/hypercore-holepunch-prior-art.md) (data-plane).
- **README enrichment**: the **Edge Problem** (doorway), **Networking Problem** (steward), and **Content Addressing Problem** sections now carry this prior art.

Still open:

- **A spec stub** — *doorway ActivityPub inbox leg via FEP-1042* — not yet authored. It would be an ActivityPub-flavor companion to `2026-05-01-atproto-lexicon-projection-doorway-design.md` (which explicitly *defers* ActivityPub generalization until a driver appears), gated through the `p2p-design-gate` skill (live inbox queue = Operational; actor/posts = projections; follower relationships = run the gate).

## Credit

Hypha Worker Co-operative built and operate Distributed Press and the Social Inbox; **Mauve (RangerMauve / Mauve Signweaver)** drove much of the Social Inbox, FEP-1042, and Agregore. The Holepunch team (formerly the Hypercore Protocol / Dat project) built Hypercore, Hyperswarm, and the holepunching DHT. Their work made this engagement worthwhile.
