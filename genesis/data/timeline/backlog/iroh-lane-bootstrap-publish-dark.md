---
id: "backlog-iroh-lane-bootstrap-publish-dark"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Post-flip: iroh conductors do not republish agent-infos to bootstrap — possible 7-island partition behind a serving facade"
slug: "iroh-lane-bootstrap-publish-dark"
written: "2026-08-07"
author: "agentic-developer"
status: "open"
priority: "critical"
area: "dataplane"
domain: "code"
tags: [iroh, wave2, bootstrap, kitsune2, partition, soak, code-domain]
---

# iroh-lane bootstrap publish is dark

Evidence (2026-08-07 00:15-01:20Z, seam-smoke direct runs post-flip):
- Deploy-time (#1313, mid-restart): bootstrap-sharing OK "5 16 spaces/agents" — the OLD tx5 entries.
- +30m and +80m post-flip: bootstrap-sharing FAIL "A=0 0 B=0 0"; peer-store FAIL total=0 addressed=0
  on BOTH doorways; n0/tx5 seams cannot run ("no peer URLs to inspect").
- Meanwhile: conductors ARE on iroh and registered with the sovereign relays (do_insert_relay at
  23:51Z, all peers), and the federation route serves 200 on both doorways.

Reading: the old bootstrap entries expired (~20min) and the iroh conductors never republished.
On a full-arc fleet every zome read is local-only, so serving routes prove LOCAL data, not mesh —
**the fleet may be 7 DHT islands behind a working facade** (recurrence of the kitsune2
bootstrap-gap class from the doorway-ops incidents, now on the transport-iroh lane).

Decisive next probe (cheap, run first): cross-peer convergence — author a new action on peer A,
watch it arrive on peer B (`✓ canonical head propagated` on the next deploy, the dht-fetch seam
A=/B= heads, or an a2o @concern:content-sync run). If it converges, discovery is degraded but the
mesh holds (relay-held connections / peer stores populated somewhere the doorway view doesn't
read). If it does NOT converge, the flip broke peer discovery and the options are: (a) fix the
bootstrap publish leg on the iroh lane (does kitsune2 transport-iroh wire coreBootstrap the same
way? config key drift? the bootstrap_url flow was tx5-era-verified only), or (b) D9 rollback
(one-line STS repoint to hc-elohim-0.6.3) while (a) is fixed off-fleet.

Suspect surfaces: conductor NetworkConfig bootstrap wiring under transport-iroh (the dep-verification
pack history doc covered bootstrap wire compat — re-read it); doorway /admin/bootstrap-coherence +
conductor-diagnostics read legs (could be reading a store the iroh lane writes elsewhere — verify
the probe against a conductor's own admin API before trusting the zero).

Divergence risk while open: writes on different islands diverge until heal; heal is fills-never-moves
so nothing is lost, but the reconcile grows with time. Alpha-only (staging/prod are tx5).
