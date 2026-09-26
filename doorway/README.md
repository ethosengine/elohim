# Doorway

> *"Knock, and the door will be opened to you."* — Matthew 7:7

The porch of the peer-to-peer network.

A physical porch has an address (so people can find you), a door (so people can enter), and a mailbox (so messages arrive). Doorway does the same for the Elohim Protocol: it gives the P2P network a web address, a way in, and a place for content to land.

## Why Doorway Exists

Holochain gives us agent-centric identity, content-addressed data, and cryptographic validation at the edges. What it doesn't give us is a URL. The web still runs on HTTP, DNS, and TLS. Billions of people navigate by typing addresses into browsers. Doorway is where those two worlds meet.

But a bridge can become a bottleneck — or worse, a chokepoint. Traditional federated systems (Mastodon, email) solve this by making instances authoritative for their users. If your instance goes down, you lose access to your data. If your admin decides to censor you, your content disappears.

Doorway refuses this bargain. A doorway is a **projection** of the DHT and peer fabric, not an authority over it. Your identity is your cryptographic key, not your doorway account. Your content lives in the distributed hash table, validated by DNA rules (Holochain's name for an app's validation logic) that no single operator controls. If a doorway misbehaves, you walk to the next one — or configure your own.

Running your own sounds like freedom until you're the one on call. Anyone who has self-hosted or run a fediverse instance knows three things wear you down: keeping it up, keeping it running, and answering for what it serves. Doorway is designed so that none of them falls on you alone.

**Uptime without the datacenter.** Your blog just has to work. But when your power flickers, your internet drops, or lightning finds your house, you go dark, and so does everything you host. That's why most people rent from a cloud provider. It's how a handful of companies turned high availability into rent and took the web's sovereignty along with it. Now the datacenters that sell it are going up in our neighborhoods, drawing on the same power grid our homes and communities depend on. With Doorway, your box is one of many. The peers you're connected to keep supplying what you'd otherwise rent: backup, failover, recovery, content delivery, and defense. When you go offline, your absence is felt, and the network moves to restore you. Defense works the same way. Many doorways can answer for the same name, so anything held on the mesh has plural DNS. A flood aimed at one doorway, or a firewall that blocks one domain, doesn't take the content down with it.

**Operations without the toil.** Every doorway runs with an [elohim operator](../genesis/docs/content/elohim-protocol/resilience/README.md): an AI agent that does the job a household IT person would do, if the household had one. Its inference doesn't have to run on the doorway's own box. It tells you when a disk is wearing out, when to order more RAM, when a service tech should come out, and when a disaster means the whole box needs replacing. When your doorway is carrying more than its share, it renegotiates the balance with your neighbors. Hosting becomes a joy, not a chore.

**Serving with responsibility held in trust.** A doorway doesn't dodge responsibility for what it serves. It owns it honestly. Responsibility is held in trust upstream, through governance negotiated among the people involved, and [reach is earned before anything spreads](../genesis/docs/content/elohim-protocol/values-forward.md), so what arrives at your doorway has already been governed. You are not an unpaid moderator standing alone, answerable for whatever your instance carries. The hardest cases, like [CSAM detection](../genesis/research/witnessed-harm-limit-research-2026-08-09.md), are met with real care rather than promises, and the safety intelligence every network needs lives in the commons, not on your shoulders.

Attribution is held upstream too. The network doesn't borrow an outside copyright filter; it holds itself to a higher bar. Every contributor is recognized, and what their labor produced is theirs, [held in trust](../genesis/docs/content/elohim-protocol/shefa.md) until they claim it, even if they haven't joined yet. Beyond that, knowledge becomes [a true commons, the common inheritance of all humanity](../genesis/docs/content/elohim-protocol/values-forward.md), held in trust and in love. The network stakes that claim on stewardship of the ties that bind everyone who creates, not on anyone's landed claims. Your role and your right to take part are protected by the protocol's constitutional contract, not by the goodwill of whoever runs the next server over.

