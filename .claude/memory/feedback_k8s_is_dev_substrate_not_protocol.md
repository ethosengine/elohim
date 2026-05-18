---
name: k8s-is-dev-substrate-not-protocol
description: K8s is the dev/test-bench scaffolding the alpha cluster runs on today; it is NOT part of the elohim protocol. Protocol docs must keep k8s primitives (pods, PVCs, kubectl, StatefulSets, kube-rs) out of substrate descriptions. K8s carries forward only as inspirational analogue for the elohim-operator complexity-collapse pattern.
metadata:
  type: feedback
---

K8s and the elohim protocol are easy to confuse because the analogues are so close. They are NOT the same thing.

**The rule:** Kubernetes is the developer test-bench the alpha cluster currently runs on. It is the bootstrap-phase developer-substrate while the protocol matures. It is NOT a substrate layer of the elohim protocol. Protocol docs must keep k8s primitives — pods, PersistentVolumeClaims, kubectl, StatefulSets, kube-rs, OOMKill, eviction, container-orchestrator APIs, cluster-API observers — out of substrate descriptions.

**What carries forward, once k8s retires:** the *pattern* k8s pioneered. Operator-as-continuous-reconciliation. Declarative-state-as-source-of-truth. Controller-shape (observe → reconcile → no hesitation). These are inspirational analogues for how the elohim-operator orchestrates P2P-native compute, served by AI specialist subagents, on the protocol's own substrate.

**The trajectory** (from `project_upstream_proxy_pattern_brit_rakia` and `/projects/elohim/rakia/docs/plans/2026-05-06-substrate-as-upstream-containment.md`):
- **Today**: alpha cluster runs on k8s. Matthew (the human) is the operator. Seed-deployment records (`deployments.json`) drive k8s manifest rendering.
- **Brit** layer: household-cluster covenant/contract layer. Content replicates via stewardship commitments. Doorway projects web2-compatible APIs (GitHub-shape, etc.). The protocol carries the bytes; web2 is one projection.
- **Rakia** layer: the protocol's own firmament — the substrate that hosts its own development. Once rakia matures, the alpha-cluster's k8s scaffolding retires.
- **The transition is the work** — that's what "generational" means in `project_intelligence_revolution_scales_to_humans`.

**Why:** A leak happened on 2026-05-18 — the resilience epic chapter described the substrate's missing edges as "k8s pod-lifecycle → REA EconomicEvent" and "k8s-style reconciliation controller pattern" and "the alpha cluster's runtime today is k8s — not elohim-hub." User correction: "you leaked k8s implementation into the protocol docs... remember that k8s is our developer convenience/test-bench for development elohim protocol, it goes away when we can retire the abstraction and actually live on our own substrate." The substrate's missing edges are *protocol-native* (node-health observable via gossip + libp2p connection state + EPR reach signals; interpretation in elohim-operator discernment); k8s incidentals belong only in dev/test-bench docs (deployments.json operational notes, jenkins infrastructure, devspace setup).

**How to apply:**
- In substrate / protocol docs (`genesis/docs/content/elohim-protocol/`, `genesis/docs/superpowers/specs/` when describing protocol layers), describe gaps in protocol-native terms: "node-health observable", "peer goes silent", "committed counterparty stops fulfilling", "libp2p connection state changes", "EPR reach signal pathways." Do NOT say "k8s pod-lifecycle", "OOMKill", "kube-rs", "PVC", "kubectl", "StatefulSet" except when explicitly framing k8s as inspirational analogue (clearly labelled as such with `inspired by` / `analogue` framing).
- In dev/test-bench / operational docs (`genesis/orchestrator/`, `genesis/data/rakia/compute-capacity.json`, `_edgenode-consolidated.template.yaml`, `genesis/plans/2026-04-13-device-archetypes-design.md`, Jenkins docs), describe k8s incidentals freely — that's the right place.
- When using k8s primitives as analogues (Part VII's mapping table is a correct example), label them as analogues: "the mapping to k8s is exact in shape and inverse in posture" or "inspired by container-orchestrator controller shapes." Make clear the left side is the inspiration; the right side is the elohim-native answer.
- When describing the alpha cluster, frame it as "developer-substrate today" with the brit/rakia trajectory named, not as "the substrate."
- "PVC" → "storage-volume" or "persistent-volume" (substrate-generic).
- "Pod" → "elohim-node deployment" or "node" (substrate-generic) in protocol docs; "Pod" is fine in dev-substrate docs.
- "kube-rs / kubectl / cluster API" → "substrate-native node-health observer" / "protocol-native observer" in protocol docs.

This is the corrective frame: k8s gave us the *task description* for what elohim-operators have to do; it did not give us the runtime they have to do it on. The protocol's runtime is its own.
