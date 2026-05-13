---
name: Storage RS-distribution = quilt; weave is Moss; lattice is governance
description: Vocabulary register decision for reed-solomon storage distribution; preserves three reserved words against accidental reuse
type: project
originSessionId: a03c2b82-4940-42f2-847d-d6d05c45509e
---
Storage RS(N,K) distribution of a content unit across N shards (any K reconstruct) is named **`quilt`**. Verb pairings: `quilt content into N shards`, `the quilt for content X`, `re-quilt` (restitch after losses), `RS(N,K) quilt`. Pantries hold quilts; you stock pantries with quilts; you draw a quilt then reconstruct from K shards.

**Reserved words — do not reuse for new concepts:**
- `weave` belongs to Holochain Moss (`@theweave/api`, "Weave Tool", `weave.service.ts`). RNO sub-project #8 (lamad-as-Moss-Weave-Tool, High priority on cross-wave guidance) makes the collision identifier-space, not just prose.
- `lattice` belongs to cross-collective governance ("the holonic lattice" — `genesis/plans/2026-04-10-collectives-schema-design.md`).
- `quilt` is the storage RS-distribution term; do not let it bleed into governance/Moss contexts.

**Why:** The vocabulary cleanup sprint (`genesis/docs/superpowers/plans/2026-04-30-vocabulary-cleanup-sprint-kickoff.md`, resolved 2026-04-30) chose words deliberately so the substrate-replication and doorway-cache agents work from one dictionary. Quilt fits RS reconstruction better than weave (N patches survive losing some; threads in a weave don't), fits the domestic register paired with `pantry`/`stock`/`draw`, and reinforces the household-as-resilience-unit framing.

**How to apply:** When designing or naming anything storage/replication adjacent (signals, events, configmap keys, admin endpoints, tracing spans, Rust types we're inventing fresh), use `quilt`/`pantry`/`stock`/`draw`. When seeing `weave` or `lattice` in elohim code, those are the *other* meanings; don't conflate. Wire-level terms (HTTP `/blob/<hash>`, `sha256-{hex}`, internal Rust `BlobStore`) keep their existing names — boundary rule from Task 2.
