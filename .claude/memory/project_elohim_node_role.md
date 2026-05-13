---
name: elohim-node as deployment wrapper (not runtime)
description: elohim-node is a container/placement wrapper enabling elohim-operator to coordinate blades; elohim-storage is the peer-stewarded P2P workload inside it
type: project
originSessionId: b2c4fbee-2183-475a-8cde-b245fa745bc4
---
elohim-node is a **deployment wrapper**, peer-class analogue to the tauri-desktop wrapper or the browser wrapper. Its job is to package elohim-storage for a specific deployment context (a k8s blade, a rack node, a plug-and-play storage unit) and provide whatever deployment glue that context needs.

elohim-storage is the **peer-stewarded P2P workload** — stewards its own state, its policy, its failure modes. It fills its container and makes the most of it. It can deliberately crash its host to test failure-mode intentions. Correction from earlier iterations: the external network surface (e.g., TCP forwarder in front of the localhost-bound conductor) lives **inside elohim-storage as an optional capability**, not in the wrapper — one implementation serves every form factor uniformly, gated by policy.

The split mirrors k8s-but-P2P-native (k8s is transitional developer scaffolding; the long-term orchestrator is **elohim-operator**, which k8s analogies help explain but shouldn't constrain):
- **elohim-operator** + **elohim-node** = "get nodes" level (cluster placement, blade inventory, network topology, storage racks)
- **elohim-storage** = "get pods" level (workload concerns, peer-stewarded state)

Don't anchor designs to k8s primitives (probes, ConfigMaps, SIGTERM). Frame wrapper contracts in operator-generic terms: lifecycle signals, shape reporting, config plumbing — k8s happens to be one implementation today, elohim-operator will be another.

**Why:** A clean wrapper/workload split means elohim-storage is identical across form factors (k8s blade, desktop app, browser, mobile). Deployment-specific concerns (orchestrator lifecycle translation, OS integration, config delivery) live in the wrapper layer. elohim-operator can coordinate placement across blades because nodes expose a consistent wrapper contract.

**How to apply:**
- elohim-node (and peer wrappers: tauri, browser, mobile) are **thin deployment glue**: shape reporting at boot, orchestrator-lifecycle translation, config plumbing, operator-facing health surface.
- Feature capabilities (forwarder, policy engine, heartbeat, stewardship) live in elohim-storage, gated by policy flags. Each wrapper flips the appropriate flags for its context.
- Device/archetype reporting is the wrapper's job (it knows the container shape); policy evaluation is elohim-storage's job (it knows the workload state).
- When designing features, ask: "is this about *placing / packaging* the workload (wrapper) or *running* it (elohim-storage)?"
