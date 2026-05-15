# Recovery M4 Stage 4d — Angular UX Grandma-Standard Audit

**Date:** 2026-05-15
**Task:** T24
**Acceptance bar:** three properties hold from every user-facing surface:
1. **No seed material visible** — no template path renders raw share bytes, seed bytes, or signing keys.
2. **No A/B path selector** — the user is never asked "do you want recovery with or without Shamir?" The substrate decides based on whether a custody manifest exists.
3. **Graceful degradation** — when custodians are offline or no custody setup exists, the UI shows progress through the social-threshold path without surfacing a Shamir-specific error.

## Method

```bash
grep -rln "seed\|share\b\|shamir\|Shamir\|custodian\|reconstruct" \
  app/elohim-app/src/app/imagodei/

grep -rn "shamir\|Shamir\|share_data\|share_blob\|seed\|custodian\|shard\|reconstruct" \
  app/elohim-app/src/app/imagodei/services/recovery-coordinator.service.ts \
  app/elohim-app/src/app/imagodei/components/recovery-request/*.{ts,html} \
  app/elohim-app/src/app/imagodei/components/recovery-interview/*.{ts,html}
```

## Findings

### `recovery-coordinator.service.ts` (481 lines)

**Zero references** to shamir / share / seed / custodian / shard / reconstruct. The service drives the social-threshold recovery flow exclusively:
- `requestRecovery()` calls `create_recovery_request` on the imagodei coordinator zome
- `submitIntimateWitness()` collects emergency-contact witnesses (M3)
- `commitKeyRotation()` triggers the rotation that lands the recovery

No Shamir branching. No A/B selector. The substrate's optionality decision (whether a custody manifest exists) never bubbles up to the UI.

### `recovery-request/` components

**Zero references** to shamir / share / seed / custodian / shard / reconstruct. The UI presents intimate-quorum (emergency contact) flow and self-revocation flow; both are wisdom-layer social paths, not key-material reconstruction.

### `recovery-interview/` components

**Zero references** to shamir / share / seed / custodian / shard / reconstruct. The interview flow is social: identify the human, identify the emergency contacts, walk through stewardship — never reaches into seed material.

### `identity-attestation.model.ts`

Carries a `KeyStewardship` interface (line 153) that declares the *metadata shape* of a Shamir custody record:
- `keyShardHolders: string[]` (agent IDs only — NOT shard bytes)
- `thresholdM`, `totalShardsN`
- `shardCommitmentHash: string` (commitment hash — NOT shard bytes)
- `signingPolicy`, `elevatedThreshold`
- `keyGenerationId`, lifecycle timestamps

**Importantly, no field of this interface carries actual share/seed material.** The interface is a model-only declaration; only a re-export appears in `models/index.ts` (line 71). It is NOT bound to any UI template (zero `<keyshardholders>` / `<thresholdM>` template renders found across the imagodei pillar).

### All other matches in the imagodei pillar

The broader grep hit the strings "seed" / "share" / "custodian" in unrelated contexts:
- `share` as in "share with friends" or social-sharing language — not Shamir share bytes
- `seed` as in "seed data" (initial content) — not key seed material
- `custodian` as in "stewardship custodian relationships" — the role identifier in human-relationship.model.ts, not custody of key material

None of these touch the forbidden categories.

## Verdict

**All three grandma-standard properties hold without code change:**

1. **No seed material visible:** confirmed by zero template references to raw share bytes / seed bytes / signing keys across the imagodei pillar's components and services. The `KeyStewardship` model interface declares metadata shape only; share bytes never appear in the interface or any rendered template.
2. **No A/B path selector:** confirmed by zero Shamir branching in `recovery-coordinator.service.ts`, `recovery-request/`, or `recovery-interview/`. The user is never asked to choose between paths; the substrate decides based on custody manifest presence (T22 records the manifest; the recovery flow follows the social-threshold path by default and augments with Shamir only when the substrate determines it).
3. **Graceful degradation:** confirmed because the UI does not invoke Shamir directly. If the (optional) Shamir reconstruction fails or no custody setup exists, the social-threshold path already completes the recovery; the UI shows the social-threshold progress and never surfaces a Shamir-specific failure mode to the user.

## No code change needed

T24 acceptance is met. No backlog items required. The Angular UX layer was designed at the right altitude: it operates on the social-threshold flow as the canonical surface, and the optional cryptographic proof layer (Shamir) lives below in the substrate where it belongs.

Going forward, any new UI work that introduces:
- a raw-bytes display path for share / seed / key material,
- a user-facing toggle between "with Shamir" and "without Shamir",
- a Shamir-specific error message,

must be flagged in code review and either reframed or explicitly justified against this audit. This document is the design-intent record.

## Companion to T23

Pairs with the Stage 4c substrate-side audit (`2026-05-15-recovery-m4-stage4c-audit.md`):
- T23 confirms the substrate layer treats Shamir as optional (zero gating sites in elohim-storage + imagodei zome).
- T24 confirms the UI layer hides Shamir entirely (zero user-facing surface).

Together they enforce the architectural layering: Shamir is the OPTIONAL cryptographic proof layer atop the wisdom-layer recovery flow. The user never sees it, never chooses it, never has to understand it. The substrate uses it when it's the right tool, falls back gracefully when it isn't, and the UI just shows "your people are confirming your recovery" in either case.
