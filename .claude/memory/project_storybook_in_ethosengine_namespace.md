---
name: Storybook lives in ethosengine namespace, not elohim-<env>
description: elohim-<env> namespaces enforce P2P peer isolation via default-deny-cross-env NetworkPolicy; tooling (storybook, design surfaces) belongs in ethosengine which has no such restriction
type: project
originSessionId: cc51fa69-af87-4c58-a30c-b86120b754fc
---
The `elohim-alpha` / `elohim-staging` / `elohim-prod` namespaces each carry a `default-deny-cross-env` NetworkPolicy that enforces P2P peer environment isolation. The policy works correctly for peer-to-peer traffic but has a latent gap with ingress-nginx pods (which run with `hostNetwork: true`).

**The latent gap:** when nginx-ingress-controller and a backend pod are co-located on the same node, Calico's `DefaultEndpointToHost` exception lets host→local-pod traffic skip ingress NetworkPolicy. When they're on different nodes, the packet's source IP is the node IP (e.g. 192.168.86.102), which doesn't match any `namespaceSelector` in the policy, so Felix drops it. `intel-nuc` happens to host nginx-ingress and most frontends, so the gap is invisible until a frontend lands elsewhere. Storybook ended up on `thinkc-p1s` (because intel-nuc was overcommitted) and the gap surfaced.

**Why:** Storybook is a static design surface, NOT a P2P peer. It has no business in a peer-isolation namespace. The `ethosengine` namespace is the existing tooling home and has no default-deny policy, so ingress-nginx hostNetwork traffic reaches the backend regardless of node placement.

**How to apply:** When deploying any pure-tooling/dev-surface workload (storybook, design dashboards, internal admin UIs that aren't peers), use `namespace: ethosengine` rather than `elohim-<env>`. Reserve `elohim-<env>` for actual P2P peer runtimes whose isolation is load-bearing for the protocol's env-validation story.

If you ever do need a tooling deploy *inside* an `elohim-<env>` namespace (rare — say a per-env diagnostic surface), the workaround is an additive NetworkPolicy with podSelector `app.kubernetes.io/component: design-surface` and an `ipBlock` allow for the node LAN range (192.168.86.0/24). But default to `ethosengine` first.