**Restoring trust to the web.** Governed serving pays off beyond the network itself. A site served through a doorway carries the frame of an EPR (Elohim Protocol Record), visible to anyone on the ordinary web. A visitor who has never heard of Elohim can see that the site answers for its own story on the commons. They can inspect its claims, the governance behind it, and the story it presents. If something seems wrong, whether misinformation, a missing claim, or anything else, [every piece of content carries a feedback channel](../genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md) where they can talk it through with an agent, and the network owes them a response. Its shared intelligence [tells a genuine complaint from someone spamming the form](../genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md), and a genuine one can pull back a surface's reach while it's reviewed. It's the same care that helps a steward negotiate a decision against a limit their council has set, and the whole network carries it together, through the inference commitments everyone shares. The site's standing is earned, not bought or merely asserted. As bad faith rips through the legacy internet, and AI slop turns the dead-internet theory into everyday experience, that matters. It's a human-centered answer for a web lost in a firehose of falsehood: a place where trust can be earned again.

## What a Doorway Steward Does

Running a doorway is an act of stewardship. You provide compute, bandwidth, DNS, and a projection cache so that your community's content is accessible from the web. In return, the protocol makes your contribution legible — the work you do is measurable, governable, and valued through shefa (the economic pillar).

A doorway steward for a PTA, a church, a co-op, or a neighborhood is doing the same thing a community librarian does: making knowledge accessible while respecting the relationships that govern it. The difference is that the governance is cryptographic, the catalog is a DHT, and the building is a box on a shelf or in a closet.

### The Steward's Own Identity

The steward is also a human in the network. Their identity lives in the conductor pool alongside hosted users — same DNA, same DHT participation. The only difference is that they control the hardware. They are the first peer in their own doorway.

## How Content Reaches the World

Content flows through doorway in two paths:

**Visitors** — people browsing from the open web — hit the projection cache. No Holochain identity needed. No conductor cells consumed. This is a web server reading from a database. Every scaling technique the web already knows applies here.

**Hosted humans** — people who have created accounts but haven't yet graduated to their own devices — write through the conductor. Their custodial keys live in doorway's memory. Their cells participate in the DHT. This costs real resources: RAM, CPU, gossip bandwidth.

The beautiful thing is that the second group shrinks over time. As people graduate to running their own nodes, their conductor cells are freed. The steward's identity-hosting load *decreases as doorway succeeds*. Meanwhile, the projection layer may be busier than ever — because the content is good, and the world is reading it.

## Federation Without Lock-In

Doorways federate with each other, but the federation is fundamentally different from the fediverse:

| | Traditional Fediverse | Doorway Federation |
|-|----------------------|-------------------|
| **Authority** | Instance owns user data | DHT owns all data |
| **Lock-in** | Users tied to home instance | Users switch doorways freely |
| **Replication** | Instance-to-instance (O(n^2)) | DHT gossip (automatic) |
| **Validation** | Trust between instances | Cryptographic, edge-enforced |
| **Identity** | Instance-relative (@user@host) | Agent keys (portable) |
| **Censorship** | Switch instances, lose history | Switch doorways, keep everything |

Peers can register with multiple doorways for redundancy, account recovery, and geographic distribution. Doorways share projection caches. The network doesn't need one giant doorway — it needs many small ones, each serving their community, each with content worth reading and people worth hosting.

## The Escape Hatch

If a doorway misbehaves, users can:
1. **Switch to another doorway** — any doorway can serve any content from the DHT
2. **Run their own** — doorway is open source, runs anywhere
3. **Go direct** — connect to Holochain without any doorway at all
4. **Verify independently** — all data is content-addressed and signed

A doorway is operationally useful but architecturally replaceable. It cannot create entries without agent signatures, cannot modify data that fails DNA validation, cannot prevent users from leaving, cannot access private data without authorization, and cannot censor content that other doorways will serve.

This is the design constraint that keeps doorway honest: it's useful *because* it can't capture you.

## What's Here

```
doorway/
  doorway-service/   Rust gateway — bootstrap, signal, conductor proxy,
                     route registry, projection cache, identity hosting
  doorway-app/       Angular operator dashboard — node health, federation,
                     graduation pipeline, user management
```

See `CLAUDE.md` for developer guidance.
