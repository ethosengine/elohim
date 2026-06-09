# SUBJECT FOCUS BASELINE   (generated — sibling of subject-routing.yaml; refreshed on every scope flip)
#
# No narrowing in a subject ⇒ it is FAIR-GAME: work it freely, like ranging over plans/specs out of
# the box. A capability going down narrows a subject — its focus, why, and options are shown below.
# Source of truth: genesis/manifests/cluster-state.yaml + the a2o held/ tree (via scope-reconcile).

substrate: available [harbor-registry, household-nodes, observability, shem]   unavailable [alpha-cluster-6peer(degraded)]

FAIR-GAME subjects (14) — no narrowing, everything in focus:
  auth · browser · content · deployment · doorway · elohim-core · lamad · peer-oauth-portal · protocol · qahal · resilience · shefa · ssr · storage

NARROWED subjects (3) — a capability is down; focus + options per subject:

### delivery   ⚠ narrowed — alpha-cluster-6peer down
  IN FOCUS  : 9 live feature(s), fully testable on available compute
              · genesis/a2o/features/delivery/acquisition-pins.feature
              · genesis/a2o/features/delivery/client-resilience.feature
              · genesis/a2o/features/delivery/content-addressing.feature
              · genesis/a2o/features/delivery/delivery-diagnostics.feature
              · genesis/a2o/features/delivery/landing-page.feature
              · genesis/a2o/features/delivery/peer-mesh.feature
              · … +3 more
  MIXED     : spa-bundle-delivery.feature — 2 scenario(s) need alpha-cluster-6peer (runtime-skipped, NOT failed)
  WHY       : alpha-cluster-6peer unavailable — 6-peer alpha soak cluster (adam+matthew bootstrap pair)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set alpha-cluster-6peer=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to delivery's in-focus slice; to pivot, pick a FAIR-GAME subject above

### elohim   ⚠ narrowed — alpha-cluster-6peer down
  IN FOCUS  : 4 live feature(s), fully testable on available compute
              · genesis/a2o/features/elohim/compute-coordination.feature
              · genesis/a2o/features/elohim/content-reach-negotiation.feature
              · genesis/a2o/features/elohim/elohim-presence.feature
              · genesis/a2o/features/elohim/network-health-posture.feature
  HELD      : elohim/compute-allocation.feature — needs alpha-cluster-6peer · returns when alpha-cluster-6peer available
  WHY       : alpha-cluster-6peer unavailable — 6-peer alpha soak cluster (adam+matthew bootstrap pair)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set alpha-cluster-6peer=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to elohim's in-focus slice; to pivot, pick a FAIR-GAME subject above

### federation   ⚠ narrowed — alpha-cluster-6peer down
  IN FOCUS  : 4 live feature(s), fully testable on available compute
              · genesis/a2o/features/federation/cross-doorway-content.feature
              · genesis/a2o/features/federation/epr-cross-peer-resolution.feature
              · genesis/a2o/features/federation/peer-advertisement.feature
              · genesis/a2o/features/federation/shard-tracking.feature
  HELD      : federation/cross-mesh-discovery.feature — needs alpha-cluster-6peer · returns when alpha-cluster-6peer available
  WHY       : alpha-cluster-6peer unavailable — 6-peer alpha soak cluster (adam+matthew bootstrap pair)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: scope-reconcile.py --set alpha-cluster-6peer=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to federation's in-focus slice; to pivot, pick a FAIR-GAME subject above

