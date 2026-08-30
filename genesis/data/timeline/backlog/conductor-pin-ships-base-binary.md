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

## 2026-08-30 05:55Z — after 9d2842a63 (edge #1392, storage copies /bin/holochain): unchanged

All 7 pods on 1.0.0-dev-9d2842a6 (start ≈05:37Z). Household pods (matthew/jessica/james, node
"ethosengine") log the pre-fix `access.rs:192` line within minutes (matthew 1.04 M lines / 10 min);
shem pods (adam/eve/susan/gertrude) are quiet — 0 saturated, 0 spin, 0 throttled lines — which is
consistent with EITHER the new binary OR a fresh restart on the old one (their spin took >4 min to
resume after the 04:15 roll too). Fleet agent-infos re-signed 05:37–05:47 still carry iroh relay
URLs (`https://relay.alpha.elohim.host…`). The edgenode Dockerfile compiles the fork
`--no-default-features` — tx5 only, no `transport-iroh` (its own comment: "the iroh transport flip
is a later operator-gated wave") — so a CI-built conductor cannot be what signs those addresses.
Whatever the COPY mechanics, the conductor alpha runs is the holo-host base's own iroh-capable
0.6.3-lineage build (correcting the "stock 0.6.0" wording above: the base tag says hc0.6.0 but its
line numbers are 0.6.3's).

Operator check (cannot be done from the workspace — needs exec):
  kubectl -n elohim-alpha exec elohim-matthew-alpha-0 -c elohim-node -- sh -c 'holochain --build-info; sha256sum /usr/local/bin/holochain; ls -la /bin /usr/bin/holochain 2>&1 | head'
  kubectl -n elohim-alpha exec elohim-adam-alpha-0    -c elohim-node -- sh -c 'holochain --build-info; sha256sum /usr/local/bin/holochain'
and on the edgenode image itself: `docker run --rm --entrypoint sh harbor…/elohim-edgenode:conductor-c9a6c4439293-923e17c36c04 -c 'ls -la /bin /usr/bin/holochain; /bin/holochain --build-info; /usr/bin/holochain --build-info'`.

Consequence: the fork's iroh transport (`transport-iroh`, default feature) has never been what the
fleet runs either — the fleet's iroh is the base's. Any fork commit that must reach the fleet needs
(1) the edgenode build to compile the transport the fleet uses, (2) the copy path proven by
`--build-info` in the image build log, (3) a per-pod `--build-info` line at storage boot.
