---
id: "backlog-native-dev-evidence-laundering-gaps-integrity"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Evidence-laundering gaps [integrity]: reanchor backfill, release threshold diversity, DNA issuer authorization, device-health subject kinds"
slug: "native-dev-evidence-laundering-gaps-integrity"
written: "2026-11-04"
author: "claude (design pass, native-dev-plane spec; steward human:matthew)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "native-dev-plane-one-derivation-earned-reach-design"
tags: [native-dev-plane, integrity, security, attestation, laundering, dna, d5]
---

Source spec: `native-dev-plane-one-derivation-earned-reach-design` (genesis/docs/superpowers/specs/2026-11-04-native-dev-plane-one-derivation-earned-reach-design.md).

## Item (spec §7 rule 8, D5) [integrity: each ruled by a Claude model]

Four gaps, each to be verified in code before a fix is designed:

1. **Reanchor backfill still admits attestations.** Not located in this pass (grep of `content_store` for backfill found only an ordering heal at `lib.rs:~4401`). Start from `dataplane-reanchor-dead-remaining-rekeyed-peer`.
2. **Release threshold counts deduplicated agent ids without enforcing diversity.** See `task-release-soak-attestation-rail` (diversity thresholds deferred as 'additive later').
3. **DNA attestation issuer authorization accepts all.** Related: `security-attestation-issuer-relaundering` (replicas re-authored under the local key), `dna-health-attestation-ci-authz`.
4. **Release attestations ride `attestation:device-health`, whose subject kinds are unenforced.** D5 rejects this carrier.

**Note.** A `derivation` attestation kind (one per convergence) is to be minted in the next batched constitutional DNA move (join F6). Do not ship a stopgap on `device-health`.

**Receipt.** Adversarial tests per gap; no attestation admitted from backfill, from a non-authorized issuer, or below the diversity threshold.

**retire-when.** The four gaps close with tests and the `derivation` kind lands.

## 2026-11-04 (adversarial review): verified locations and graduation gating (spec §5 D5, §7 rule 8, §11 F6)

- **Gap 1 verified.** Reanchor backfill admitting attestations: `elohim/elohim-storage/src/services/reanchor_backfill.rs:51` (supersedes the "not located" note above).
- **Gap 2 verified.** Release threshold counts distinct agent strings rather than principals: `elohim/elohim-storage/src/services/release_attestation.rs:859-861`.
- **Gap 3 location.** DNA issuer authorization accepts all: `content_store_integrity/src/attestation_validator.rs:67`.
- **Graduation is an enforced verb, not `to_attestation`.** It takes the cited policy, the accepted record set, one standing proof per counted principal, the acting steward and the target reach, and only then mints one notarized attestation per converged (derivation, output CID). `to_attestation` (`elohim/eprfs/eprfs-core/src/derive.rs:175`) is only a draft constructor the verb uses; it takes no policy, standing proof, record set or target reach.
- **Gating.** The three fixes (issuer authorization, release principal counting, reanchor backfill) must be enforced, each with an adversarial test where one human with several devices and agents yields k = 1, before join F6 (graduation). Until the `derivation` kind lands, nothing graduates past `intimate`.
