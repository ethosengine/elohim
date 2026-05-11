-- Observation/Event Layer — Stage 3.2 diversity summary view.
-- See genesis/docs/superpowers/specs/2026-05-11-observation-event-layer-design.md §6.2
--
-- Source of truth: aggregation over the observations table. Classification: C.
-- This is a SQL view (re-evaluated on each query). If query cost dominates,
-- a follow-on can materialise it as a refreshed table driven by a tokio task.

CREATE VIEW observation_diversity_summary AS
SELECT
    subject_cid,
    observation_kind,
    COUNT(DISTINCT observer_cid)              AS distinct_agents,
    COUNT(DISTINCT observer_household_cid)    AS distinct_households,
    COUNT(DISTINCT observer_collective_cid)   AS distinct_collectives,
    COUNT(DISTINCT observer_region)           AS distinct_regions,
    COUNT(DISTINCT observer_archetype)        AS distinct_archetypes,
    COUNT(DISTINCT observer_compute_class)    AS distinct_compute_classes,
    COUNT(*)                                  AS total_count,
    MIN(observed_at)                          AS first_observed_at,
    MAX(observed_at)                          AS last_observed_at
FROM observations
WHERE subject_cid IS NOT NULL
GROUP BY subject_cid, observation_kind;
