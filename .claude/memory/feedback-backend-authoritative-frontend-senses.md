---
name: feedback-backend-authoritative-frontend-senses
title: Backend authoritative; frontend senses/inspires
description: Backend truth-layer owns view types; contract = view schema + app manifest; Rust conforms, codegen projects to TS; UI senses and inspires, never dictates.
metadata: 
  node_type: memory
  type: feedback
  originSessionId: e0e33c02-4c73-4e11-a457-fc723a8c374c
---

The **backend is authoritative.** The frontend **senses, responds to, and inspires** the backend — it surfaces what is useful and what composes well — but it **does not dictate** the final design. When designing the Rust→TS view boundary, author the authoritative view types from the backend truth-layer (domain model, folds, DHT/projection shapes); a front-end design/sensing pass is an INPUT/inspiration to that, never the spec that defines it.

**Why:** Stated by the architect 2026-06-27 during the plural-mishpat-lenses scaffolding, correcting an "outside-in: the UI needs define the view contracts" framing I used. It restates the protocol's truth-layer law (types flow FROM Rust through ts-rs to TS; snake_case never leaves Rust; Angular adapters add computed/derived fields only, never transform wire format; rust-architect owns offline-correct truth, angular-architect stays thin). Letting the UI dictate the view schema inverts the authority and leaks presentation concerns into the substrate.

**How to apply:** Still do the front-end sensing pass for inspiration (what composes, what existing components already consume — e.g. ControversyBadge/Psephos/SymbolicGauge). But DECIDE the view types on the backend's terms, and don't block authoring the authoritative views on the front-end pass. Fold the UI sensing in at the ergonomics/naming/adapter level, not the structural level. Sibling framing guard to [[feedback_frontend_review_eyes_first]] (that is about REVIEW order — eyes before source; this is about AUTHORITY — backend before UI) and [[feedback_k8s_is_not_the_architecture]].

**The CONCRETE contract is the schema + manifest (architect, 2026-06-27 follow-on):** the front↔back contract that makes the code compile consistently end-to-end is the **view schema** (`elohim/sdk/schemas/v1/views/*.schema.json` — SoT for field names, validated by `elohim-storage/tests/schema_contract.rs`) PLUS the **app/SDK manifest vocabulary** (`elohim/sdk/domains/*/manifest.json`, e.g. `signalKinds`). The Rust view (`elohim/elohim-views/src/*.rs`) CONFORMS to the schema; ts-rs `export_bindings` + `pnpm run schema:codegen:ts` are the mechanical projections to TS. Author the schema first, make the Rust conform, never hand-author the TS or let it lead. ("Adding a new view" 6-step: schema → Rust struct → schema_contract test → INTERFACE_FILES → codegen → pre-push freshness gate.)
