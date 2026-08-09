---
title: "Bound the lineage-DAG walk — sedimentree strata over the declared-head primitive"
id: version-dag-strata-compaction-handoff
status: Draft
class: design-handoff
domain: D2
sprint: unassigned
cites:
  - version-dag-lives-at-l2-not-in-crdt-doc | The record that settled WHERE the version DAG lives (L2, the DHT) and named the four running instances this handoff proposes to compact | path: genesis/docs/content/elohim-protocol/history/2026-08-07-version-dag-lives-at-l2-not-in-the-crdt-doc.md
  - lens-version-dag-epr-policy-dependency-design | The spec that SEALED the declared-head-over-lineage-DAG primitive at L2 — the substrate a strata layer would sit under | path: genesis/docs/superpowers/specs/2026-06-27-lens-version-dag-epr-policy-dependency-design.md
  - identity-head-key-lineage | The spec that GENERALIZED the primitive across four instances and named the compose-don't-build rule this handoff applies | path: genesis/docs/superpowers/specs/2026-07-17-identity-head-key-lineage-design.md
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
---

# Handoff: bound the lineage-DAG walk with levelled strata

> **START HERE, then STOP.** This is a handoff to a **design** step, not an implementation slice.
> It ends at a p2p-design-gated brainstorm → spec. Do not write zome code from this document.

**The ask in one line:** our declared-head-over-lineage-DAG primitive walks its DAG one hop at a
time with no depth bound, in every place it is implemented. Sedimentree's levelled strata are a
real answer, and the four instances share one substrate — so the fix is built once and serves all
four.

---

## Read this correction first — it will save you an hour

The concern was originally described (by me, in the session that filed borrows row 9) as *"the L2
version-lineage DAG, `Mishpat::Commitment` + `version_parent`."* Two things about that phrasing
will send you to the wrong file:

1. **`Commitment` has no `version_parent` field.** It is a generic envelope —
   `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:275`:
   ```rust
   pub struct Commitment { pub action: String, pub payload_json: String, pub signed_at: String }
   ```
   `version_parent` lives *inside* `payload_json`, which is why the projector reaches for it with
   `payload.get("version_parent")` at `elohim/elohim-storage/src/mishpat_projection.rs:346`.
   **This turns out to matter a great deal — see the hypothesis below.**

2. **The only instance with a *live DHT walk* is key rotation, not the content lens.** And that
   walk deliberately does **not** use a `version_parent` field at all: the 07-17 B0 decision reads
   the existing `KeyRotation{superseded → new}` edges *as* the version DAG. Go to
   `elohim/holochain/dna/imagodei/zomes/imagodei/src/identity_lineage.rs`, not to mishpat.

---

## Grounded state of the four instances (verified against the tree, 2026-08-07)

| # | Instance | Where the walk is | Bound? |
|---|---|---|---|
| 1 | **Content lens / policy** | L3 only — `VERSION_CHAIN` datalog rule, `elohim/elohim-storage/src/graph/primitives.rs:49` | **No.** Unbounded recursion |
| 2 | **Provenance** (epr-meta lineage edges) | no walk implemented | n/a |
| 3 | **REA collective** (`Collective` + `Membership{role:Steward}`) | no walk implemented | n/a |
| 4 | **Human key rotation** | L2/DHT — `identity_lineage.rs` | **No.** Unbounded loop |

### Instance 4 — the sharpest one (`identity_lineage.rs`)

`identity_chain_root` (`:143`) is a `loop` that per hop does a `get_links` **plus** a `get` per
returned link (`resolve_version_parents`, `:112`), walking back until `version_parent = []`. There
is **no depth cap** — only a cycle guard, and that guard is `visited.contains(&current)` on a
`Vec`, i.e. O(n²) on its own. So the cost is *unbounded DHT round trips, linear in chain length*.

`chain_head_of` (`:89`) finds the tip with a nested scan —
`edges.iter().filter(|e| !edges.iter().any(...))` — **O(E²)**.

### Instance 1 — unbounded too, but failing differently

`VERSION_CHAIN` is a recursive rule with no `$max_hops` parameter (contrast `NEIGHBORS` in the same
file, which takes one). `atom_version_chain::build`
(`elohim/elohim-storage/src/graph_views/lamad/atom_version_chain.rs:37`) materialises the **entire**
chain into a `Vec` and then takes `canonical_cid = chain.last()` — walking everything to find the
head. Three latent defects worth noting while you are in there, though none is this work:

1. **`chain.last()` is not the newest version — it is the lexicographically-largest CID.** Cozo
   sorts result-relation rows by value, not traversal order, so the rows of the recursive
   `VERSION_CHAIN` rule come back CID-sorted. `canonical_cid = chain.last()` therefore returns a
   *wrong head on any chain with ≥2 successors, even a linear one*, and the `version: (idx + 2)`
   numbering is wrong for the same reason. Silent today only because no atom has been versioned
   twice yet. This one is a correctness bug, not a performance shape — it was pulled out of this
   handoff and fixed directly (hop-counted rule variant; ordering by depth, not row order).
