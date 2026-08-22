---
name: project_reach_earned_push_deterministic_floor
description: "The pre-push gate dogfoods the protocol — a repo push is a reach-earned attestation at the deterministic floor."
title: Repo push = reach-earned attestation (deterministic floor)
metadata: 
  node_type: memory
  type: project
  originSessionId: 255362f1-5d7f-4734-ba6f-99bc94310d75
---

Operator framing (2026-07-14): the local pre-push gate is the protocol's own mechanics dogfooded on our workflow. The working tree (dev) is the **authoring space** (free — max autonomy). Pushing to the shared repo is a **reach-earning activity** ([[project_weave_epic_arc]] Stance III.2, reach earned before it spreads): it must **earn a reach attestation** (passing gates) before landing. The gate is the **deterministic floor** (Stance II.1) — un-lobbyable HARD-BLOCK. Push this discipline "as low as we can, maybe even epr-meta/eprfs," **at least for CID-addressed artifacts that import into the runtime** (drift there = a live dangling CID / `/blob/<hash>` 404).

**Why:** the gate isn't overhead, it's the protocol enforcing itself on us — this is the north star for how much drift-discipline to build and where.

**How to apply — the ladder (remediation → prevention):**
1. **Push-time (BUILT c133989af):** `genesis/seeder/src/cid-artifact.ts` (canonical derivation) + `cid-artifact-integrity.spec.ts` (attestation) + `sync:cid-artifacts` (fix). Contract: content == frontmatter-stripped source, blobHash == sha256(file), blobCid == CIDv1-raw-sha256(file). Only artifact today: `manifesto.json`.
2. **Edit-time (NEXT, unbuilt):** an `.epr-meta` rule on `genesis/docs/content/elohim-protocol/` firing when a `.md` that is a CID-artifact `sourcePath` is edited → surface "re-sync" at authoring time.
3. **Endgame (deepest floor):** don't STORE blobHash/blobCid — **derive them from `sourcePath` at import time** so the CID is correct by construction; storing a derivable value is the drift surface. Removes the class vs guarding it. Bigger change (seeder + schema + doorway blob flow).

Aside noted here: `genesis/seeder` typecheck is red pre-existing (`wait-for-drain.ts`/`wait-for-pull.ts` missing `@elohim/storage-client` exports); `just gate` runs `install validate test`, not typecheck — see [[feedback_pvc_deferral_hides_gate_debt]].
