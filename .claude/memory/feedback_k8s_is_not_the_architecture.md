---
name: k8s-is-not-the-architecture
description: "Anti-pattern — confusing k8s resources (deployments.json, cluster-state, nodeTypes, pods) with the protocol's actual architecture; k8s is interim compute/hardware/network modeling, destined for subsumption into peer-native EPR compute contracts (brit/rakia)"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: dda22ff0-818e-4f87-8398-38ed1ef4e174
---

Operator gotcha (2026-06-04, qahal household design session): it is a **common anti-pattern to confuse our k8s for our actual architecture**. Short-to-medium term, k8s (deployments.json, cluster-state.yaml, nodeTypes, pod placement, Jenkins deploy-render) only helps model compute/hardware/network. At full protocol maturity it **goes away completely**, subsumed into peer-native modeling and development empowered by EPR compute contracts (**brit/rakia** vocabulary; cf. genesis/data/rakia/compute-capacity.json and [[project_rea_compute_commitment_primitive]]).

**Why:** Design conclusions drawn from k8s surfaces (e.g. "deployments.json can't see households, so add householdId to deployments.json") mistake the scaffolding for the building. The protocol-native home for such facts is the DHT/REA layer (NodeRegistration, Collective membership, commitments), with k8s as a temporary projection of it — never the other way around.

**How to apply:** When a gap appears on a k8s-layer surface, first ask where its peer-native home is (DHT entry, REA commitment, view projection) and design there; treat the k8s artifact as derived/interim. Don't propose enriching k8s manifests as the architectural fix. Also don't over-invest design effort in k8s-layer modeling that the brit/rakia maturity path will subsume.
