# SUBJECT FOCUS BASELINE   (generated — sibling of subject-routing.yaml; refreshed on every scope flip)
#
# No narrowing in a subject ⇒ it is FAIR-GAME: work it freely, like ranging over plans/specs out of
# the box. A capability going down narrows a subject — its focus, why, and options are shown below.
# Source of truth: genesis/manifests/cluster-state.yaml + the a2o held/ tree (via scope-reconcile).

substrate: available [harbor-registry, household-nodes]   unavailable [alpha-cluster-6peer(degraded), shem(false)]

FAIR-GAME subjects (7) — no narrowing, everything in focus:
  browser · doorway · elohim-core · peer-oauth-portal · protocol · ssr · storage

NARROWED subjects (10) — a capability is down; focus + options per subject:

### auth   ⚠ narrowed — shem down
  IN FOCUS  : 21 live feature(s), fully testable on available compute
              · genesis/a2o/features/auth/agency-context-labels.feature
              · genesis/a2o/features/auth/agency-pipeline-coherence.feature
              · genesis/a2o/features/auth/auth-lifecycle.feature
              · genesis/a2o/features/auth/conductor-pool-recovery.feature
              · genesis/a2o/features/auth/fixture-humans.feature
              · genesis/a2o/features/auth/operator-onboarding.feature
              · … +15 more
  MIXED     : recovery-shamir-optional.feature — 3 scenario(s) need shem (runtime-skipped, NOT failed)
  HELD      : auth/recovery/cross-stack/recovery-cross-stack-transport.feature — needs shem · returns when shem available
  HELD      : auth/recovery/freeze-floor-blocks-intimate-rotation.feature — needs shem · returns when shem available
  HELD      : auth/recovery/intimate-quorum-happy-path.feature — needs shem · returns when shem available
  WHY       : shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set shem=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to auth's in-focus slice; to pivot, pick a FAIR-GAME subject above

### content   ⚠ narrowed — shem down
  IN FOCUS  : 3 live feature(s), fully testable on available compute
              · genesis/a2o/features/content/relationship-idempotency.feature
              · genesis/a2o/features/content/ssr_capability.feature
              · genesis/a2o/features/content/stewardship-allocation.feature
  MIXED     : content-lifecycle.feature — 2 scenario(s) need shem (runtime-skipped, NOT failed)
  HELD      : content/epr-content-addressing.feature — needs shem · returns when shem available
  WHY       : shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set shem=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to content's in-focus slice; to pivot, pick a FAIR-GAME subject above

### delivery   ⚠ narrowed — alpha-cluster-6peer, shem down
  IN FOCUS  : 4 live feature(s), fully testable on available compute
              · genesis/a2o/features/delivery/content-addressing.feature
              · genesis/a2o/features/delivery/landing-page.feature
              · genesis/a2o/features/delivery/protocol-omnibar.feature
              · genesis/a2o/features/delivery/transport-perf.feature
  MIXED     : delivery-diagnostics.feature — 1 scenario(s) need shem (runtime-skipped, NOT failed)
  MIXED     : peer-mesh.feature — 5 scenario(s) need shem (runtime-skipped, NOT failed)
  MIXED     : spa-bundle-delivery.feature — 5 scenario(s) need alpha-cluster-6peer,shem (runtime-skipped, NOT failed)
  HELD      : delivery/client-resilience.feature — needs shem · returns when shem available
  HELD      : delivery/web2-absorption.feature — needs shem · returns when shem available
  WHY       : alpha-cluster-6peer unavailable — 6-peer alpha soak cluster (adam+matthew bootstrap pair); shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set alpha-cluster-6peer=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to delivery's in-focus slice; to pivot, pick a FAIR-GAME subject above

### deployment   ⚠ narrowed — shem down
  IN FOCUS  : 9 live feature(s), fully testable on available compute
              · genesis/a2o/features/deployment/conductor-admin-reachability.feature
              · genesis/a2o/features/deployment/doorway-self-registration.feature
              · genesis/a2o/features/deployment/human-device-mapping.feature
              · genesis/a2o/features/deployment/ingress-body-size-budget.feature
              · genesis/a2o/features/deployment/p2p-validation.feature
              · genesis/a2o/features/deployment/peer-diversity.feature
              · … +3 more
  MIXED     : compute-commitment-bounds.feature — 1 scenario(s) need shem (runtime-skipped, NOT failed)
  MIXED     : conductor-visibility.feature — 2 scenario(s) need shem (runtime-skipped, NOT failed)
  HELD      : deployment/persona-testnet-validation.feature — needs shem · returns when shem available
  WHY       : shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set shem=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to deployment's in-focus slice; to pivot, pick a FAIR-GAME subject above

