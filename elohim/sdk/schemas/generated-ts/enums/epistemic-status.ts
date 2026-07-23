/* eslint-disable @typescript-eslint/consistent-indexed-object-style */
/* Generated from protocol schema: enums/epistemic-status.schema.json -- DO NOT EDIT */

/**
 * Communal standing of a claim, as it moves from emergent truth through peer review toward settled canon. Derived-never-stored law: these values name the outcomes of a deterministic fold over witnessed review events (elohim/epr-rea/src/epistemic.rs), NEVER a stored status column — no table, no migration, reconstruction is a re-fold in canonical CID order. Partial order: `emergent` -> `reviewed` -> `canon` is the mechanical ladder; `contested` is sideways, not a rung — an active dispute that routes to a `refer` decision, never a silent resolution; `superseded` is terminal, reached via close-interval by a successor, and the original remains queryable forever. `canon` can only be conferred by a referenced governance act (Mishpat Precedent lineage) — a threshold alone is structurally insufficient to mint it; the maximum mechanical status is `reviewed`. Source of truth: derived — the fold in elohim/epr-rea, never persisted. Orthogonal to the witness ladder (local -> peer-validated -> notarized), which measures evidentiary custody of the record, not communal standing of the claim.
 */
export type EpistemicStatus = 'emergent' | 'reviewed' | 'contested' | 'canon' | 'superseded';
