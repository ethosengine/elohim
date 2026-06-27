-- Lens-market C-class fold-input tables (lens-market S4).
-- Source of truth: NONE — Classification C (Operational): these are the raw input
-- records the facing folds aggregate ON READ (affinity_in_scope, contention_index).
-- DELIBERATELY NOT DHT-ANCHORED (no dht_anchor_hash by design): affinity and
-- contention are COMPUTED values, and spec §4.4's A/C seam invariant says computed
-- values are never given a dht_anchor_hash. The tables hold the fold INPUTS
-- (selections, verdicts); the OUTPUTS (affinity/contention) are recomputed per request.
--
-- NOTE (plan-name refinement): the plan named these `lens_affinity`/`epr_contention`,
-- but those are fold OUTPUTS. The accurate names for the stored INPUT records are
-- `lens_selections` and `lens_verdicts`. Affinity/contention stay purely computed.
--
-- DORMANT in this slice: the production write-path for selections/verdicts is the
-- ballot/selection leg (DEFERRED — plan A6). These tables read EMPTY until that
-- producer lands; the facing folds degrade to affinity=0 / contention=0 (a still-valid
-- plural market: lenses surfaced side-by-side, no ranking signal yet).
-- Plan: 2026-06-27-plural-mishpat-lenses-service-layer-plan.md (S4).

-- Affinity fold input — an agent exercised (selected) a lens within an EPR scope.
-- Maps 1:1 to elohim_facings::folds::lens_affinity::LensSelectionRow.
CREATE TABLE lens_selections (
    id TEXT PRIMARY KEY,                 -- slug: {lens_cid}:{epr_scope}:{selector_agent}
    lens_cid TEXT NOT NULL,              -- the selected lens (= lenses.cid)
    selector_agent TEXT NOT NULL,        -- who exercised it (distinct selectors → affinity)
    epr_scope TEXT NOT NULL,             -- EPR slug-id the selection is scoped to
    selected_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_lens_selections_scope ON lens_selections(epr_scope);

-- Contention fold input — an agent rendered a verdict on a lens within an EPR scope.
-- Maps 1:1 to elohim_facings::folds::lens_contention::LensVerdictRow.
CREATE TABLE lens_verdicts (
    id TEXT PRIMARY KEY,                 -- slug: {epr_scope}:{lens_cid}:{agent}
    epr_scope TEXT NOT NULL,            -- EPR slug-id the verdict is scoped to
    lens_cid TEXT NOT NULL,            -- the lens being judged (= lenses.cid)
    verdict TEXT NOT NULL,             -- agree|up|for | disagree|down|against (spec §8 controversy)
    agent TEXT NOT NULL,               -- who rendered the verdict
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX idx_lens_verdicts_scope ON lens_verdicts(epr_scope);
