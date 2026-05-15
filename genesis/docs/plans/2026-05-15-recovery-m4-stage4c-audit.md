# Recovery M4 Stage 4c — Shamir Optionality Enforcement Audit

**Date:** 2026-05-15
**Task:** T23
**Acceptance bar:** zero gating sites — recovery completion path must succeed when no Shamir custody setup exists.

## Method

Grep every site in the recovery-completion path that references Shamir, then classify each as either:

- **Gating** — recovery would fail if Shamir was not satisfied
- **Optional** — only runs when a `governance-action:shamir-custody-setup` manifest is present
- **Informational** — comments, type definitions, or unrelated references

```bash
grep -rln "shamir\|Shamir\|share_assembler\|ShareAssembler" \
  elohim/elohim-storage/src/services/ \
  elohim/holochain/dna/imagodei/zomes/imagodei/src/
```

## Findings

### imagodei coordinator zome (`zomes/imagodei/src/lib.rs`)

| Line | Site | Classification | Notes |
|---|---|---|---|
| 2741 | `// Custodian CIDs are populated by the Task 22 Shamir setup flow when an explicit custody manifest exists; intimate recovery does not pre-designate custodians, so leave empty here.` | Informational | Comment in `create_recovery_request`. The `custodian_cids` metadata field is emitted empty when no setup exists. Recovery proceeds. |
| 3622-3776 | `create_shamir_custody_setup` extern + `ShamirCustodySetupInput` / `ShamirCustodianAssignment` / `ShamirCustodySetupOutput` types | Optional (producer) | Only called by recovery-setup ceremony. Never invoked by the completion path. |

**`commit_key_rotation` (the recovery completion site, line 3502):** zero references to Shamir. The gates are:
- `revocation_floor` — checks for effective `governance-action:key-revocation` entries
- `freeze_floor` — checks for effective `governance-action:identity-freeze` entries

Both are protocol-correctness gates independent of Shamir. **No Shamir gating.**

### elohim-storage services (`elohim-storage/src/services/`)

**Zero hits.** No services reference Shamir at all. The completion path that matters:

- `services/recovery_flow_projector.rs` (T8-T9) — consumes recovery-flow signals (the M3/M4 quorum machine), updates `recovery_flows` state machine. Independent of Shamir.
- `services/elohim_content_dispatcher.rs` (T10) — routes Content signals by prefix. Independent of Shamir.
- All other services (back_prop, epr_service, federation, recovery flow, etc.) do not reference Shamir.

### elohim-storage substrate (`elohim-storage/src/`)

Shamir references exist in:
- `recovery/share_assembler.rs` (T21) — the OPTIONAL reconstruction primitive. Called explicitly by recovery-subject when key material is needed. Never invoked by the completion path.
- `recovery/mod.rs` — module wiring; documentation only.
- `p2p/shamir_transport.rs` (T19/T20) — codec + protocol types. The swarm responder arm only fires on inbound requests; it doesn't gate anything outbound.
- `p2p/behaviour.rs` (T19) — codec registration in the behaviour struct.
- `p2p/mod.rs` (T20) — responder arm; only emits responses on inbound `ShamirShareRequest`. Doesn't affect the completion path.
- `db/recovery_approval_gate.rs` (T20) — checks DHT attestation for *inbound* share requests; doesn't gate the completion path.
- `db/custodian_shares.rs` (T21) — local share store for custodian-side use only.
- `db/models.rs`, `db/mod.rs`, `db/diesel_schema.rs` — Diesel scaffolding for the share store table.

All sites are downstream of the recovery-completion path: the completion succeeds without any of them firing.

## Verdict

**Zero gating sites. The acceptance bar is met without code change.**

Recovery completion can proceed via either:
1. **Social-threshold-only path**: intimate-quorum (M3) or vote-quorum (M4) reaches threshold → recovery becomes effective → `commit_key_rotation` runs the revocation-floor + freeze-floor gates → rotation lands. No Shamir involvement at any step.
2. **Shamir-augmented path**: social threshold + Shamir reconstruction of key material from custodian shares. The reconstruction is invoked explicitly by the recovery-subject; if it fails or if no custody setup exists, the social-threshold path still completes the recovery (but does not recover the key material — the human re-keys instead of reconstructing).

The architecture is correctly layered: Shamir is the OPTIONAL cryptographic proof layer atop the attestation-DHT-driven recovery flow, exactly as the architectural decision in `genesis/docs/superpowers/specs/2026-05-15-dna-signal-as-epr-envelope.md` and the protocol's three-layer truth model describe.

## Test coverage

Add a sweettest at `elohim/holochain/tests/sweettest/src/tests/recovery_m4.rs`:

```rust
#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires packed DNAs from CI; runs in Jenkins pipeline"]
async fn recovery_completes_without_shamir_path() {
    // 1. Setup: human, 3 emergency contacts, NO governance-action:shamir-custody-setup committed.
    // 2. Drive intimate-quorum path → witnesses submitted → quorum reached.
    // 3. Assert: key rotation commits successfully.
    // 4. Assert: no ShamirShareRequest is dispatched anywhere in the flow.
    // 5. Assert: recovery_flows row transitions Open → Quorum → Effective.
}
```

`#[ignore]` follows the established pattern from T14 / T18 sweettests — runs in the Jenkins DNA integration pipeline, not locally, because it requires the packed DNAs.

## No code change needed

T23 acceptance is met as of commit `7d9503c34` (T22 landing). No refactor or feature flag was necessary because the completion path was already Shamir-free by design. The audit confirms the design holds.

Going forward, any new code that introduces a Shamir dependency in the completion path must be flagged and reverted — guidance for code review.
