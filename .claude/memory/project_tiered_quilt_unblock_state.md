---
name: tiered-quilt-unblock-state
description: "tiered-quilt — the attestation consolidation (wave-0 Stage A) already LANDED (Phase-2a); don't re-plan it; the real remaining unblock is the tier-substrate waves"
metadata: 
  node_type: memory
  type: project
  originSessionId: 814f0995-0478-402e-89e7-00813f34980d
---

If asked to "unblock the tiered quilt": the big 4-DNA **attestation consolidation** (wave-0 Stage A — collapse 18+ attestation-shaped entry types across imagodei/elohim/infrastructure/mishpat into one `Content content_type:"attestation:<subtype>"` primitive) **ALREADY SHIPPED** — Phase-2a sprint, commit `34fcf1070` (Stage A→G: coordinator fns, unified `attestations`/`governance_actions` projection + tally, legacy-table drop, Shamir transport, unified routes). History: `2026-06-02-attestation-consolidation-phase2a-dedup.md`. **Do NOT re-plan/re-dispatch the consolidation** (the wave-0 plan + design were stale, presenting it as to-do + awaiting-sign-off; corrected 2026-06-20, commit 9f84c0003 — design `status: Implemented`, wave-0 has a LANDED banner).

**What actually remained (and was closed 2026-06-20, commit `b78908924`):** a codegen `$ref` bug — `codegen-rs.mjs` did a flat `Object.keys(manifest['attestations'])` that never resolved lamad's `$ref` (`lamad/manifest.json:45`), so lamad's 4 subtypes (`attestation:mastery|content-quality|content-succession|custodian-commitment`) were un-mintable (F1 fail-closed) + a phantom `"$ref"` kind leaked. Fixed (resolveRefBlock in codegen). ⚠ It changed the integrity-zome hash → **DNA reinstall operator-gated** (ALLOW_DNA_REINSTALL on adam+matthew together); the mint sweettest was disk-deferred to CI.

**Known open bug (backlog `content-attestations-table-dropped-but-still-consumed.md`):** `content_attestations` table is DROPPED (migration `100300`, applied) but still in `diesel_schema:742` + queried by 8 live files incl. EPR-head reads — a Phase-2a incompleteness; needs an ~8-file migration onto the unified `attestations` projection (NOT a blind dead-code deletion — the dead-code premise was wrong).

**The REAL remaining tiered-quilt work** (the feature, not done): wave-0 **Stage B** (the `lamad_event_type`→`elohim_event_type` rename, EPR-Phase-4-gated) + the **tier-substrate waves** (`quilt_tier_state`, pledge `tier_floor`, the `stocked-warm`/`stocked`/`shelved` temperature classes + tier attestations — tiered-quilt Wave 1+). The tier substrate is what ultimately unblocks [[weave-epic-arc]] #2 tier-capability (which has no servable-tier signal until the temperature classes exist). New tier attestations now mint-able thanks to the $ref fix.
