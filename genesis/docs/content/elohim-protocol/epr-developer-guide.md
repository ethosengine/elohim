# How Content Links Work in the Elohim Protocol

## The Problem With Links Today

Think about how links work on the internet right now.

When you click a link on YouTube, it takes you to a video on YouTube's servers. YouTube owns that address. If YouTube decides to remove the video, change its rules, or shut down, every link to that video breaks. The link isn't really yours — it belongs to the platform.

Now imagine a different kind of link. One that:

- Works no matter which computer is serving the content
- Knows who stewards the content and how they should be recognized
- Understands what community rules apply
- Changes its behavior based on where *you* are in your learning journey

That's what content links in the Elohim Protocol do.

## Three Things Every Link Carries

In most systems, a link is just an address — it tells you *where* something is. In the Elohim Protocol, every content link carries three dimensions:

### 1. What it is (Knowledge)

The title, the subject, how it relates to other content. Think of this as the label on a book: "Introduction to Economics, Chapter 3, relates to Chapter 7."

### 2. Who stewards it (Value)

Which humans steward this content — the nearest responsible caretakers — and how recognition flows to them. This is like the stewardship record on a community resource — except the system enforces it. You can't circulate the knowledge without the system knowing who to recognize.

When someone accesses this content, a small economic event is recorded — not a financial transaction, but a protocol-level acknowledgment that serving knowledge generates value for those who care for it. Recognition flows proportionally to each steward based on their allocation (Alice 60%, Bob 40%, for example).

The value isn't denominated in a single currency. It flows through multiple channels depending on the *kind* of contribution: a learning token for educational content, a care token for community support, a steward token for maintaining resources. Each community defines its own circulation rules — how fast tokens move, how much can accumulate, what thresholds matter — through their constitutional documents. This is the bridge between the content link you're looking at and the REA (Resource-Event-Agent) economic layer that tracks real contributions.

### 3. What rules apply (Governance)

Is this content personal (just for you)? Shared with trusted friends? Open to a community? Available to everyone? These aren't permissions bolted on after the fact — they're baked into the link itself.

**The key insight**: You literally cannot create a content link without all three. The system refuses to address content that doesn't declare its stewards and its governance. This is what we mean by "love as technology" — the architecture itself makes it structurally difficult to circulate knowledge without recognizing its stewards, and structurally easy to honor their care.

### What happens when nobody claims stewardship?

Here's one of the most important design decisions in the protocol: **unattributed content doesn't disappear — it flows to the commons.**

If a piece of content enters the system without a steward attesting to it, the constitutional governance layer takes responsibility. The content is governed by the community, value still flows — but to the commons pool rather than to any individual.

This solves a problem that kills most new systems: the cold-start problem. Content doesn't need a steward to be useful. It enters, it circulates, it teaches. But the moment someone recognizes their work — "I wrote this," "I maintain this," "I translated this" — they can attest to their stewardship, and value redirects to them.

Think of it like a library. Books on the shelf serve the community whether or not the author is standing next to them. But when the author shows up and says "that's mine," the library doesn't erase the book — it starts sending royalties. The knowledge was always accessible. The recognition was always waiting.

This creates a natural incentive: find your content, attest to your care for it, and the value flows. Don't, and it still serves the world — just under community stewardship instead of yours.

## Links That Know Where You Are

Here's the feature that makes this system genuinely different from anything else on the internet.

Imagine you're a student learning about economics. You're on Chapter 3 of a learning path called "How Communities Share Resources." In the text, there's a link to "Foundations of Fair Exchange."

On a normal website, that link always goes to the same page. But in the Elohim Protocol, that link is *context-aware*:

- **If "Foundations of Fair Exchange" is Chapter 6 in your current path**, the link takes you to Chapter 6 — keeping your place, your progress, your journey intact.

- **If it's in a different learning path**, the link lets you peek into that other path — a cross-reference, like a footnote that says "see also."

- **If you're just browsing freely** (not in any path), the link takes you to a standalone view of the content.

Same link, three different behaviors, depending on *where you are*. The link adapts to your context instead of forcing you into a fixed destination.

This recovers something the internet lost decades ago. In the 1980s, a program called HyperCard let people create cards with links that understood their context. When the web replaced HyperCard, we gained global reach but lost that contextual awareness. Every link became a fixed address to a fixed page.

The Elohim Protocol brings that awareness back.

## How Content Is Verified

When you download a file from the internet, how do you know it hasn't been tampered with? Usually, you don't. You trust the server.

The Elohim Protocol uses a technique called *content addressing*: instead of naming content by where it lives, you name it by *what it is*. The name is computed from the content itself — like a fingerprint. If someone changes even one character of the content, the fingerprint changes, and you know it's been altered.

This means:
- Content can be served by any computer, and you can verify it's authentic
- Content can be cached, copied, and shared without losing integrity
- No single server needs to be trusted

