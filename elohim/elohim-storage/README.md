# elohim-storage

> *"By wisdom a house is built, and through understanding it is established; through knowledge its rooms are filled with rare and beautiful treasures."* — Proverbs 24:3–4

The house behind the door.

If you came in through the [doorway](../../doorway/README.md), the porch that faces the web, you have just stepped inside. elohim-storage is the storage peer a household runs. It keeps the household's photos, videos, pages, and other large content, stocks them on its shelves, and passes them to neighbors over the peer fabric (peer-to-peer connections built on libp2p and iroh, two open networking toolkits). It usually runs on the household's always-on box, and it can also run inside the desktop app on a laptop. Phones and small devices reach it rather than running it. If you arrived some other way, welcome; the porch is one link back.

A house is only as good as what it keeps and how faithfully it keeps it, so this README explains what the storage peer holds, why it goes to so much trouble, and how it keeps its promises to the neighborhood around it. Developers will find the working details at the end.

## Why the House Exists

**The house holds the stuff.** The neighborhood (the distributed hash tables households share, run by Holochain) keeps the shared record: a small signed account of who said what and what each household has promised to keep. It never keeps the bytes ([why bytes never go to the notary](../../genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md)). The bytes live here. Each piece is named by a content identifier (CID): a fingerprint of the bytes themselves, so whoever receives them can check they got exactly what was named. The same bytes have the same name in every house.

**The doorway asks one house.** When a doorway serves a page, it forwards the request to one storage peer and caches the answer. It never goes hunting across households for bytes. If the bytes aren't in the house it asked, that is the house's job to fix, by replication, not the doorway's. The porch stays simple because the house stays stocked.

**Sturdier than a datacenter.** Why go to this trouble when a cloud drive is a click away? Because a family's storage that spans cities, states, and jurisdictions is more resilient than any single datacenter cluster. No single operator, datacenter, or trust root binds it. Grandma's family photo album survives a flood in one city, a power outage in another, and a court order in a third.

## The Pantry

The house keeps its bytes the way a careful household keeps a pantry: some things out on the counter, some within reach, some down in the cellar. The storage peer sorts everything it holds into four temperatures ([the shelves in depth](../../genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md)):

- **Drawn**: a working copy taken out for use. The house makes no promise to keep it, and it is the first to go under pressure.
- **Stocked-warm**: promised, and kept ready in fast memory.
- **Stocked**: promised, and kept on disk.
- **Shelved**: promised, and moved to slow, durable storage, such as an archive or a neighbor's cellar.

The line between drawn and everything else is the line between a copy and a promise.

Bytes move between shelves by a handful of verbs. The house stocks what it has promised and draws what it needs to use. It promotes what is wanted to a warmer shelf, demotes what has gone quiet to a cooler one, and shelves what should last. It evicts only what no promise holds it to keep. When something is lost, it restitutes: it rebuilds what was there.

**Grandma never sees a tier.** None of this is anyone's chore. The household's elohim (the protocol's AI agent that tends it) and its peers handle it, and it stays silent unless an operator opens the dashboard.

**Pieces spread among neighbors.** Large content can be split into N pieces such that any K of them rebuild the whole. This is Reed-Solomon erasure coding, written RS(N,K), and the pieces are spread across households. After a loss, the house gathers K surviving pieces and restitutes the rest.

## How the House Keeps Its Promises

**It mends without being asked.** The storage peer works as a reconciliation controller. The neighborhood's shared record holds what each household has promised to keep, its custody commitments. The storage peer reads those promises, compares them with what it actually holds, and eagerly mends the difference, without waiting for anyone to notice. The promise lives in the neighborhood; the bytes live in the house; the storage peer keeps the two agreeing.

**It takes no neighbor's word.** A new version never becomes current because a neighbor gossiped it, an HTTP call carried it, or an announcement named it. Announcements are doorbells, not deliveries. Verification ends in the house's own copy of the shared record, its own Holochain conductor (the runtime that runs the household's share of the neighborhood), before the house serves anything as current.

**Heal fills, never moves.** When the house heals, it may fill in something missing, but it never overrides what the shared record has declared current. Only the proper declaration channels move what counts as current. Mending a gap is never a way to change the answer.

**It finds neighbors by who they are, not where they are.** Two fabrics carry the house's traffic: libp2p for the everyday exchange between peers, and iroh as the main road for large bytes on the more capable household boxes. Either way, the storage peer finds another household by its peer identity, not by a web address. "Reach my household's box" is not a DNS question.

**It keeps chatter off the record.** Houses tell each other who has what. That inventory gossip is operational chatter, and it is deliberately not signed into the shared record: signing it would cost notary prices for news that goes stale on the first failed fetch. Integrity rests on something sturdier: because every piece is named by the fingerprint of its bytes, every serve can be checked on arrival.

**Fast views, never the truth.** So that pages load quickly, the house keeps fast local read-views, in a local database, of what the neighborhood has recorded. They are projections, never the truth. The truth is the shared record.

**One door for every visitor.** A single HTTP surface serves both the doorway, for browsers coming through the porch, and the desktop app, directly on the same machine. It speaks in ready-to-use shapes, so apps never have to translate.

## What the House Gives Back

Every move the house makes, whether stocking, shelving, serving a neighbor, or rebuilding, leaves an economic event in REA (resource, event, agent: a long-established accounting model the protocol builds on). That makes a household's contribution legible. The space and bandwidth it gives its neighbors shows up on the record and can be fairly shared in [shefa](../../genesis/docs/content/elohim-protocol/shefa.md), the protocol's economic pillar. This is what lets a doorway promise its operator compensation for the compute, infrastructure, and effort it takes. A household that keeps a neighbor's cellar is doing real work, and the record says so.

## What the House Is Not

The house is not the source of truth; that is the neighborhood. It is not where signatures and promises live; those are kept in the shared record, and the house reads them. And it is not a doorway; the porch belongs to the doorway, which comes to the house for what it serves.

The house holds the stuff. The neighborhood keeps it honest.

## Down the Street

A page is fast because the house is stocked, and trustworthy because the neighborhood checked it. To keep walking:

- [Back to the porch](../../doorway/README.md): how a doorway gives this house a web address and serves what it holds to the ordinary web.
- [The neighborhood](../holochain/README.md): how households that share rules decide together what is true, who said it, and what each has promised to keep.

To go deeper:

- [The shelves](../../genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md): the pantry temperatures, custody commitments, and how pieces spread across households.
- [The rules the house keeps with its neighbors](../../genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md): the operator's runbook for what the house guarantees and how to tell when it slips.

## What's Here

```
elohim-storage/
  src/           The storage peer: HTTP surface, pantry and blob store,
                 the libp2p and iroh fabrics, the reconcile controller,
                 projections
  migrations/    Local database schema
  CLAUDE.md      Developer guidance
  tests/         Tests
  docs/
    EPR_REST_API.md   HTTP reference
```

elohim-storage is a Rust crate. With the Rust toolchain and the `just` task runner installed, build and test it through the repository gate (the project's build-and-test check), `just gate elohim-storage`, run from the repository root. See [`CLAUDE.md`](CLAUDE.md) in this directory for developer guidance and [`docs/EPR_REST_API.md`](docs/EPR_REST_API.md) for the HTTP reference.
