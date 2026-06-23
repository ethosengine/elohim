---
name: project_epr_link_first_class_seed_authoring
description: "Deferred design — make content-relationship (EPR link) authoring first-class in the lamad seed pipeline, content-addressed/sealed like doc cites; deferred until course docs graduate to peer-native."
metadata: 
  node_type: memory
  type: project
  originSessionId: 80b79077-9d75-43ad-819d-8856d18dbd6e
---

When regenerating lamad seed JSON from source markdown (elohim-import pipeline), content relationships — `children[]`, `relatedNodeIds[]`, `relationships[]`, `contributors[].presenceId`, prose cross-refs — are authored as bare slug strings with NO verify step, so they drift silently: dangling children, stale `richMedia.bibleVerseCount`, ESV/NIV/NRSVue mismatch on `fct-bible-*` verse nodes, contributor presenceIds with no matching presence.

Operator intent (2026-06-14): promote **EPR-link authoring to first-class**, modeled on doc cite-sealing (`cite-gen --seal`). Each link becomes a content-addressed envelope `{target-content-address, type, context/reach, status: healthy|HELD|DEAD}`; the slug survives only as a display alias resolved at the edge. A `seed-link-seal`/verify tool (sibling of cite-gen) resolves+verifies targets, recomputes derived counts, normalizes formatting, flags drift — it is simultaneously the "careful regen" guardrail AND the EPR-link capability (one tool, not two). The elohim-import skill would gain an "Updating existing seed data" section (automate the mechanical, document the judgment).

P2P-gate classification of the link entity: **Category A2 (Derived)** — a Holochain Link on the parent Content entry using existing Relationship semantics; **content-derived address** (CID/EntryHash, not slug); DHT link is the source of truth, the seed JSON is a projection that should carry the sealed address. MVP needs no DNA change (content-address can start as sha256 of the target node's canonical content, exactly like cite-gen).

**Why:** ad-hoc slug links are the same invisible-drift disease cite-sealing already cured for docs; sealing makes the seed projection match eventual DHT truth.

**How to apply:** DEFERRED by operator until "we graduate these documents into the peer-native world" — do NOT build the seal tool / DNA link tags now. Current practice is the simple update+regen route: mirror the `.md` edits into the seed `content` (it embeds `**Introduction:** / **Conclusion:** / **Raw Markdown:**` copies — update every copy), keep the child-verse graph + counts coherent, bump `updatedAt`, validate with hc-rna-fixtures. When graduation starts, build the seal/verify tool first and dogfood it on FCT. See [[project_principle_p1_reconciliation_controller]] (DHT = truth, storage = projection).