### elohim   ⚠ narrowed — alpha-cluster-6peer, shem down
  IN FOCUS  : 2 live feature(s), fully testable on available compute
              · genesis/a2o/features/elohim/content-reach-negotiation.feature
              · genesis/a2o/features/elohim/elohim-presence.feature
  MIXED     : network-health-posture.feature — 4 scenario(s) need shem (runtime-skipped, NOT failed)
  HELD      : elohim/compute-allocation.feature — needs alpha-cluster-6peer · returns when alpha-cluster-6peer available
  HELD      : elohim/compute-coordination.feature — needs shem · returns when shem available
  WHY       : alpha-cluster-6peer unavailable — 6-peer alpha soak cluster (adam+matthew bootstrap pair); shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set alpha-cluster-6peer=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to elohim's in-focus slice; to pivot, pick a FAIR-GAME subject above

### federation   ⚠ narrowed — alpha-cluster-6peer, shem down
  IN FOCUS  : 1 live feature(s), fully testable on available compute
              · genesis/a2o/features/federation/shard-tracking.feature
  MIXED     : epr-cross-peer-resolution.feature — 2 scenario(s) need shem (runtime-skipped, NOT failed)
  MIXED     : peer-advertisement.feature — 9 scenario(s) need shem (runtime-skipped, NOT failed)
  HELD      : federation/cross-doorway-content.feature — needs shem · returns when shem available
  HELD      : federation/cross-mesh-discovery.feature — needs alpha-cluster-6peer,shem · returns when alpha-cluster-6peer,shem available
  WHY       : alpha-cluster-6peer unavailable — 6-peer alpha soak cluster (adam+matthew bootstrap pair); shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set alpha-cluster-6peer=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to federation's in-focus slice; to pivot, pick a FAIR-GAME subject above

### lamad   ⚠ narrowed — shem down
  IN FOCUS  : 5 live feature(s), fully testable on available compute
              · genesis/a2o/features/lamad/assessment-completion-feedback.feature
              · genesis/a2o/features/lamad/attention-analytics.feature
              · genesis/a2o/features/lamad/epr-link-navigation.feature
              · genesis/a2o/features/lamad/learning-journey.feature
              · genesis/a2o/features/lamad/path-adaptation.feature
  HELD      : lamad/know-thyself-discovery.feature — needs shem · returns when shem available
  HELD      : lamad/love-map-negotiation.feature — needs shem · returns when shem available
  WHY       : shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set shem=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to lamad's in-focus slice; to pivot, pick a FAIR-GAME subject above

### qahal   ⚠ narrowed — shem down
  IN FOCUS  : 1 live feature(s), fully testable on available compute
              · genesis/a2o/features/qahal/collective-governance.feature
  HELD      : qahal/feedback-dialogue-panel.feature — needs shem · returns when shem available
  WHY       : shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set shem=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to qahal's in-focus slice; to pivot, pick a FAIR-GAME subject above

### resilience   ⚠ narrowed — shem down
  IN FOCUS  : 1 live feature(s), fully testable on available compute
              · genesis/a2o/features/resilience/substrate-reconciliation.feature
  MIXED     : observable-distribution.feature — 2 scenario(s) need shem (runtime-skipped, NOT failed)
  WHY       : shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set shem=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to resilience's in-focus slice; to pivot, pick a FAIR-GAME subject above

### shefa   ⚠ narrowed — shem down
  IN FOCUS  : 0 live feature(s), fully testable on available compute
  MIXED     : human-resilience.feature — 6 scenario(s) need shem (runtime-skipped, NOT failed)
  MIXED     : m1-matthew-terrance-delivery.feature — 2 scenario(s) need shem (runtime-skipped, NOT failed)
  WHY       : shem unavailable — multi-tenant live P2P canvas — the cross-node proving ground
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set shem=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to shefa's in-focus slice; to pivot, pick a FAIR-GAME subject above

