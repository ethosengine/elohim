# Backlog: CID identity-namespace enforcement — rollout (audit + remediation)

**Status:** in-progress (rungs 1-2 foundation landed) · **Captured:** 2026-06-16 (operator directive: "start enforcement of CID, so the cleanup shakes out") · **Class:** identity coherence · **Owner-next:** reviewed wiring + rung-3 fix

## Convention being enforced
`agent_cid` (`uhCAk…`) is the canonical identity join key (`elohim/elohim-storage/CLAUDE.md` → "Identity & Transport-Identity Coherence"). Three namespaces must never be raw-string-joined: agent_cid (`uhCAk`), libp2p (`12D3Koo`), iroh NodeId (64-hex). Cross-namespace equality silently empties joins → the all-zeros resilience card.

## Audit — write paths to `agent_cid` join-key columns
| Column | Joins treat as | Live write namespace | Verdict |
|--------|----------------|----------------------|---------|
| `humans.agent_pub_key` | agent_cid | **NULL** (seeder sends null; only `household_id` backfilled) | design-gap — heal needed (rung 3) |
| `rea_commitments.provider` | agent_cid | **libp2p `12D3Koo`** via provide-loop `provider = self_cid` | **VIOLATION** — primary live drift |
| `shard_locations.peer_id` | agent_cid (⚠ misnamed) | agent_cid (seeder path) | OK per CLAUDE.md / RCA |
| `peer_statuses.peer_id` | agent_cid | agent_cid | OK |
| `peer_identity_bindings.peer_id` | libp2p (NOT a join key) | libp2p `12D3Koo` | correct — NOT instrumented |

**Primary live violation site:** `elohim/elohim-storage/src/services/conductor_commitment_author.rs` — `build_content_payload` (`:122` `"provider": self_cid`) and `build_provide_announce_input` (`:158` `provider: self_cid.to_string()`). Both pure builders; `self_cid` is a libp2p transport id at runtime. Wire observation at their **caller** (keep the builders pure) — grep callers of `build_provide_announce_input` / `build_content_payload`.

## Enforcement ladder — status
- **Rungs 1-2 (LANDED, commit `3d026f226`, NOT pushed):** `elohim/elohim-storage/src/identity_namespace.rs` — pure classifier (`is_agent_cid` / `classify`) + `observe_agent_cid_write(column, value)` → WARN + `elohim_identity_namespace_violation_total{column,expected,got}` counter. Never rejects. 7 unit tests green. Registered in `lib.rs`. Currently **unwired** (no behavior change yet).
- **Rung 2b (NEXT — reviewed):** wire `observe_agent_cid_write` at the provide-loop caller (the live violation) + optionally the other 3 columns' write sites. Observation-only, safe. Build + clippy + push → on deploy the counter "shakes out" the drift in prod metrics/logs. *This is the step that realizes "so the cleanup shakes out."*
- **Rung 3 (the actual fix — plan):**
  1. **Populate `humans.agent_pub_key`** with `agent_cid` from the DHT human projection — extend `reconcile/controller.rs` `on_membership_projected` to stamp it (the heal route from commit `e05d9f10` does NULL-only; build on it). This lights the resilience card (mission criterion #4).
  2. **Fix the provide-loop** to write `agent_cid` (not `self_cid`) as `rea_commitments.provider` — resolve the node's own agent_cid at the caller of `conductor_commitment_author` builders.

## Caveat (do not violate)
The transport-id→agent_cid RESOLVER (`2026-06-15-coherent-transport-identity-resolver-design.md`) is BLOCKED + unsafe (no signed `AgentPeerBinding`; self-asserted/unsigned — open security item). Do NOT build enforcement on resolution; fix by populating agent_cid from DHT truth.

## Links
- Module: `elohim/elohim-storage/src/identity_namespace.rs` (commit `3d026f226`)
- Convention: `elohim/elohim-storage/CLAUDE.md` → "Identity & Transport-Identity Coherence"
- Root RCA: `genesis/docs/content/elohim-protocol/history/2026-06-15-matthew-edge-resiliency-rca-fanout-synthesis.md` §6
- Resilience card join: `elohim/elohim-storage/src/services/household_resilience.rs` (lines 74, 172–174, 447–449)
