# SUBJECT FOCUS BASELINE   (generated — sibling of subject-routing.yaml; refreshed on every scope flip)
#
# No narrowing in a subject ⇒ it is FAIR-GAME: work it freely, like ranging over plans/specs out of
# the box. A capability going down narrows a subject — its focus, why, and options are shown below.
# Source of truth: genesis/manifests/cluster-state.yaml + the a2o held/ tree (via epr flow report scope).

substrate: available [alpha-cluster-6peer, harbor-registry, household-nodes, observability, shem]   unavailable [local-conductor(false), owned-substrate(false)]

FAIR-GAME subjects (11) — no narrowing, everything in focus:
  browser · devflow · elohim-core · lms · peer-oauth-portal · protocol · qahal · rms · ssr · stewardship · trust

NARROWED subjects (11) — a capability is down; focus + options per subject:

### auth   ⚠ narrowed — local-conductor down
  IN FOCUS  : 37 live feature(s), fully testable on available compute
              · genesis/a2o/features/auth/agency-context-labels.feature
              · genesis/a2o/features/auth/agency-pipeline-coherence.feature
              · genesis/a2o/features/auth/auth-discovery-neighbourhood.feature
              · genesis/a2o/features/auth/auth-discovery.feature
              · genesis/a2o/features/auth/auth-lifecycle.feature
              · genesis/a2o/features/auth/conductor-pool-recovery.feature
              · … +31 more
  HELD      : auth/stewarded-device-sync.feature — needs local-conductor · returns when local-conductor available
  WHY       : local-conductor unavailable — a conductor THIS workspace runs and joins to the lane's network (`just dev conductor alpha`) — the evidence ladder's T3 hybrid rung
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set local-conductor=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to auth's in-focus slice; to pivot, pick a FAIR-GAME subject above

### compute   ⚠ narrowed — owned-substrate down
  IN FOCUS  : 0 live feature(s), fully testable on available compute
  MIXED     : peer-executed-stage.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  WHY       : owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set owned-substrate=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to compute's in-focus slice; to pivot, pick a FAIR-GAME subject above

### content   ⚠ narrowed — owned-substrate down
  IN FOCUS  : 14 live feature(s), fully testable on available compute
              · genesis/a2o/features/content/closure-posture.feature
              · genesis/a2o/features/content/content-graph-resolver-constraints.feature
              · genesis/a2o/features/content/content-lifecycle.feature
              · genesis/a2o/features/content/epistemic-standing.feature
              · genesis/a2o/features/content/epr-atom-home.feature
              · genesis/a2o/features/content/epr-content-addressing.feature
              · … +8 more
  MIXED     : contributor-presences.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  WHY       : owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set owned-substrate=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to content's in-focus slice; to pivot, pick a FAIR-GAME subject above

### dataplane   ⚠ narrowed — owned-substrate down
  IN FOCUS  : 36 live feature(s), fully testable on available compute
              · genesis/a2o/features/dataplane/accountable-correction.feature
              · genesis/a2o/features/dataplane/blob-replication.feature
              · genesis/a2o/features/dataplane/contributor-presence-witnessed-holding.feature
              · genesis/a2o/features/dataplane/coordinator-hot-swap.feature
              · genesis/a2o/features/dataplane/delegated-sweettest.feature
              · genesis/a2o/features/dataplane/doorway-catching-up-page.feature
              · … +30 more
  MIXED     : content-sync.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : doorway-apex-transition.feature — 5 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : doorway-failover.feature — 2 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : doorway-node-identity.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : epr-app-channel-isolation.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : epr-app-deliverability.feature — 6 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : federation-version-convergence.feature — 2 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : notary-authority.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : 11-pull-queue-retires.feature — 2 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  HELD      : dataplane/conductor-bridge-recovery.feature — needs owned-substrate · returns when owned-substrate available
  WHY       : owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set owned-substrate=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to dataplane's in-focus slice; to pivot, pick a FAIR-GAME subject above

### delivery   ⚠ narrowed — owned-substrate down
  IN FOCUS  : 11 live feature(s), fully testable on available compute
              · genesis/a2o/features/delivery/client-resilience.feature
              · genesis/a2o/features/delivery/content-addressing.feature
              · genesis/a2o/features/delivery/delivery-diagnostics.feature
              · genesis/a2o/features/delivery/happ-lineage-migration.feature
              · genesis/a2o/features/delivery/landing-page.feature
              · genesis/a2o/features/delivery/nachalah-allotment.feature
              · … +5 more
  MIXED     : acquisition-pins.feature — 4 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : happ-coordinator-delivery.feature — 3 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : spa-bundle-delivery.feature — 2 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : web2-absorption.feature — 2 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  WHY       : owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set owned-substrate=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to delivery's in-focus slice; to pivot, pick a FAIR-GAME subject above

