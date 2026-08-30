---
id: "backlog-conductor-pin-ships-base-binary"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The conductor pin tag names the fork commit but the fleet ran the holo-host base's conductor — storage copied /usr/bin/holochain, the edgenode build writes /bin/holochain over a bin→usr/bin base"
slug: "conductor-pin-ships-base-binary"
written: "2026-08-30"
author: "shift 2026-08-30T03-25-workspace-peer-native-content-sync"
status: "in-progress"
priority: "high"
jobs: [elohim-edge, elohim-conductor]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "backlog-2026-08-24-matthew-conductor-saturation"
tags: [conductor-image, pipeline, dockerfile, buildkit, museum-candidate]
---
## Measured (2026-08-30, edge #1391 → all 7 pods on 1.0.0-dev-a17316af, pin conductor-c9a6c4439293-923e17c36c04)

Three independent source-line fingerprints in the fleet's conductor logs after the roll:
`holochain_sqlite/src/db/access.rs:192` "Database read connection is saturated. Util N%" (no
"further occurrences suppressed" suffix — the fork's throttled line is at :239 and always
carries it); `builder.rs:170` passphrase, `config/conductor.rs:552` NETAUDIT (0.6.3 lineage,
not the base tag's 0.6.0); and adam still running the sys-validation spin (37k "No peers to
fetch record" / 15 min) that c9a6c4439 removes. edgenode #26 DID compile c9a6c4439 and
`COPY … /bin/holochain`; the storage image (`elohim/elohim-storage/Dockerfile`) copied
`/usr/bin/holochain` (step executed, not cached; digest 9494bae… is what the nodes pulled).
The holo-host base ships `usr/bin/holochain` (53.7 MB, 2025-11-18) with `bin -> usr/bin`.
Conclusion: `/usr/bin/holochain` in the pinned image is the base's binary; every pointer bump
promoted through this path shipped the base conductor under the fork's tag.

## Cure
- landed 9d2842a63: storage Dockerfile copies `/bin/holochain` (identical when /bin is a plain
  symlink; the only correct path when shadowed).
- follow-up (che-devworkspaces, not auto-watched — `[build:conductor]`): edgenode Dockerfile also
  writes `/usr/bin/holochain`, and prints `holochain --build-info` in the image build log.
- operator confirmation: `holochain --build-info` inside any elohim-node pod, before/after.

## Consequence to re-read
Every "fork fix on the fleet" claim since the holo-host-base Dockerfile (jemalloc leak cure,
cross-relay preflight e4a1c9bb2, …) needs the build-info check before it is trusted; the
saturation flood's "cure" tonight was measured as NOT landed, which is what surfaced this.
