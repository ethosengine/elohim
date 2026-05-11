-- Removes legacy per-entry-type projection tables superseded by 2026-05-12-100000_attestations
-- (source of truth: Holochain DHT); see genesis/docs/superpowers/specs/2026-05-11-attestation-consolidation-design.md §7.4 for the full table list

DROP TABLE IF EXISTS imagodei_attestations;
DROP TABLE IF EXISTS humanity_witnesses;
DROP TABLE IF EXISTS key_stewardships;
DROP TABLE IF EXISTS stewardship_grants;
DROP TABLE IF EXISTS renewal_attestations;
DROP TABLE IF EXISTS recovery_requests;
DROP TABLE IF EXISTS recovery_votes;
DROP TABLE IF EXISTS identity_challenges;
DROP TABLE IF EXISTS challenge_supports;
DROP TABLE IF EXISTS key_revocations;
DROP TABLE IF EXISTS revocation_votes;
DROP TABLE IF EXISTS identity_freezes;
DROP TABLE IF EXISTS stewardship_appeals;
DROP TABLE IF EXISTS policy_inheritances;
DROP TABLE IF EXISTS content_attestations;
DROP TABLE IF EXISTS custodian_commitments;
DROP TABLE IF EXISTS content_successions;
DROP TABLE IF EXISTS health_attestations;
DROP TABLE IF EXISTS doorway_heartbeat_summaries;
DROP TABLE IF EXISTS gate_decision_attestations;
DROP TABLE IF EXISTS proposal_votes;
DROP TABLE IF EXISTS statement_votes;
DROP TABLE IF EXISTS governance_reactions;