### deployment   ⚠ narrowed — local-conductor, owned-substrate down
  IN FOCUS  : 13 live feature(s), fully testable on available compute
              · genesis/a2o/features/deployment/compute-commitment-bounds.feature
              · genesis/a2o/features/deployment/conductor-admin-reachability.feature
              · genesis/a2o/features/deployment/conductor-visibility.feature
              · genesis/a2o/features/deployment/device-envelope-floors.feature
              · genesis/a2o/features/deployment/doorway-self-registration.feature
              · genesis/a2o/features/deployment/hub-topology.feature
              · … +7 more
  MIXED     : p2p-validation.feature — 5 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : sync-control.feature — 6 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  HELD      : deployment/persona-testnet-validation.feature — needs (an unavailable cap) · returns when the cap available
  HELD      : deployment/sovereign-peer-join.feature — needs local-conductor · returns when local-conductor available
  WHY       : local-conductor unavailable — a conductor THIS workspace runs and joins to the lane's network (`just dev conductor alpha`) — the evidence ladder's T3 hybrid rung; owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set local-conductor=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to deployment's in-focus slice; to pivot, pick a FAIR-GAME subject above

### doorway   ⚠ narrowed — owned-substrate down
  IN FOCUS  : 0 live feature(s), fully testable on available compute
  MIXED     : native-epr-projection.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : peer-conductor-connection-resilience.feature — 7 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : self-healing-flow-control.feature — 3 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  WHY       : owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set owned-substrate=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to doorway's in-focus slice; to pivot, pick a FAIR-GAME subject above

### elohim   ⚠ narrowed — env down
  IN FOCUS  : 3 live feature(s), fully testable on available compute
              · genesis/a2o/features/elohim/compute-coordination.feature
              · genesis/a2o/features/elohim/content-reach-negotiation.feature
              · genesis/a2o/features/elohim/network-health-posture.feature
  HELD      : elohim/compute-allocation.feature — needs (an unavailable cap) · returns when the cap available
  HELD      : elohim/elohim-presence.feature — needs (an unavailable cap) · returns when the cap available
  WHY       : 
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set <cap>=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to elohim's in-focus slice; to pivot, pick a FAIR-GAME subject above

### federation   ⚠ narrowed — owned-substrate down
  IN FOCUS  : 9 live feature(s), fully testable on available compute
              · genesis/a2o/features/federation/cross-doorway-content.feature
              · genesis/a2o/features/federation/cross-mesh-discovery.feature
              · genesis/a2o/features/federation/doorway-multi-address-failover.feature
              · genesis/a2o/features/federation/doorway-pool-degrade.feature
              · genesis/a2o/features/federation/epr-cross-peer-resolution.feature
              · genesis/a2o/features/federation/membrane-rate-limit.feature
              · … +3 more
  MIXED     : peer-advertisement.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : peer-recovery.feature — 3 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  HELD      : federation/shard-tracking.feature — needs (an unavailable cap) · returns when the cap available
  WHY       : owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set owned-substrate=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to federation's in-focus slice; to pivot, pick a FAIR-GAME subject above

### resilience   ⚠ narrowed — owned-substrate down
  IN FOCUS  : 14 live feature(s), fully testable on available compute
              · genesis/a2o/features/resilience/commitment-backed-card-lighting.feature
              · genesis/a2o/features/resilience/conductor-memory-soak.feature
              · genesis/a2o/features/resilience/conductor-validation-spin.feature
              · genesis/a2o/features/resilience/doorway-footprint-convergence.feature
              · genesis/a2o/features/resilience/governed-distribution.feature
              · genesis/a2o/features/resilience/grandma-photos-survive-node-loss.feature
              · … +8 more
  MIXED     : app-blob-heal-on-read.feature — 3 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : chaos-peer-churn.feature — 5 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : death-witness.feature — 2 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  MIXED     : peer-byteplane-reconstruction.feature — 1 scenario(s) need owned-substrate (runtime-skipped, NOT failed)
  WHY       : owned-substrate unavailable — the lane OWNS its substrate — scenarios may kill peers, restart conductors/doorways, delete blobs, re-key agents (processControl: true)
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set owned-substrate=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to resilience's in-focus slice; to pivot, pick a FAIR-GAME subject above

### storage   ⚠ narrowed — env down
  IN FOCUS  : 3 live feature(s), fully testable on available compute
              · genesis/a2o/features/storage/constitutional-ratio-enforcement.feature
              · genesis/a2o/features/storage/declared-storage-policy.feature
              · genesis/a2o/features/storage/household-resiliency-handshake.feature
  HELD      : storage/disaster-burst-resilience.feature — needs (an unavailable cap) · returns when the cap available
  WHY       : 
  OPTIONS   : (a) work the in-focus + any household scenarios now  (b) expand the plate: epr flow hold --scope --set <cap>=on --apply  (c) pivot to a fair-game subject
  BASELINE/PIVOT: /shift or /brainstorm scoped to storage's in-focus slice; to pivot, pick a FAIR-GAME subject above

