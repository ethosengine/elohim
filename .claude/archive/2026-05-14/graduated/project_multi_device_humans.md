---
name: Humans have multiple device archetypes
description: Human personas (Matthew, Jessica, etc.) are modeled with multiple simultaneous devices — node, laptop/desktop device, tablet, phone — not a single pod per human.
type: project
originSessionId: d59b6174-405f-478d-a6fc-567fd30edc74
graduated_to: "experience-story-james-son--as-stewardee--stewarded-device-sync"
graduated_on: "2026-05-14"
---

> **GRADUATED 2026-05-14** — Story `james-son--as-stewardee--stewarded-device-sync` carries the multi-device-archetype lesson via lived narrative (James's Chromebook + Matthew's family node + Jessica's device all participating with distinct authority levels). The story is now the canonical surface for this knowledge. This entry preserved in the graduated archive for traceability. See `genesis/data/stories/james-son--as-stewardee--stewarded-device-sync.md`.


Each human persona in the Elohim deployment represents a real person with multiple devices, not a single k8s pod. Matthew has a node (bootstrap/doorway host), a primary device, plus mobile devices (tablet, phone). Jessica has the same shape. James is a stewarded child with a Chromebook-class device dependent on his parents' nodes.

**Why:** Real-world P2P testing needs peer diversity across device archetypes per person — sync behavior differs between a 24-core node, a laptop, and a phone. One-pod-per-human collapses this diversity.

**How to apply:** When designing human manifests, P2P topology, or test scenarios, expect multiple StatefulSets per human (e.g., `elohim-matthew-node`, `elohim-matthew-device`, `elohim-matthew-tablet`). Archetypes should be swappable and runnable simultaneously. Current single-pod-per-human setup is transitional — the target model is multi-device per persona mapped to the Device Archetypes catalog (`genesis/plans/2026-04-13-device-archetypes-plan.md`).