2. The numbering additionally assumes a **linear** chain and remains ambiguous on a fork
   (deterministic after the hop fix, but fork semantics are still an open design question).
3. `superseded_at` is hardcoded `None`.

**Do not be misled by `max_depth` in `graph_engine.rs`** (default 2, hard-capped at 3 at `:207`).
That bounds the *relationship* traversal, a different query — and it bounds by **truncation**,
which is the wrong medicine here: a truncated version chain returns a *wrong head*, not a slow one.

---

## What sedimentree actually contributes

Two mechanisms from the design (`https://github.com/inkandswitch/keyhive/blob/main/design/sedimentree.md`):

- **Levelled strata + a support relation.** A stratum compacts a commit range; stratum *x* supports
  stratum *y* when it contains all of *y*'s commits, checked by looking for *y*'s start/end hashes
  among *x*'s `(start, end, checkpoints)`. A walk can then skip a whole compacted range in one step
  instead of hop-by-hop.
- **Boundary selection by hash trailing-zeros.** Read a hash as a numeral in base *b*, count
  trailing zeros; *n* zeros marks a level-*n* boundary, ~1 boundary per *bⁿ*. The point is that
  **the boundary is a property of the content**, so peers who never negotiate still agree on where
  strata begin and end. That is what makes it viable on a DHT where there is no coordinator to ask.

**We are borrowing the technique, not the crate.** Beelay is self-described pre-alpha, unaudited,
"DO NOT use this release in production applications." Nothing gets linked.

---

## The compose-don't-build hypothesis (for the gate to test, NOT a decision)

Because `Commitment` is a **generic envelope** (`{action, payload_json, signed_at}`), a stratum may
be expressible as a `Commitment` with `action: "version-stratum"` and its boundary/checkpoint set in
`payload_json` — needing **no new entry type, no integrity-zome change, and therefore no DNA-hash
change**. That would make this a coordinator-only addition, exactly the shape the 07-17 B0 decision
took for identity lineage.

If that holds it is the difference between a network event and a hot-swap
(`ALLOW_COORDINATOR_UPDATE` / `sync_coordinators`), which is most of the cost of this work.

**Treat this as the first question the gate must answer, not as a settled design.** It has not been
validated. The entry-type budget question still applies if it turns out false — check headroom
(mishpat is the small DNA; the link-type cap is the tighter wall at 225/256 in `content_store`).

---

## Step 0 is MEASURE. Do not skip it.

**There is no measurement behind this concern.** I found no benchmark, no depth histogram, and no
production complaint. Key rotation is rare, so instance 4's chains are short *today*; instance 1's
could grow faster but nobody has counted.

Building a compaction layer for an unmeasured bottleneck is exactly the "instrument with no reader"
failure the repo gospel warns about. So:

1. Instrument or query actual chain depth for instances 1 and 4 (a sweettest can drive instance 4's
   rotation chain; instance 1 can be counted from the `lenses` projection).
2. Get the DHT round-trip cost per hop for `identity_chain_root`.
3. **If depths are trivial and flat, the honest outcome is to write that down, note the shape is
   understood, and close the row.** That is a success, not a failure — and it is a cheaper success
   than a spec nobody needed.

Proceed to design only if the numbers (or a credible growth curve) justify it.

---

## The gate

`p2p-design-gate` is **MANDATORY** here — this touches DHT entry types and their read paths. Invoke
the skill before proposing approaches. Its four questions, pre-loaded with what is already known:

1. **Notarization class?** Strata describe already-notarized lineage. Are they class A (notarized),
   or a class-C derived cache that any peer can recompute? *This is the hinge of the whole design.*
2. **Does an entry type already exist?** See the hypothesis above — `Commitment` may already be it.
3. **Identity content-derived?** A stratum's identity should be derivable from its boundary hashes.
   Note the existing trap: `Commitment` cid is `entry_hash`, and `action_hash` is only
   `dht_anchor_hash` — returning the wrong one silently breaks bounds-gates.
4. **Which coordinator function creates it, which signal projects it?**

---

## Definition of done for this handoff

A spec in `genesis/docs/superpowers/specs/` that: carries the measurement from step 0; answers all
four gate questions; states whether the Commitment-envelope hypothesis holds; and — because the
whole argument for doing this is that the instances share a substrate — shows **one** strata design
serving all four, or explains honestly why it cannot.

Then update borrows row 9 leg (a) with a forward cite to that spec.

## What NOT to do

- **Do not adopt sedimentree's sync protocol.** It carries no notarization, no authority, no head
  election; its trust model is availability from an untrusted relay where L2's is validation by a
  notary. That question is settled — see the cited history record, and the "below the line" note in
  the borrows cluster.
- **Do not bound the walk by truncating it.** A truncated lineage walk returns a wrong head. That is
  worse than a slow correct one.
- **Do not build a fifth instance of the primitive.** Compose with the four that exist — that is the
  named lesson of the 07-17 spec, and the uncomposed-fork failure the 08-07 record was written about.
