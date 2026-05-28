-- Source of truth: derived projection of SessionHumanView × Manifest{kind:'onboarding'}
-- payload for a specific agent.
-- Category C operational — reconstructable from EconomicEvent stream + onboarding
-- Manifest at any time by re-evaluating trigger conditions against SessionHumanView.
--
-- No dht_anchor_hash column: this projection joins two Category C sources.
-- The underlying Manifest DHT entry can be verified via the manifests table
-- (which carries dht_anchor_hash per manifest row).
--
-- Stores active_prompts_json: JSON array of UpgradePromptItem objects (promptId,
-- trigger, title, message, benefits[]). The full schema is defined in:
--   elohim/sdk/schemas/v1/views/upgrade-prompt-view.schema.json
--
-- Keyed by (agent_id, h_app_id) — one row per agent per app.
-- Projection writer: elohim-storage/src/db/upgrade_prompt_view.rs
-- Signal triggers:
--   1. Signal::EconomicEventCreated (qualifying lamadEventType) — re-evaluates
--      after SessionHumanView is updated.
--   2. Signal::ManifestUpdated{kind:"onboarding"} — re-evaluates for all agents
--      when the onboarding Manifest changes.
-- Schema: elohim/sdk/schemas/v1/views/upgrade-prompt-view.schema.json
CREATE TABLE IF NOT EXISTS upgrade_prompt_view (
    agent_id              TEXT NOT NULL,
    h_app_id              TEXT NOT NULL,
    active_prompts_json   TEXT NOT NULL DEFAULT '[]',
    computed_at           TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, h_app_id)
);

CREATE INDEX IF NOT EXISTS idx_upgrade_prompt_view_h_app_id
    ON upgrade_prompt_view (h_app_id);
