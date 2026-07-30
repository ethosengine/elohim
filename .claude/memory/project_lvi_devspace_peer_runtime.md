---
name: project_lvi_devspace_peer_runtime
title: lvi — elohim-native devspace peer-runtime
description: lvi = P2P Eclipse-Che-killer devspace runtime (elohim/lvi/); confluence of brit/rakia/eprfs/pod/doorway; spec+roadmap planted 2026-07-20.
metadata: 
  node_type: memory
  title: lvi — elohim-native devspace peer-runtime
  type: project
  originSessionId: 211ded97-7961-428a-8ec6-6754edca6092
  modified: 2026-07-21T00:19:00.039Z
---

**lvi** (льви — "lions"; Lviv — homage to Eclipse Che's Ukrainian engineering) = the Elohim
Protocol **devspace peer-runtime**: P2P-sharable dev environments (openvscode-server in-browser,
doorway-projected preview URLs, semi-ephemeral peer stewardship). Frame: **k8s-powers-over-p2p**,
re-derived not ported ([[k8s_is_not_the_architecture]]); a devspace is **a covenant you MOUNT, not
an image you build+push**.

The key insight is **CONFLUENCE, not greenfield** — five shipped streams nobody had composed:
[[project_brit_next_gen_epr_meta_foundation]] (covenant+source-closures) · rakia (distributed build)
· eprfs (`LocalMaterializer` = mount-don't-ship) · steward/node `pod` (kubelet-analog) · doorway
(ingress) · [[project_rea_compute_commitment_primitive]] (`delegates-compute` = RBAC/quota). Proof:
brit's own `devfile.yaml` pulls harbor/Che today — the exact stack lvi replaces.

**Home + all detail:** `elohim/lvi/` (in-tree, self-governing `.epr-meta` `covers:subtree`, graduates
to `ethosengine/lvi`). Spec `docs/specs/2026-07-20-elohim-native-devspace-design.md`, roadmap
`docs/plans/2026-07-20-lvi-devspace-roadmap.md`, gospel `CLAUDE.md`. Non-negotiable invariant: a
hard sandbox quota isolates a devspace from the host's co-resident conductor/DHT. lvi is an INSTANCE
of the protocol-wide onboarding ladder (device-local conductor → doorway-invite flywheel →
household-rack backend), whose auth/identity spine is ALREADY-shipped doorway substrate (OAuth
authorize/token, custodial keys, chaperone /hc/connect, graduation export_key/confirm_stewardship)
consumed WHOLESALE — lvi adds zero general auth. **Doorway is THIN (bootstrap·signal·auth), NOT a
projector**; the offering peer's **household-rack blades HOST** (contracted per-blade via
delegates-compute); browser↔blade is **WebRTC** (doorway-signaled), openvscode tunneled over the data
channel. P2P-gate: **zero new Holochain entry types** (DevspaceSeed + edit-seal = content-addressed
EPR atoms; run-authority = existing Mishpat delegates-compute; instance+session = operational).
**v1 (operator 2026-07-21) = the firewall-access headline** (reach your devspace from a library
computer through a corporate firewall): M0 scaffold → M1 eprfs proof (+ S1 WebRTC-transport spike, the
gating de-risk) → M2 device-local actuator+containment ([[feedback_household_nodes_is_the_stable_floor]])
→ M3 flywheel+devspace-OAuth(PKCE)+per-devspace-binding → M4 household-rack + WebRTC firewall access.
Deferred: Track B (nexus/harbor/github replacement), cross-peer placement-market, reach-as-promotion,
R4 metering, untrusted-tenant tier, generalized Rung-3 self-service (doorway epic). Confirmed forks: Q1
accept cache loss · Q2 household-trust TTL+quota · Q3 placement-fixed-then-market. Designed via two
adversarial workflows (29-agent vision + 10-agent onboarding-pattern verification).
