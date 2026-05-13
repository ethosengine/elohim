---
name: HDI integrity-zome validators cannot use get_links
description: Holochain validation callbacks can only access deterministic primitives (must_get_valid_record, must_get_entry, must_get_action, must_get_agent_activity). Link traversal (get_links) and non-deterministic reads are HDK-only (coordinator). Cross-entity enforcement rules that require link state must live in coordinator pre-commit gates, not validators.
type: project
originSessionId: 5ba0c4a3-96ec-40af-913d-cb7ebf8d7a3c
---
Holochain separates validation (HDI — runs deterministically on every validating peer) from coordination (HDK — runs in the authoring client only). Validators can only call deterministic primitives:

- `must_get_valid_record(ActionHash)` — resolves a specific record
- `must_get_entry(EntryHash)` — resolves a specific entry
- `must_get_action(ActionHash)` — resolves a specific action
- `must_get_agent_activity(AgentPubKey, ChainFilter)` — enumerates one agent's source chain
- `hash_entry(...)` — hashes an entry for anchor derivation

**`get_links` is NOT available in validators.** Link state is non-deterministic (links can be added/removed after entries they reference). Any validation rule that depends on "does link X currently exist?" is architecturally incorrect.

**Consequence for enforcement patterns:**

- **Entry→entry references (hash pointers):** fine to validate. `must_get_valid_record` resolves the referenced entry deterministically.
- **Cross-entity state queries ("is there an active X?"):** must be enforced at the coordinator level via pre-commit guards (coordinator uses `get_links`, bails before `create_entry` if state blocks the operation).
- **Post-hoc integrity:** if a bad entry slips past coordinator gating, the network's observer peers can author revocation/challenge entries. The validator is not the only check — it's the deterministic floor.

**How to apply:**
- When designing validators, ask: "does this rule need non-deterministic data to evaluate?" If yes, move the enforcement to the coordinator.
- `must_get_agent_activity` is the one chain-scoped query available — useful for "has this agent authored X?" but only when you know which agent to inspect.
- Document the enforcement split in specs: "validator checks X (deterministic); coordinator pre-check guards Y (non-deterministic)." Avoid the common mistake of treating the validator as the universal gate.
- Recovery Protocol Phase 2 M2 hit this: freeze-floor check cannot traverse ActiveFreezes links in the validator; enforcement moves to M5 coordinator pre-commit gate. Pure-logic rules helper is fully testable and shared between validator (when link state is provided) and coordinator.
