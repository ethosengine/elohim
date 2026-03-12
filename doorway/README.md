# Doorway

> *"Knock, and the door will be opened to you."* — Matthew 7:7

The porch of the peer-to-peer network.

A physical porch has an address (so people can find you), a door (so people can enter), and a mailbox (so messages arrive). Doorway does the same for the Elohim Protocol: it gives the P2P network a web address, a way in, and a place for content to land.

## Why Doorway Exists

Holochain gives us agent-centric identity, content-addressed data, and cryptographic validation at the edges. What it doesn't give us is a URL. The web still runs on HTTP, DNS, and TLS. Billions of people navigate by typing addresses into browsers. Doorway is where those two worlds meet.

But a bridge can become a bottleneck — or worse, a chokepoint. Traditional federated systems (Mastodon, email) solve this by making instances authoritative for their users. If your instance goes down, you lose access to your data. If your admin decides to censor you, your content disappears.

Doorway refuses this bargain. A doorway is a **projection** of the DHT, not an authority over it. Your identity is your cryptographic key, not your doorway account. Your content lives in the distributed hash table, validated by DNA rules that no single operator controls. If a doorway misbehaves, you walk to the next one — or build your own.

## What a Doorway Steward Does

Running a doorway is an act of stewardship. You provide compute, bandwidth, DNS, and a projection cache so that your community's content is accessible from the web. In return, the protocol makes your contribution legible — the work you do is measurable, governable, and valued through shefa (the economic pillar).

A doorway steward for a PTA, a church, a co-op, or a neighborhood is doing the same thing a community librarian does: making knowledge accessible while respecting the relationships that govern it. The difference is that the governance is cryptographic, the catalog is a DHT, and the building is three blades in a closet.

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