The browser-side verification is lazy-loaded (it doesn't slow down your initial page load) and has a 5-second timeout before falling back to a standard download. So it's invisible to the user — you just get verified content, fast.

## The Metadata Envelope

Every piece of content has a small metadata envelope — about 500 bytes, small enough to be whispered between computers through word-of-mouth networking. This envelope contains:

```
+--------------------------------------------------+
| "Foundations of Fair Exchange"                     |
|                                                    |
| Knowledge:                                        |
|   Type: concept                                   |
|   Format: article                                 |
|   Tags: economics, fairness                       |
|                                                    |
| Stewardship:                                      |
|   Steward: Alice (60%), Bob (40%)                 |
|   Token type: learning                            |
|   Recognition flows on delivery                   |
|                                                    |
| Governance:                                       |
|   Access: open to everyone                        |
|   Authority: community-level                      |
|                                                    |
| Connections:                                      |
|   Teaches: "mutual credit"                        |
|   Requires: "value flows"                         |
|                                                    |
| Fingerprint: bafyrei...  (content verification)   |
+--------------------------------------------------+
```

This envelope is how the system knows everything it needs about a piece of content without downloading the content itself. It's how search works, how previews render, how the three pillars stay coupled.

## How This Compares to What Exists

| | Traditional Links | Peer-to-Peer Links | Elohim Protocol Links |
|---|---|---|---|
| **Works if the server goes down?** | No | Yes | Yes |
| **Verifies content hasn't been tampered with?** | No | Yes | Yes |
| **Knows who stewards the content?** | No | No | Yes |
| **Knows what governance applies?** | No | No | Yes |
| **Adapts based on where you are?** | No | No | Yes |
| **Recognizes stewards automatically?** | No | No | Yes |
| **Unattributed content has a home?** | No | No | Yes (commons) |

## For Developers: Getting Started

### Adding a content link to a page

```html
<!-- Shows as a card with title, description, and preview on hover -->
<app-epr-link epr="epr:manifesto" display="card"></app-epr-link>

<!-- Shows as an inline text link -->
<app-epr-link epr="epr:fair-exchange" display="inline"></app-epr-link>
```

### Linking within written content

In any markdown document, content links are auto-detected:

```markdown
Learn about [fair exchange](epr:fair-exchange) to understand
how communities share resources without exploitation.
```

### Creating linkable content

Give your content a readable name (called a slug). That name becomes its permanent address:

```json
{
  "id": "fair-exchange",
  "title": "Foundations of Fair Exchange",
  "contentType": "concept",
  "contentFormat": "markdown",
  "contentBody": "# Fair Exchange\n\nHow communities share resources..."
}
```

Now anyone can link to it as `epr:fair-exchange` — and the link will carry all three pillars, resolve contextually, and verify the content's integrity.

## Further Reading

- **The Manifesto**: `genesis/docs/content/elohim-protocol/manifesto.md` — why this protocol exists
- **Protocol Specification**: `genesis/docs/content/elohim-protocol/protocol-specification.md` — the complete technical reference
- **Developer Skill**: `.claude/skills/epr-content-addressing/SKILL.md` — quick reference for building with these patterns

---

## Glossary

Technical terms used in the codebase, explained in plain language.

| Term | What it means |
|------|---------------|
| **Commons default** | When no one attests stewardship, content is governed by the community and value flows to the commons pool. Stewards can claim their content at any time, redirecting value to themselves. |
| **Content addressing** | Naming content by its fingerprint (what it IS) instead of its location (where it LIVES). Like identifying a song by its melody rather than which radio station plays it. |
| **Content fingerprint (CID)** | A unique identifier computed from the content itself. If the content changes, the fingerprint changes. Starts with `bafk...` in this system. |
| **DAG-CBOR** | A compact binary format for writing metadata envelopes. Think of it as shorthand — same information, smaller package, faster to transmit. |
| **Doorway** | A gateway server that connects browsers to the peer network. Like a lobby that helps you enter a building. |
| **EPR** | Elohim Protocol Reference — the system's native content link format. Written as `epr:content-name`. |
| **EPR Head** | The ~500-byte metadata envelope. The "label on the book" that describes what it is, who wrote it, and who can read it. |
| **Gossip** | How computers share metadata envelope information — by passing small messages to their neighbors, who pass them to their neighbors, like word of mouth. |
| **Helia** | A browser-side library that verifies content fingerprints. Makes sure what you downloaded is what was promised. |
| **IPFS / IPLD** | Peer-to-peer content sharing standards that the Elohim Protocol builds on. IPFS moves content between computers; IPLD structures how content relates to other content. |
| **Lamad** | Hebrew for "to learn." The knowledge pillar — what content IS and how it connects to other content. |
| **REA** | Resource-Event-Agent — an accounting pattern where economic activity is tracked as events (someone did something with a resource). The protocol uses REA to record when content is served and recognition flows to stewards. |
| **Recognition event** | The economic event created when content is delivered — acknowledgment that serving knowledge generates value. Not a financial transaction, but a protocol-level signal that flows to stewards. |
| **Qahal** | Hebrew for "assembly." The governance pillar — what rules and access levels apply to content. |
| **Shefa** | Hebrew for "abundance." The value pillar — who stewards content and how recognition flows. |
| **Three pillars** | The requirement that every content link carries knowledge context, value context, and governance context simultaneously. |
