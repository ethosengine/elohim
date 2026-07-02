---
name: versioned-entity-head-is-declared-dependency
title: Versioned-entity HEAD is a declared dependency
description: "Which version applies is a DECLARED dependency (cid-pin=lockfile), not recency; versions are a DAG (fork/revert/merge); binding picks head, not the query layer."
metadata: 
  node_type: memory
  type: project
  originSessionId: 6e06fa79-220c-45d9-82d7-bb465649dc34
---

Architect's design direction (2026-06-27, from the plural-Mishpat lens-market code review, PR4 disposition):

**When an entity is versioned via a content-addressed back-pointer (e.g. `version_parent`), do NOT resolve
"which version applies" by recency in the read path.** Two principles:

1. **Versions are a DAG, not a linear chain.** fork (branch) · revert (move head back to an earlier cid) ·
   merge (≥2 parents). `cid == entry_hash` (immutable, content-addressed) is what makes the DAG sound — every
   version is a node, edges are back-pointers, nothing is mutated. Nothing is ever deleted; an older version
   stays canonical and fetchable by `cid`.

2. **The HEAD that applies to a consumer is a DECLARED dependency**, the `package.json` model: the consumer
   (the EPR, here) declares *"I depend on policy P"* with a constraint — a range/branch-pointer, or a **pinned
   exact `cid` which IS a lockfile** (reproducible, content-addressed). A resolver picks the effective head
   per the declaration. **The binding decides the head; the infrastructure must not auto-pick "newest."**

**Why:** a read-time "newest-in-chain hides older" filter (or even linear chain-aggregation of standing)
bakes in a head-selection policy the protocol should leave to the declaration — premature, and ripped out
when the binding lands. The "duplicate versions both surface" that a reviewer calls a bug is really "the
binding-declaration layer doesn't exist yet"; surfacing all heads is the honest default until it does.

**How to apply:** when designing ANY "which version applies" surface (lenses, content, policies, manifests),
route it through `p2p-design-gate` and ask: is the head-selection a *declared content-addressed dependency*
(range or cid-pin lockfile) rather than a recency inference? Don't auto-resolve in the query layer. Plurality
(distinct authors co-surfacing) is orthogonal to versioning (one author's own DAG) — never conflate them.

Seed doc (full spec-to-be): `genesis/data/timeline/backlog/lens-version-dag-policy-dependency.md`.
Sibling: [[project_mishpat_commitment_cid_is_entry_hash]] (cid=entry_hash is the DAG-node identity);
[[feedback-backend-authoritative-frontend-senses]] (the binding/declaration is backend-authoritative).
