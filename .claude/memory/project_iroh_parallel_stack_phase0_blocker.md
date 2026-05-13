---
name: iroh parallel stack — pinned iroh-blobs 0.94 + iroh 0.92 (compile-tested)
description: Three pin shifts walked from iroh 1.0-rc.0 standalone → iroh-blobs 0.98 → iroh-blobs 0.94. The 0.94 pin is the highest version with stable ed25519-dalek 2.2 + curve25519-dalek 4.1; 0.95+ pulls a broken pre-release crypto path. No holochain bump needed.
type: project
originSessionId: c1e4ec6a-3fe5-4cee-a4ae-b04e6b47ea55
---
iroh parallel-stack plan (`genesis/docs/superpowers/plans/2026-05-07-iroh-parallel-stack.md`) — three pin iterations on 2026-05-07 before landing.

**Final pin (committed 2bea677c):** `iroh = "=0.92"` + `iroh-blobs = "=0.94"`. iroh-gossip deferred to Phase 4 (its 0.94 inclusion conflicted with multihash-codetable's crypto-common at a different major).

**Pin walk:**

| Iter | Pin | Resolves? | Compiles? | Why rejected |
|---|---|---|---|---|
| 1 | iroh 1.0.0-rc.0 standalone | n/a | n/a | Pivoted away from iroh-blobs entirely; over-narrow probe |
| 2 | iroh-blobs 0.98 + iroh 0.96 + iroh-gossip 0.96 | ✅ | ❌ | iroh-base 0.96 → ed25519-dalek 3.0.0-pre.1 → curve25519-dalek 5.0.0-pre.1 has stale `digest::crypto_common::BlockSizeUser` import; published-source bug |
| **3** | **iroh-blobs 0.94 + iroh 0.92** | ✅ | ✅ | 9m 16s clean fresh compile; stable ed25519-dalek 2.2 + curve25519-dalek 4.1 |

**Soak boundary:** iroh-blobs 0.95+ moved to pre-release crypto path. iroh-blobs 0.94 (Sep 2025) is the highest version using stable crypto. 0.93/0.91/0.90 also viable as fallbacks.

**Why holochain bump no longer needed:** With iroh 0.92 (not 1.0-rc.0), the serde 1.0.228 requirement that originally forced the holochain bump goes away. holochain_client 0.9.0-dev.5 + holochain_types 0.7.0-dev.5 stay where they are.

**Cargo.lock gotcha:** During iter 3, ran `cargo update -p iroh-blobs` — it transitively bumped holochain_types from dev.5 to dev.22, which broke kitsune2_api version coherence (dev.22 uses kitsune2_api 0.5; holochain_client 0.9.0-dev.5 expects 0.4). Reverted Cargo.lock with `git checkout` and re-built; cargo's resolver added iroh deps without disturbing holochain stack.

**How to apply:**
- For Phase 1+2 work, this pin is locked. Do NOT pick a different iroh-blobs version if `just build-iroh` later fails — surface BLOCKED and probe inline (per `feedback_subagent_dep_conflict_supervision.md`).
- API drift later: when n0 ships iroh-blobs aligned with iroh 1.0 stable + non-broken crypto, version-bump. The Phase 2 `IrohBlobStore` wrapper is the chokepoint for absorbing API drift.
- iroh-gossip will get its own version probe at Phase 4 pickup.
