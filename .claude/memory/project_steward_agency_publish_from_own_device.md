---
index: false
name: project-steward-agency-publish-from-own-device
title: "Steward agency: publish from own device"
description: "Operator 2026-09-29 — master the steward's own agency: publish content from his own device via root-author delegation; pipeline grading is interim"
id: project-steward-agency-publish-from-own-device
metadata:
  node_type: memory
  title: "Steward agency: publish from own device"
  type: project
  originSessionId: 2041165d-7a5c-48ab-9a3e-7bb078885c99
  modified: 2026-09-29T03:16:55.290Z
cites:
  - elohim/holochain/dna/elohim/zomes/content_store/src/head_delegation.rs
---

Operator (2026-09-29, after a night of forcing FCT v2 onto the fleet through the genesis
pipeline): "we need to master my own agency over this content." The target is Matthew
publishing a course from his own workspace conductor (`just dev conductor alpha`) with one
command, and every peer adopting it — body AND reach — because it verifiably came from him.

**Why:** the pipeline path writes as each peer's own agent, so the corpus has mixed root
authors (matthew-alpha, adam, a retired key `uhCAk1pZY60SQ…`), reach never travels on
adoption (F19), and the earned tier is forgeable (anyone can tag a link earned). The
per-node `steward-grade` pipeline step built 2026-09-28 is a workaround, not the model.

**How to apply:** order is (1) authenticate the earned tier (coordinator-only), (2) re-land
F19 with the red-team fixes, (3) device delegation via the existing
`grant_head_delegation` (content_store lib.rs ~4150, scope prefix e.g. `fct-*`) from each
root author to the workspace device, steward-publish `--delegation`. Queued as
`.claude/shifts/next-objective-steward-agency-publish-from-own-device.objective.json`.
The course-as-one-signed-CID-tree (EPRFS/brit) is the later atomic unit. Related:
[[project-attribution-cut-binding-proof-status]], [[feedback-elohim-witness-human-identity]].

**Ruling 2026-10-02 (operator): the authority shape is operator runtime first, devices under it.** In practice the human provisions their peer runtime first — today the hosted operator agent `uhCAkiqczp…` on alpha conductor-4 (account `matthew.operator.dev@alpha.elohim.host`), later their own steward-tier conductor — and workspaces such as Eclipse Che are provisioned *from* that runtime as enrolled devices. Che's binding already has this shape (controller list = the operator key, device = Che). So content roots are authored by the operator runtime and devices receive scoped `grant_head_delegation`s; a device must never be its own root author for a delegation proof. Campaign 1.4 leg 2 therefore starts with the operator account publishing one root, then delegating it to Che (see [[project_campaign14_leg2_lineage_and_root_author]]). Moving the operator runtime from hosted to self-run later is a relocation of the controller, not a redesign.
