# Hypercore / Holepunch — Prior Art for the P2P Data Plane

> The p2p-dataplane sprint asks: a generalized P2P data plane beside the Holochain DHT, with
> "replication follows relationship" and NAT traversal that scales. Hypercore / Holepunch is the
> most mature JS-ecosystem answer to that exact shape. This note captures it as prior art for the
> networking + data-plane brainstorming — *not* the truth layer.
>
> Full cross-cutting survey (incl. the doorway/ActivityPub thread):
> [`genesis/research/hypha-distributed-press-cross-pollination-2026-06-23.md`](../../../genesis/research/hypha-distributed-press-cross-pollination-2026-06-23.md).
> The sprint that frames the need: [`.claude/prompts/p2p-dataplane-sprint.md`](../../../.claude/prompts/p2p-dataplane-sprint.md).

---

## The prior art

A [**Hypercore**](https://github.com/holepunchto/hypercore) is *a secure, distributed append-only
log*: **single-writer** (one keypair owns write access), integrity-secured by a **signed BLAKE2b
merkle tree**, replicated **sparsely** — peers *"only download the data they are interested in."* On
top of the single core sit the rest of the stack:

- **Hyperbee** — B-tree key/value store (and a basis for queries).
- **Hyperdrive** — a filesystem = a metadata core + a content core.
- **Corestore** — manages many cores + their replication.
- **Autobase** — linearizes *multiple* single-writer cores into a multi-writer structure (no global
  consensus bottleneck).
- **Hyperswarm** over **[HyperDHT](https://github.com/holepunchto/hyperdht)** — peer discovery by a
  *topic / `discoveryKey`* (derived from the core key, so peers find each other *without leaking the
  key*), with **holepunching as a first-class feature**: UDP holepunching across NATs/firewalls into
  Noise-encrypted streams over UTP.

The end state is [**Pear**](https://docs.pears.com/): ship fully P2P desktop/mobile apps with *no
servers*. (Org note: `hypercore-protocol` is now legacy — `hypercore-next` / `corestore-next` are
archived; active work is in [`holepunchto`](https://github.com/holepunchto); Hypercore 11 (current,
RocksDB-backed for storage + atomicity) supersedes the v10 LTS line.)

## The instructive resonance — and the hard line

**A Hypercore is structurally a Holochain source chain**: single-writer, append-only, hash-linked,
signed by the owning key. The instructive part is the divergence:

| | Hypercore | Holochain |
|-|-----------|-----------|
| Trust model | *Trust the writer's key* — no shared validation | *Validation intrinsic to data type* (DNA rules) |
| DHT role | Discovery / holepunching only | Graph **validating** DHT |
| Guarantees | Integrity + availability | Integrity + availability + **validity** |
| Queries | Via Hyperbee on top | DHT links + zome queries |

So Hypercore is **integrity, not validity**. It cannot be the protocol's truth layer — it is a
candidate **data-plane / blob-transport** substrate sitting *beside* iroh/libp2p, never a
replacement for the Holochain validating DHT. This is exactly the layering the
[k8s-is-not-the-architecture](../../../.claude/memory/feedback_k8s_is_not_the_architecture.md)
discipline and the p2p-dataplane sprint already insist on: trust stays in Holochain; bytes move on a
generalized data plane.

## What's worth learning for the data plane

1. **Sparse replication = "replication follows relationship."** *"Download only the blocks you
   need"* is the off-the-shelf shape of the sprint's hardest requirement ("not everyone has
   everything") and a direct comparator for our byte-mobility work — the tiered-quilt data-plane
   truth (`genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md`)
   and blob-custody reconciliation (`genesis/docs/superpowers/specs/2026-05-02-blob-custody-reconciliation-design.md`).
2. **Autobase is prior art for multi-writer convergence** — the "how do N agents converge a shared
   structure without global consensus" problem we currently answer with Automerge. Read it for the
   linearization approach.
3. **HyperDHT is one of the most mature holepunching-first DHTs in the JS ecosystem** (UDP holepunching
   is a first-class feature, deployed at scale via Holepunch/Keet). This is
   the live WAN-NAT gap's neighborhood (p2p layer — iroh/libp2p/tx5, *not* doorway/federation).
   Relevant to the tx5/go-pion conductor-leak lineage and the
   [WAN-NAT p2p-vs-federation gap](../../../genesis/data/timeline/backlog/agent-peer-binding-cross-signed-proof.md)
   framing. (Federation-layer sibling note: [`doorway/research/activitypub-federation-prior-art.md`](../../../doorway/research/activitypub-federation-prior-art.md).)
4. **Discovery-by-topic rhymes with affinity-weighted peer topology.** A `discoveryKey` per
   affinity-topic is one concrete shape for "steward topology emerges from content affinity rather
   than being centrally planned" — the open question in the README's Networking Problem.

## Cautions

- **Written in JS/Node** (Holepunch/Pear/Bare runtime). Adopting any module means a Rust port or a
  bridge — same caveat the p2p-dataplane sprint already flagged for Pinecone. Treat as *design prior
  art* first; integration is a separate, costed decision.
- **Don't conflate the layers.** Hypercore's append-only log is tempting to read as a truth
  primitive. It is not — it has no shared validation. Keep it in the data plane.
- **iroh is our existing bet** for the substrate transport. Read Hypercore for *what good sparse
  replication + holepunching looks like*, then ask whether iroh/libp2p already gives us the same
  affordances before importing anything.
