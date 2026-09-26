# The Neighborhood

> *"A matter must be established by the testimony of two or three witnesses."* — Deuteronomy 19:15

The rules and networks households share.

This directory holds the neighborhood of the Elohim Protocol: the shared rules households agree to live by, and the networks those rules create, where each household's word is checked by the others. If you came in through the [doorway](../../doorway/README.md), this is the street outside the house you just walked through. No household here vouches for itself. Every record a doorway serves has been checked by someone other than the one who wrote it.

Rules do little until people hold them in common, so this README explains what the neighborhood is and why it is built this way, and leaves building and testing the rules to the developer guides listed at the end.

## Why the Neighborhood Exists

Holochain is a framework for peer-to-peer applications. It gives each person an identity that is their own cryptographic key rather than an account on someone's server, names every piece of data by a fingerprint of its bytes, and puts validation at the edges, in the hands of the participants themselves. There is no central server deciding what is true. Deciding is left to the members.

A house can hold anything, and it can say anything about itself. A record is only as good as the witnesses behind it. In the neighborhood, each person signs their own actions in their own chain, and the members of a space check each record against the rules they hold in common. One household's claim is testimony; the neighborhood is where it meets its witnesses.

**Not one network but many.** Arthur Koestler called a whole that is also a part a *holon*: a cell within an organ, a household within a village. Holochain makes the holon the unit of the network, with one set of shared rules per group. So there is no single "the network." There are the networks you are a member of. A family's own devices share household rules. A collective such as a church, a co-op, or a school has a larger membership and rules of its own. The commons holds the protocol's public content and learning paths, read by thousands and kept by tens of stewards. Each of these spaces keeps its own distributed hash table (DHT), a shared record kept in pieces by its members. Your device is where your holons meet.

**The rules are the network.** Holochain calls an app's validation logic its DNA, and the DNA's hash *is* the network. Change the rules and you have founded a new network, a new neighborhood, the way amending a street's covenant would found a different street. That is why anything that should stay adjustable, such as how far a piece of content reaches or how the system is tuned, is never baked into the rules themselves.

**Each space checks its own.** Members validate against their own space's rules and no one else's. A household validates household rules, and that is all it can do. It cannot vouch for commons reach on your behalf. Getting there takes promotion: a witnessed re-publish into a wider space.

## How Reach Is Earned

The doorway promises that [reach is earned before anything spreads](../../genesis/docs/content/elohim-protocol/values-forward.md). Promotion is how the neighborhood keeps that promise. A household note becomes a collective's deed, and the deed becomes a commons page, each step a witnessed re-publish into a wider space. The content's identity, its CID (content identifier: the fingerprint of its bytes), never changes. What changes is who holds it and who has checked it.

An entry lives in one space, and links between entries don't cross from one space into another. One neighborhood refers to another's content by its CID, the way you might cite a book by its catalog number rather than borrowing the shelf it sits on.

The elohim (the protocol's AI agents, which share its name, tend households and witness what passes between spaces) and the doorways stand outside every single neighborhood. They witness promotions across holons, gather what the spaces say, and project it to the web. The doorway is the porch of the whole street, not a house in it.

## What the Neighborhood Keeps

The neighborhood keeps small signed records, on the order of 500 bytes each: who said what, who people are, the promises households make about what they will keep (custody commitments), attestations (one person vouching for another, or for a record), economic commitments, and reach. It never keeps the bytes. The photos, videos, and pages live in the houses. [The neighborhood is a notary, not a warehouse](../../genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md).

The neighborhood is the most expensive layer to write to, so only what needs a witness no one can forge goes here. Operational chatter stays out. The page a doorway serves is fast because the house is stocked, and trustworthy because the neighborhood checked it.

## What It Costs to Belong

**Every member is a full participant.** Each member runs a Holochain runtime called the conductor, and the conductor runs one cell (an instance of that space's DNA running under the member's own key) for each space the member belongs to. A cell carries its share of that space's memory and gossip. That cost is real, so spaces stay household-sized and collective-sized: never one per document, never one per pair of people. It is the same reason the doorway calls hosting people unscalable by design.

**Patience is part of the price.** A freshly written record takes minutes, not seconds, to spread through the neighborhood. Word travels the way it does on a real street, from neighbor to neighbor.

**Each neighborhood finds its own.** Discovery is per neighborhood: each network has its own way for members to find one another.

## Keeping Faith Over Time

**Changing the rules is a ceremony, not an accident.** Because new rules make a new network, crossing to them means moving records by witnessed re-publish, and nothing is thrown away on the way. This directory keeps a migration toolkit for the crossing. Upgrades that change only how members act, and leave alone what counts as a valid record, can swap in without making a new network at all. The full account is in [how the rules change](../../genesis/docs/content/elohim-protocol/architecture/2026-06-11-dna-upgrade-governance.md).

**Recovery comes from people who know you.** If someone loses their key, a quorum of people who actually know them approves the recovery, and the approvals are recorded as attestations in the neighborhood. It is the testimony of witnesses, not a support ticket.

That is the true commons: the shared ground no single house owns and no single operator can rewrite.

## Down the Street

- [Back to the porch](../../doorway/README.md): how a doorway gives this network a web address and serves its pages to the ordinary web.
- [The house](../elohim-storage/README.md): how a household holds the bytes the neighborhood only notarizes.
- [Holons are spaces](../../genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md): how we use Holochain, in depth.
- [The protocol specification](../../genesis/docs/content/elohim-protocol/protocol-specification.md): the canon this directory implements.

## What's Here

```
elohim/holochain/
  dna/           The rules: elohim (content and learning; the rules,
                 not the agents), imagodei (identity and relationships),
                 mishpat (governance commitments and justice),
                 infrastructure (doorway registrations and node
                 infrastructure), node-registry, hrea (reserved
                 placeholder); lamad-v1 is an earlier set of rules, no
                 longer changed, kept so records written under it can be
                 carried forward
  rna/           Migration toolkit (named for RNA, which carries records
                 from one set of rules to the next)
  tests/         Multi-conductor integration tests
  edgenode/      Container packaging and configuration for a conductor
  elohim-wasm/   WASM utility crate
  local-dev/     Deployed bundles for the local dev stack
  docs/          Technical pointer index
```

Build and test through the repository gate (the project's build-and-test check), `just gate elohim/holochain/dna`. See [`dna/CLAUDE.md`](dna/CLAUDE.md) for the integrity-layer rules (read it before any zome change) and [`docs/README.md`](docs/README.md) for the technical pointer index.
