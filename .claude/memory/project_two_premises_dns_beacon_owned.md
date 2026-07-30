---
name: project-two-premises-dns-beacon-owned
title: Two-premises DNS — beacon-owned apex/alpha split
description: "Since 2026-07-28 apex=shem/adam, alpha.elohim.host=operations/matthew, both beacon-owned; bites when debugging DNS/TURN/signal routing or record drift"
metadata: 
  node_type: memory
  title: Two-premises DNS — beacon-owned apex/alpha split
  type: project
  originSessionId: 9caa25ac-e730-4203-a14b-f55feef01c40
  modified: 2026-07-28T21:37:34.961Z
---

Since 2026-07-28 the elohim.host zone models the two premises: apex `elohim.host` A = shem WAN (adam's doorway-B; owned by the shem coturn beacon), `alpha.elohim.host` A = operations WAN (matthew; owned by the ops coturn beacon). Adam-side CNAMEs (`doorway`, `signal`, `signal.doorway`) follow the apex; matthew-side names (`doorway-alpha`, `storybook`, `staging`, `signal.alpha`, `signal.doorway-*`) CNAME to `alpha.elohim.host`. Matthew's signal is `signal.alpha.elohim.host`; adam's stays `signal.elohim.host`. ICE URLs are role-named: `turn:alpha.elohim.host:3478` (ops leg) / `turn:elohim.host:3478` (shem leg).

Non-repo facts: a ddclient on the ethosengine HOST (outside k8s, invisible to Loki) used to keep the apex on the ops IP and reverted the beacon's flip 25s after publish (write-war); it now maintains only `ethosengine.com` — elohim.host was removed from it 2026-07-28. Shem's GFiber router cannot edit forward ranges (relay pool 49160-49200 is its fixed contract; ops leg widened to 49152-49999). The beacon's owned lane does per-cycle freshness verification since 345844b06 — an external clobber logs `exclusive record DRIFTED` and self-heals in ≤30s, so check coturn beacon logs first for any future DNS mystery. Related: [[project_alpha_topology_bootstrap_pair]].
