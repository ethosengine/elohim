---
name: Quilt/pantry storage vocabulary — register at genesis/graphos/vocabulary.md
description: Storage and distribution vocabulary — `quilt` (RS-encoded content unit), `pantry` (peer-tended container), `stock`/`draw` (verbs), `shard`, `RS(N,K)`. Wire-level identifiers keep legacy names. Decided 2026-04-30.
type: project
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
---
The protocol's storage and distribution vocabulary is registered in `genesis/graphos/vocabulary.md`. Use it for design discussion, signal/event names, narrative, and any new identifier we invent.

| Term | Meaning |
|---|---|
| `quilt` | RS-encoded distribution of a content unit across N shards, any K of which reconstruct |
| `pantry` | Peer-tended container that holds shards on behalf of the household |
| `stock` (verb) | Deposit content into a pantry (replaces `upload` where new) |
| `draw` (verb) | Retrieve content from a pantry (replaces `download` where new) |
| `shard` | One piece of an RS-encoded quilt, addressed by `sha256-{hex}` |
| `RS(N,K)` | Contract policy: N total shards, K reconstruct; archetype-tunable |

**Boundary rule:** Wire-level (HTTP routes, file paths, Rust struct names like `BlobStore`, `/blob/{hash}`, `sha256-{hex}`, CID) keeps existing terminology because it's externally legible. New vocabulary applies one layer up.

**Why `quilt` over `weave`:** Moss `@theweave/api` / Weave Tool / `weave.service.ts` collision is concrete (RNO sub-project #8 is High priority, lamad-as-Moss-applet packaging). Sharing `weave` would collide in identifier space. `quilt` also fits the metaphor better — a quilt of N patches naturally survives losing some, matching RS(N,K) reconstruction; threads in a weave do not. Documented at `genesis/docs/superpowers/plans/2026-04-30-vocabulary-cleanup-sprint-kickoff.md`.

**Why `pantry` over `bucket`/`store`:** stewardship vocabulary, not ownership vocabulary. A pantry is something a household tends and shares from, not a destination you PUT to. Reinforces the "no sovereignty, stewardship instead" framing.

**How to apply:**
- When naming new signals, events, configmap keys, admin endpoints, tracing spans, narrative concepts: use the new register.
- When touching wire-level identifiers (HTTP routes, Rust struct names, file paths): leave them alone unless the user asks. They're externally legible and stable.
- The legacy `/store/<hash>` and `/api/blob/<hash>` HTTP paths were retired 2026-04-30 — canonical path is `/blob/<hash>` (registry-routed via storage manifest).
- If a future agent reaches for `weave` in elohim-storage / replication context, redirect to `quilt`. `weave` is reserved for Moss integration concepts.
