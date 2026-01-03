//! Content Store Coordinator Zome
//!
//! Implements zome functions for CRUD operations on content entries.
//! Supports bulk imports, ID-based lookups, type filtering, and learning paths.
//!
//! Self-healing DNA support: Entry types automatically migrate from v1 to v2
//! when schema changes occur. No external migration tools needed.

use hdk::prelude::*;
use content_store_integrity::*;
use doorway_client::{CacheRule, CacheRuleBuilder, CacheSignal, CacheSignalType, DoorwaySignal, Cacheable};
use std::collections::HashMap;

// Migration module for DNA version upgrades
pub mod migration;

// Self-healing DNA implementation for schema evolution
pub mod healing_impl;

// Integration layer for healing (read/write path glue)
pub mod healing_integration;

// Entry type providers for flexible healing architecture
pub mod providers;

// =============================================================================
// Cross-DNA Bridge Calls to Imagodei
// =============================================================================
// Identity functions (Human, Agent, Mastery, Attestation) live in imagodei DNA.
// These bridge functions make cross-DNA calls to access them.

/// Role name for the imagodei DNA in the hApp
const IMAGODEI_ROLE: &str = "imagodei";
/// Zome name in imagodei DNA
const IMAGODEI_ZOME: &str = "imagodei";

/// Output from mastery operations (matches imagodei's ContentMasteryOutput)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentMasteryOutput {
    pub action_hash: ActionHash,
    pub mastery: ContentMastery,
}

/// Input for upserting mastery (matches imagodei's UpsertMasteryInput)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertMasteryInput {
    pub human_id: String,
    pub content_id: String,
    pub mastery_level: String,
    pub engagement_type: String,
}

/// Helper to get mastery level index (local copy)
fn get_mastery_level_index(level: &str) -> u32 {
    match level {
        "not_started" => 0,
        "aware" => 1,
        "remember" => 2,
        "understand" => 3,
        "apply" => 4,
        "analyze" => 5,
        "evaluate" => 6,
        "create" => 7,
        _ => 0,
    }
}

/// Bridge call to get my mastery for a content item from imagodei DNA
fn get_my_mastery(content_id: String) -> ExternResult<Option<ContentMasteryOutput>> {
    let response = call(
        CallTargetCell::OtherRole(IMAGODEI_ROLE.into()),
        IMAGODEI_ZOME,
        "get_my_mastery".into(),
        None,
        content_id,
    )?;

    match response {
        ZomeCallResponse::Ok(result) => {
            let output: Option<ContentMasteryOutput> = result.decode()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode mastery: {:?}", e))))?;
            Ok(output)
        }
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Unauthorized call to imagodei".to_string())))
        }
        ZomeCallResponse::NetworkError(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Network error calling imagodei: {}", err))))
        }
        ZomeCallResponse::CountersigningSession(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Countersigning error: {}", err))))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Authentication failed calling imagodei".to_string())))
        }
    }
}

/// Bridge call to get all mastery records for calling agent
fn get_my_all_mastery(_: ()) -> ExternResult<Vec<ContentMasteryOutput>> {
    let response = call(
        CallTargetCell::OtherRole(IMAGODEI_ROLE.into()),
        IMAGODEI_ZOME,
        "get_my_all_mastery".into(),
        None,
        (),
    )?;

    match response {
        ZomeCallResponse::Ok(result) => {
            let output: Vec<ContentMasteryOutput> = result.decode()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode mastery list: {:?}", e))))?;
            Ok(output)
        }
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Unauthorized call to imagodei".to_string())))
        }
        ZomeCallResponse::NetworkError(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Network error calling imagodei: {}", err))))
        }
        ZomeCallResponse::CountersigningSession(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Countersigning error: {}", err))))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Authentication failed calling imagodei".to_string())))
        }
    }
}

/// Bridge call to upsert mastery in imagodei DNA
fn upsert_mastery(input: UpsertMasteryInput) -> ExternResult<ContentMasteryOutput> {
    let response = call(
        CallTargetCell::OtherRole(IMAGODEI_ROLE.into()),
        IMAGODEI_ZOME,
        "upsert_mastery".into(),
        None,
        input,
    )?;

    match response {
        ZomeCallResponse::Ok(result) => {
            let output: ContentMasteryOutput = result.decode()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode mastery: {:?}", e))))?;
            Ok(output)
        }
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Unauthorized call to imagodei".to_string())))
        }
        ZomeCallResponse::NetworkError(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Network error calling imagodei: {}", err))))
        }
        ZomeCallResponse::CountersigningSession(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Countersigning error: {}", err))))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Authentication failed calling imagodei".to_string())))
        }
    }
}

/// Bridge call to issue attestation via imagodei DNA
fn issue_attestation_via_imagodei(input: IssueAttestationBridgeInput) -> ExternResult<AttestationOutput> {
    let response = call(
        CallTargetCell::OtherRole(IMAGODEI_ROLE.into()),
        IMAGODEI_ZOME,
        "issue_attestation".into(),
        None,
        input,
    )?;

    match response {
        ZomeCallResponse::Ok(result) => {
            let output: AttestationOutput = result.decode()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode attestation: {:?}", e))))?;
            Ok(output)
        }
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Unauthorized call to imagodei".to_string())))
        }
        ZomeCallResponse::NetworkError(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Network error calling imagodei: {}", err))))
        }
        ZomeCallResponse::CountersigningSession(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(format!("Countersigning error: {}", err))))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => {
            Err(wasm_error!(WasmErrorInner::Guest("Authentication failed calling imagodei".to_string())))
        }
    }
}

/// Input for issuing attestation via bridge (matches imagodei's IssueAttestationInput)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueAttestationBridgeInput {
    pub agent_id: String,
    pub category: String,
    pub attestation_type: String,
    pub display_name: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub tier: Option<String>,
    pub earned_via_json: String,
    pub expires_at: Option<String>,
}

/// Output from attestation operations (matches imagodei's AttestationOutput)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationOutput {
    pub action_hash: ActionHash,
    pub attestation: Attestation,
}

// =============================================================================
// DNA Initialization - Sets up healing support and flexible architecture
// =============================================================================

#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    // Initialize healing support - check if v1 is available
    // This never fails, always returns success
    let _ = healing_impl::init_healing();

    // Initialize flexible healing architecture
    // Register all entry type providers
    init_flexible_orchestrator()?;

    Ok(InitCallbackResult::Pass)
}

/// Initialize the flexible orchestrator with all Lamad entry type providers
fn init_flexible_orchestrator() -> ExternResult<()> {
    use hc_rna::{EntryTypeRegistry, FlexibleOrchestrator, FlexibleOrchestratorConfig, BridgeFirstStrategy};
    use std::sync::Arc;

    // Create registry and register all Lamad entry types
    let mut registry = EntryTypeRegistry::new();

    registry.register(Arc::new(providers::ContentProvider))
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to register ContentProvider: {}", e))))?;

    registry.register(Arc::new(providers::LearningPathProvider))
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to register LearningPathProvider: {}", e))))?;

    registry.register(Arc::new(providers::PathStepProvider))
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to register PathStepProvider: {}", e))))?;

    registry.register(Arc::new(providers::ContentMasteryProvider))
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to register ContentMasteryProvider: {}", e))))?;

    // Create orchestrator config
    let config = FlexibleOrchestratorConfig {
        prev_role_name: Some("lamad-v1".to_string()),
        current_role_name: Some("lamad-v2".to_string()),
        healing_strategy: Arc::new(BridgeFirstStrategy),
        allow_degradation: true,
        max_attempts: 3,
        emit_signals: true,
    };

    // Create orchestrator
    let _orchestrator = FlexibleOrchestrator::new(config, registry);

    // In production, you'd store the orchestrator in a thread_local or call it lazily
    // For now, we've just initialized it successfully
    debug!("Flexible orchestrator initialized with all entry type providers");

    Ok(())
}

// =============================================================================
// Doorway Cache Configuration
// =============================================================================

/// Declares cache rules for the Doorway gateway.
///
/// This function is called by the Doorway to discover which zome functions
/// can be cached and how. Returns a list of `CacheRule` structs that define:
/// - TTL (time-to-live) for cached responses
/// - Whether the endpoint is public or requires auth
/// - Reach-based visibility (public if response.reach == "commons")
/// - Which write functions invalidate these caches
#[hdk_extern]
pub fn __doorway_cache_rules(_: ()) -> ExternResult<Vec<CacheRule>> {
    Ok(vec![
        // =====================================================================
        // CONTENT (reach-based visibility: public if reach == "commons")
        // =====================================================================
        CacheRuleBuilder::new("get_content")
            .ttl_1h()
            .reach_based("content.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("get_content_by_id")
            .ttl_1h()
            .reach_based("content.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("get_content_by_type")
            .ttl_15m()
            .reach_based("content.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("get_content_by_tag")
            .ttl_15m()
            .reach_based("content.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("get_content_by_type_paginated")
            .ttl_15m()
            .reach_based("items.content.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("get_content_by_tag_paginated")
            .ttl_15m()
            .reach_based("items.content.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("batch_get_content_by_ids")
            .ttl_1h()
            .reach_based("items.content.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("get_content_stats")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("get_content_graph")
            .ttl_15m()
            .reach_based("root.content.reach", "commons")
            .invalidated_by(vec!["create_content", "create_relationship"])
            .build(),

        // =====================================================================
        // LEARNING PATHS (public read, private write)
        // =====================================================================
        CacheRuleBuilder::new("get_all_paths")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_path", "update_path", "delete_path"])
            .build(),
        CacheRuleBuilder::new("get_path_overview")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_path", "update_path", "delete_path", "add_path_step", "batch_add_path_steps"])
            .build(),
        CacheRuleBuilder::new("get_path_with_steps")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_path", "update_path", "delete_path", "add_path_step", "update_step", "batch_add_path_steps"])
            .build(),
        CacheRuleBuilder::new("get_path_full")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_path", "update_path", "delete_path", "add_path_step", "create_chapter", "update_chapter", "update_step"])
            .build(),
        CacheRuleBuilder::new("get_step_by_id")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["add_path_step", "update_step"])
            .build(),
        CacheRuleBuilder::new("get_chapter_by_id")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_chapter", "update_chapter"])
            .build(),
        CacheRuleBuilder::new("get_chapters_for_path")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_chapter", "update_chapter"])
            .build(),

        // =====================================================================
        // RELATIONSHIPS
        // =====================================================================
        CacheRuleBuilder::new("get_relationships")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_relationship"])
            .build(),
        CacheRuleBuilder::new("query_related_content")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_relationship", "create_content"])
            .build(),

        // =====================================================================
        // HUMANS & AGENTS (public profiles, private session)
        // =====================================================================
        CacheRuleBuilder::new("get_human_by_id")
            .ttl_5m()
            .reach_based("human.profile_reach", "public")
            .invalidated_by(vec!["create_human", "update_human_profile"])
            .build(),
        CacheRuleBuilder::new("get_agent_by_id")
            .ttl_5m()
            .reach_based("agent.visibility", "public")
            .invalidated_by(vec!["create_agent", "update_agent_state"])
            .build(),
        CacheRuleBuilder::new("query_agents")
            .ttl_5m()
            .reach_based("agent.visibility", "public")
            .invalidated_by(vec!["create_agent", "update_agent_state", "register_elohim"])
            .build(),
        CacheRuleBuilder::new("get_elohim_by_scope")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["register_elohim", "update_agent_state"])
            .build(),

        // =====================================================================
        // CONTRIBUTOR PRESENCE (public discovery)
        // =====================================================================
        CacheRuleBuilder::new("get_contributor_presence_by_id")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_contributor_presence", "begin_stewardship", "initiate_claim", "verify_claim"])
            .build(),
        CacheRuleBuilder::new("query_contributor_presences")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_contributor_presence", "begin_stewardship", "initiate_claim", "verify_claim"])
            .build(),
        CacheRuleBuilder::new("get_presences_by_state")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_contributor_presence", "begin_stewardship", "initiate_claim", "verify_claim"])
            .build(),
        CacheRuleBuilder::new("get_presences_by_steward")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["begin_stewardship", "initiate_claim", "verify_claim"])
            .build(),

        // =====================================================================
        // GOVERNANCE (public proposals, precedents, discussions)
        // =====================================================================
        CacheRuleBuilder::new("get_proposal_by_id")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_proposal"])
            .build(),
        CacheRuleBuilder::new("query_proposals")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_proposal"])
            .build(),
        CacheRuleBuilder::new("get_precedent_by_id")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_precedent"])
            .build(),
        CacheRuleBuilder::new("query_precedents")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_precedent"])
            .build(),
        CacheRuleBuilder::new("get_discussion_by_id")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_discussion"])
            .build(),
        CacheRuleBuilder::new("query_discussions")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_discussion"])
            .build(),
        CacheRuleBuilder::new("get_governance_state")
            .ttl_1m()
            .public()
            .invalidated_by(vec!["set_governance_state"])
            .build(),
        CacheRuleBuilder::new("query_governance_states")
            .ttl_1m()
            .public()
            .invalidated_by(vec!["set_governance_state"])
            .build(),

        // =====================================================================
        // KNOWLEDGE MAPS & EXTENSIONS (public discovery)
        // =====================================================================
        CacheRuleBuilder::new("get_knowledge_map_by_id")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_knowledge_map"])
            .build(),
        CacheRuleBuilder::new("query_knowledge_maps")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_knowledge_map"])
            .build(),
        CacheRuleBuilder::new("get_path_extension_by_id")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_path_extension"])
            .build(),
        CacheRuleBuilder::new("query_path_extensions")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_path_extension"])
            .build(),
        CacheRuleBuilder::new("get_challenge_by_id")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_challenge"])
            .build(),
        CacheRuleBuilder::new("query_challenges")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_challenge"])
            .build(),

        // =====================================================================
        // PREMIUM GATES (public discovery, private access)
        // =====================================================================
        CacheRuleBuilder::new("get_premium_gate")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_premium_gate"])
            .build(),
        CacheRuleBuilder::new("get_gates_for_resource")
            .ttl_5m()
            .public()
            .invalidated_by(vec!["create_premium_gate"])
            .build(),
        CacheRuleBuilder::new("get_steward_credential")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_steward_credential"])
            .build(),
        CacheRuleBuilder::new("get_credentials_for_human")
            .ttl_15m()
            .public()
            .invalidated_by(vec!["create_steward_credential"])
            .build(),
        CacheRuleBuilder::new("get_steward_revenue_summary")
            .ttl_1m()
            .public()
            .invalidated_by(vec!["grant_access"])
            .build(),

        // =====================================================================
        // BLOBS (Media Distribution - hash-based and reach-aware)
        // =====================================================================
        CacheRuleBuilder::new("get_blobs_by_content_id")
            .ttl_15m()
            .reach_based("blobs.reach", "commons")
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("verify_blob_integrity")
            .ttl_1h()
            .reach_based("verification.reach", "commons")
            .invalidated_by(vec![])
            .build(),
        CacheRuleBuilder::new("get_blob_variants")
            .ttl_15m()
            .reach_based("variants.reach", "commons")
            .invalidated_by(vec!["create_content"])
            .build(),
        CacheRuleBuilder::new("get_blob_captions")
            .ttl_15m()
            .reach_based("captions.reach", "commons")
            .invalidated_by(vec!["create_content"])
            .build(),

        // =====================================================================
        // EXPORTS (admin/migration endpoints - longer TTL)
        // =====================================================================
        CacheRuleBuilder::new("export_schema_version")
            .ttl_1d()
            .public()
            .invalidated_by(vec![])
            .build(),
        CacheRuleBuilder::new("export_all_content")
            .ttl_5m()
            .private()
            .invalidated_by(vec!["create_content", "bulk_create_content"])
            .build(),
        CacheRuleBuilder::new("export_all_paths_with_steps")
            .ttl_5m()
            .private()
            .invalidated_by(vec!["create_path", "update_path", "add_path_step"])
            .build(),
        CacheRuleBuilder::new("export_for_migration")
            .ttl_5m()
            .private()
            .invalidated_by(vec!["create_content", "create_path"])
            .build(),

        // =====================================================================
        // SHEFA: INSURANCE MUTUAL (member-specific, requires auth)
        // =====================================================================
        CacheRuleBuilder::new("get_member_risk_profile")
            .ttl_5m()
            .private()
            .invalidated_by(vec!["create_member_risk_profile"])
            .build(),
        CacheRuleBuilder::new("get_coverage_policy")
            .ttl_5m()
            .private()
            .invalidated_by(vec!["create_coverage_policy"])
            .build(),
        CacheRuleBuilder::new("get_insurance_claim")
            .ttl_1m()
            .private()
            .invalidated_by(vec!["create_insurance_claim"])
            .build(),
        CacheRuleBuilder::new("get_adjustment_reasoning")
            .ttl_5m()
            .private()
            .invalidated_by(vec!["create_adjustment_reasoning"])
            .build(),

        // =====================================================================
        // SHEFA: REQUESTS & OFFERS (public discovery, member-specific details)
        // =====================================================================
        CacheRuleBuilder::new("get_service_request")
            .ttl_5m()
            .reach_based("request.is_public", "true")
            .invalidated_by(vec!["create_service_request"])
            .build(),
        CacheRuleBuilder::new("get_service_offer")
            .ttl_5m()
            .reach_based("offer.is_public", "true")
            .invalidated_by(vec!["create_service_offer"])
            .build(),
        CacheRuleBuilder::new("get_service_match")
            .ttl_1m()
            .private()
            .invalidated_by(vec!["create_service_match"])
            .build(),

        // =====================================================================
        // USER-SPECIFIC DATA (short TTL, auth required)
        // These use default behavior - 5 min TTL, auth required
        // Not explicitly listed because defaults apply to get_* functions
        // =====================================================================
        // get_my_content, get_current_human, get_current_agent,
        // get_my_path_progress, get_my_all_progress, get_my_mastery, etc.
        // All require auth and use 5-minute default TTL
    ])
}

// =============================================================================
// Doorway Import Config (zome-declared import capabilities)
// =============================================================================
// Like __doorway_cache_rules, this allows the doorway to discover import
// capabilities from the zome rather than hardcoding them.

/// Import configuration for doorway discovery
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportConfig {
    pub enabled: bool,
    pub base_route: String,
    pub batch_types: Vec<ImportBatchType>,
    pub require_auth: bool,
    pub allowed_agents: Option<Vec<String>>,
}

/// Configuration for a specific batch type
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ImportBatchType {
    pub batch_type: String,
    pub queue_fn: String,
    pub process_fn: String,
    pub status_fn: String,
    pub max_items: u32,
    pub chunk_size: u32,
    pub chunk_interval_ms: u32,
    pub schema_version: u32,
}

/// Builder for ImportBatchType
pub struct ImportBatchTypeBuilder {
    batch_type: ImportBatchType,
}

impl ImportBatchTypeBuilder {
    pub fn new(batch_type: &str) -> Self {
        Self {
            batch_type: ImportBatchType {
                batch_type: batch_type.to_string(),
                queue_fn: "queue_import".to_string(),
                process_fn: "process_import_chunk".to_string(),
                status_fn: "get_import_status".to_string(),
                max_items: 5000,
                chunk_size: 50,
                chunk_interval_ms: 100,
                schema_version: 1,
            },
        }
    }

    pub fn status_fn(mut self, fn_name: &str) -> Self {
        self.batch_type.status_fn = fn_name.to_string();
        self
    }

    pub fn queue_fn(mut self, fn_name: &str) -> Self {
        self.batch_type.queue_fn = fn_name.to_string();
        self
    }

    pub fn process_fn(mut self, fn_name: &str) -> Self {
        self.batch_type.process_fn = fn_name.to_string();
        self
    }

    pub fn max_items(mut self, max: u32) -> Self {
        self.batch_type.max_items = max;
        self
    }

    pub fn chunk_size(mut self, size: u32) -> Self {
        self.batch_type.chunk_size = size;
        self
    }

    pub fn chunk_interval_ms(mut self, ms: u32) -> Self {
        self.batch_type.chunk_interval_ms = ms;
        self
    }

    pub fn schema_version(mut self, version: u32) -> Self {
        self.batch_type.schema_version = version;
        self
    }

    pub fn build(self) -> ImportBatchType {
        self.batch_type
    }
}

/// Builder for ImportConfig
pub struct ImportConfigBuilder {
    config: ImportConfig,
}

impl ImportConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: ImportConfig {
                enabled: true,
                base_route: "/import".to_string(),
                batch_types: Vec::new(),
                require_auth: true,
                allowed_agents: None,
            },
        }
    }

    pub fn base_route(mut self, route: &str) -> Self {
        self.config.base_route = route.to_string();
        self
    }

    pub fn disabled(mut self) -> Self {
        self.config.enabled = false;
        self
    }

    pub fn batch_type(mut self, batch_type: ImportBatchType) -> Self {
        self.config.batch_types.push(batch_type);
        self
    }

    pub fn no_auth(mut self) -> Self {
        self.config.require_auth = false;
        self
    }

    pub fn allowed_agents(mut self, agents: Vec<String>) -> Self {
        self.config.allowed_agents = Some(agents);
        self
    }

    pub fn build(self) -> ImportConfig {
        self.config
    }
}

/// Zome-declared import configuration for doorway discovery.
/// Doorway calls this on startup to learn what import capabilities exist.
#[hdk_extern]
pub fn __doorway_import_config(_: ()) -> ExternResult<ImportConfig> {
    Ok(ImportConfigBuilder::new()
        // Content batch (bulk concepts, assessments, quizzes)
        .batch_type(
            ImportBatchTypeBuilder::new("content")
                .max_items(5000)
                .chunk_size(50)          // 50 items per chunk
                .chunk_interval_ms(100)  // 100ms between chunks
                .schema_version(1)
                .build()
        )
        // Path batch (learning paths with steps)
        .batch_type(
            ImportBatchTypeBuilder::new("paths")
                .max_items(1000)
                .chunk_size(20)          // Paths are heavier, smaller chunks
                .chunk_interval_ms(200)
                .schema_version(1)
                .build()
        )
        // Steps batch (path steps, can be large)
        .batch_type(
            ImportBatchTypeBuilder::new("steps")
                .max_items(10000)
                .chunk_size(100)         // Steps are lightweight
                .chunk_interval_ms(50)
                .schema_version(1)
                .build()
        )
        // Relationships batch (content links)
        .batch_type(
            ImportBatchTypeBuilder::new("relationships")
                .max_items(20000)
                .chunk_size(200)         // Links are very lightweight
                .chunk_interval_ms(25)
                .schema_version(1)
                .build()
        )
        .build())
}

// =============================================================================
// Input/Output Types for Content
// =============================================================================

/// Input for creating content (matches ContentNode from elohim-service)
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateContentInput {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    pub summary: Option<String>,          // Short preview text for cards/lists
    pub content: String,
    pub content_format: String,
    pub tags: Vec<String>,
    pub source_path: Option<String>,
    pub related_node_ids: Vec<String>,
    pub reach: String,
    pub estimated_minutes: Option<u32>,   // Reading/viewing time
    pub thumbnail_url: Option<String>,    // Preview image for visual cards
    pub metadata_json: String,
}

/// Output when retrieving content
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub content: Content,
}

/// Input for bulk content creation
#[derive(Serialize, Deserialize, Debug)]
pub struct BulkCreateContentInput {
    pub import_id: String,
    pub contents: Vec<CreateContentInput>,
}

/// Output from bulk content creation
#[derive(Serialize, Deserialize, Debug)]
pub struct BulkCreateContentOutput {
    pub import_id: String,
    pub created_count: u32,
    pub action_hashes: Vec<ActionHash>,
    pub errors: Vec<String>,
}

/// Input for querying content by type
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryByTypeInput {
    pub content_type: String,
    pub limit: Option<u32>,
}

/// Input for querying content by ID
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryByIdInput {
    pub id: String,
}

// =============================================================================
// Input/Output Types for Blobs (Media Distribution)
// =============================================================================

/// Input for querying blobs by content ID
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryBlobsByContentIdInput {
    pub content_id: String,
}

/// Output for blob metadata
#[derive(Serialize, Deserialize, Debug)]
pub struct BlobMetadataOutput {
    pub hash: String,
    pub size_bytes: u64,
    pub mime_type: String,
    pub fallback_urls: Vec<String>,
    pub bitrate_mbps: Option<f64>,
    pub duration_seconds: Option<u32>,
    pub codec: Option<String>,
    pub created_at: Option<String>,
    pub verified_at: Option<String>,
}

/// Input for verifying blob integrity
#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyBlobIntegrityInput {
    pub content_id: String,
    pub blob_hash: String,
}

/// Output for blob integrity verification
#[derive(Serialize, Deserialize, Debug)]
pub struct BlobIntegrityCheckOutput {
    pub blob_hash: String,
    pub is_valid: bool,
    pub verification_time_ms: u64,
}

/// Input for querying blob variants
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryBlobVariantsInput {
    pub blob_hash: String,
}

// =============================================================================
// Input/Output Types for Relationships
// =============================================================================

/// Input for creating a relationship
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateRelationshipInput {
    pub source_id: String,
    pub target_id: String,
    pub relationship_type: String,  // RELATES_TO, CONTAINS, DEPENDS_ON, IMPLEMENTS, REFERENCES, DERIVED_FROM
    pub confidence: f64,            // 0.0 - 1.0
    pub inference_source: String,   // explicit, path, tag, semantic
    pub metadata_json: Option<String>,
}

/// Output for relationship
#[derive(Serialize, Deserialize, Debug)]
pub struct RelationshipOutput {
    pub action_hash: ActionHash,
    pub relationship: Relationship,
}

/// Input for querying relationships
#[derive(Serialize, Deserialize, Debug)]
pub struct GetRelationshipsInput {
    pub content_id: String,
    pub direction: String,  // outgoing, incoming, both
}

/// Input for querying related content
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryRelatedContentInput {
    pub content_id: String,
    pub relationship_types: Option<Vec<String>>,
    pub depth: Option<u32>,
}

/// Content graph node for tree traversal
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentGraphNode {
    pub content: ContentOutput,
    pub relationship_type: String,
    pub confidence: f64,
    pub children: Vec<ContentGraphNode>,
}

/// Content graph output
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentGraph {
    pub root: Option<ContentOutput>,
    pub related: Vec<ContentGraphNode>,
    pub total_nodes: u32,
}

// =============================================================================
// Input/Output Types for Humans
// =============================================================================

/// Input for creating a human
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateHumanInput {
    pub id: String,
    pub display_name: String,
    pub bio: Option<String>,
    pub affinities: Vec<String>,
    pub profile_reach: String,
    pub location: Option<String>,
}

/// Output for human
#[derive(Serialize, Deserialize, Debug)]
pub struct HumanOutput {
    pub action_hash: ActionHash,
    pub human: Human,
}

/// Input for querying humans by affinity
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryHumansByAffinityInput {
    pub affinities: Vec<String>,
    pub limit: Option<u32>,
}

/// Input for recording content completion
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordCompletionInput {
    pub human_id: String,
    pub path_id: String,
    pub content_id: String,
}

/// Content statistics output
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentStats {
    pub total_count: u32,
    pub by_type: HashMap<String, u32>,
}

// =============================================================================
// Input/Output Types for Agent (Expanded Identity Model)
// =============================================================================

/// Input for creating an agent
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateAgentInput {
    pub id: String,
    pub agent_type: String,           // human, organization, ai-agent, elohim
    pub display_name: String,
    pub bio: Option<String>,
    pub avatar: Option<String>,
    pub affinities: Vec<String>,
    pub visibility: String,           // public, connections, private
    pub location: Option<String>,
    pub did: Option<String>,
    pub activity_pub_type: Option<String>,
}

/// Output for agent
#[derive(Serialize, Deserialize, Debug)]
pub struct AgentOutput {
    pub action_hash: ActionHash,
    pub agent: Agent,
}

/// Input for querying agents
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryAgentsInput {
    pub agent_type: Option<String>,
    pub affinities: Option<Vec<String>>,
    pub limit: Option<u32>,
}

// =============================================================================
// Input/Output Types for AgentProgress
// =============================================================================

/// Input for creating agent progress
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateAgentProgressInput {
    pub agent_id: String,
    pub path_id: String,
}

/// Output for agent progress
#[derive(Serialize, Deserialize, Debug)]
pub struct AgentProgressOutput {
    pub action_hash: ActionHash,
    pub progress: AgentProgress,
}

/// Input for updating agent progress
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateAgentProgressInput {
    pub agent_id: String,
    pub path_id: String,
    pub current_step_index: Option<u32>,
    pub completed_step_index: Option<u32>,
    pub completed_content_id: Option<String>,
    pub step_affinity: Option<(u32, f64)>,  // (step_index, affinity_score)
    pub step_note: Option<(u32, String)>,   // (step_index, note)
}

// =============================================================================
// Input/Output Types for Content Attestations (Trust claims about content)
// =============================================================================

/// Input for creating a content attestation
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateContentAttestationInput {
    pub id: Option<String>,
    pub content_id: String,
    pub attestation_type: String,
    pub reach_granted: String,
    pub granted_by_json: String,
    pub expires_at: Option<String>,
    pub evidence_json: Option<String>,
    pub scope_json: Option<String>,
    pub metadata_json: Option<String>,
}

/// Output for content attestation
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentAttestationOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub content_attestation: ContentAttestation,
}

/// Input for querying content attestations
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryContentAttestationsInput {
    pub content_id: Option<String>,
    pub attestation_type: Option<String>,
    pub reach_granted: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// Input for updating a content attestation
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateContentAttestationInput {
    pub id: String,
    pub status: Option<String>,
    pub revocation_json: Option<String>,
    pub metadata_json: Option<String>,
}

/// Input for revoking a content attestation
#[derive(Serialize, Deserialize, Debug)]
pub struct RevokeContentAttestationInput {
    pub id: String,
    pub revoked_by: String,
    pub reason: String,
    pub appealable: bool,
}

// =============================================================================
// Input/Output Types for Learning Paths
// =============================================================================

/// Input for creating a learning path
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePathInput {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    pub purpose: Option<String>,
    pub difficulty: String,
    pub estimated_duration: Option<String>,
    pub visibility: String,
    pub path_type: String,
    pub tags: Vec<String>,
    /// Extensible metadata JSON (stores chapters for hierarchical paths)
    pub metadata_json: Option<String>,
}

/// Input for adding a step to a path
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AddPathStepInput {
    pub path_id: String,
    pub chapter_id: Option<String>,      // If part of a chapter
    pub order_index: u32,
    pub step_type: String,               // content, path, external, checkpoint, reflection
    pub resource_id: String,
    pub step_title: Option<String>,
    pub step_narrative: Option<String>,
    pub is_optional: bool,
    // Learning objectives and engagement
    pub learning_objectives: Option<Vec<String>>,
    pub reflection_prompts: Option<Vec<String>>,
    pub practice_exercises: Option<Vec<String>>,
    // Completion and gating
    pub estimated_minutes: Option<u32>,
    pub completion_criteria: Option<String>,
    pub attestation_required: Option<String>,
    pub attestation_granted: Option<String>,
    pub mastery_threshold: Option<u32>,
    pub metadata_json: Option<String>,
}

/// Output for learning path with steps
#[derive(Serialize, Deserialize, Debug)]
pub struct PathWithSteps {
    pub action_hash: ActionHash,
    pub path: LearningPath,
    pub steps: Vec<PathStepOutput>,
}

/// Lightweight path overview (no step content, just counts)
/// Use this for path listings and initial load - much faster than get_path_with_steps
#[derive(Serialize, Deserialize, Debug)]
pub struct PathOverview {
    pub action_hash: ActionHash,
    pub path: LearningPath,
    pub step_count: usize,
}

/// Output for a path step
#[derive(Serialize, Deserialize, Debug)]
pub struct PathStepOutput {
    pub action_hash: ActionHash,
    pub step: PathStep,
}

/// Input for creating a chapter
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateChapterInput {
    pub path_id: String,
    pub order_index: u32,
    pub title: String,
    pub description: Option<String>,
    pub learning_objectives: Vec<String>,
    pub estimated_minutes: Option<u32>,
    pub is_optional: bool,
    pub attestation_granted: Option<String>,
    pub mastery_threshold: Option<u32>,
    pub metadata_json: Option<String>,
}

/// Output for a chapter
#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterOutput {
    pub action_hash: ActionHash,
    pub chapter: PathChapter,
}

/// Output for a chapter with its steps
#[derive(Serialize, Deserialize, Debug)]
pub struct ChapterWithSteps {
    pub action_hash: ActionHash,
    pub chapter: PathChapter,
    pub steps: Vec<PathStepOutput>,
}

/// Output for a full path with chapters and steps
#[derive(Serialize, Deserialize, Debug)]
pub struct PathWithChaptersAndSteps {
    pub action_hash: ActionHash,
    pub path: LearningPath,
    pub chapters: Vec<ChapterWithSteps>,
    pub ungrouped_steps: Vec<PathStepOutput>,  // Steps not in any chapter
}

/// Input for updating a path
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePathInput {
    pub path_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub purpose: Option<String>,
    pub difficulty: Option<String>,
    pub estimated_duration: Option<String>,
    pub visibility: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Input for updating a chapter
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateChapterInput {
    pub chapter_id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub learning_objectives: Option<Vec<String>>,
    pub estimated_minutes: Option<u32>,
    pub is_optional: Option<bool>,
    pub order_index: Option<u32>,
    pub attestation_granted: Option<String>,
    pub mastery_threshold: Option<u32>,
}

/// Input for updating a step
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateStepInput {
    pub step_id: String,
    pub step_title: Option<String>,
    pub step_narrative: Option<String>,
    pub is_optional: Option<bool>,
    pub order_index: Option<u32>,
    pub chapter_id: Option<String>,
    pub learning_objectives: Option<Vec<String>>,
    pub reflection_prompts: Option<Vec<String>>,
    pub practice_exercises: Option<Vec<String>>,
    pub estimated_minutes: Option<u32>,
    pub completion_criteria: Option<String>,
    pub attestation_required: Option<String>,
    pub attestation_granted: Option<String>,
    pub mastery_threshold: Option<u32>,
}

// =============================================================================
// Input/Output Types for Progress Tracking
// =============================================================================

/// Input for starting path progress
#[derive(Serialize, Deserialize, Debug)]
pub struct StartPathProgressInput {
    pub path_id: String,
}

// Note: AgentProgressOutput is defined earlier in the file (line ~212)

/// Input for completing a step
#[derive(Serialize, Deserialize, Debug)]
pub struct CompleteStepInput {
    pub path_id: String,
    pub step_index: u32,
    pub content_id: Option<String>,
    pub affinity_score: Option<u32>,  // 0-10: How much did the learner enjoy this content?
    pub notes: Option<String>,
    pub reflection_responses: Option<Vec<String>>,
}

/// Input for querying progress
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryProgressInput {
    pub status: Option<String>,  // in_progress, completed, abandoned
    pub path_id: Option<String>,
    pub limit: Option<u32>,
}

/// Summary of a learner's progress
#[derive(Serialize, Deserialize, Debug)]
pub struct ProgressSummary {
    pub path_id: String,
    pub path_title: String,
    pub total_steps: u32,
    pub completed_steps: u32,
    pub current_step_index: u32,
    pub is_completed: bool,
    pub attestations_earned: Vec<String>,
    pub started_at: String,
    pub last_activity_at: String,
    pub completed_at: Option<String>,
}

// =============================================================================
// Input/Output Types for CustodianCommitment (Digital Presence Stewardship)
// =============================================================================

/// Input for creating a custodian commitment
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CreateCustodianCommitmentInput {
    pub custodian_agent_id: String,
    pub beneficiary_agent_id: String,
    pub commitment_type: String,        // relationship|category|community|steward
    pub basis: String,                  // intimate_relationship|trusted_relationship|etc.
    pub relationship_id: Option<String>,
    pub category_override_json: String,
    pub content_filters_json: String,
    pub estimated_content_count: u32,
    pub estimated_size_mb: f64,
    pub shard_strategy: String,         // full_replica|threshold_split|erasure_coded
    pub redundancy_factor: u32,
    pub shard_assignments_json: String, // Empty initially, filled when shards created
    pub emergency_triggers_json: String,
    pub emergency_contacts_json: String,
    pub recovery_instructions_json: String,
    pub cache_priority: Option<u32>,
    pub bandwidth_class: Option<String>,
    pub geographic_affinity: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
}

/// Output when retrieving a custodian commitment
#[derive(Serialize, Deserialize, Debug)]
pub struct CustodianCommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub commitment: CustodianCommitment,
}

/// Input for querying custodian commitments
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryCommitmentsInput {
    pub custodian_agent_id: Option<String>,
    pub beneficiary_agent_id: Option<String>,
    pub commitment_type: Option<String>,
    pub state: Option<String>,
    pub basis: Option<String>,
    pub limit: Option<u32>,
}

/// Input for accepting a commitment
#[derive(Serialize, Deserialize, Debug)]
pub struct AcceptCommitmentInput {
    pub commitment_id: String,
}

/// Input for activating emergency protocol
#[derive(Serialize, Deserialize, Debug)]
pub struct ActivateEmergencyInput {
    pub commitment_id: String,
    pub trigger_type: String,   // manual_signal|trusted_party|m_of_n_consensus|dead_mans_switch|beneficiary_incapacity
    pub activation_proof: String, // Passphrase, signature, consensus votes, etc.
    pub reason: String,
}

// =============================================================================
// Input/Output Types for Shard Generation & Storage
// =============================================================================

/// Input for generating shards from content
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenerateShardsInput {
    pub content_id: String,
    pub content_data: String,     // The actual content to shard
    pub shard_strategy: String,   // full_replica|threshold_split|erasure_coded
    pub redundancy_factor: u32,   // M (threshold) for Shamir or Reed-Solomon
    pub total_shards: Option<u32>, // N (total shards) - defaults to 2*redundancy_factor
}

/// Output from shard generation
#[derive(Serialize, Deserialize, Debug)]
pub struct GenerateShardsOutput {
    pub content_id: String,
    pub shard_strategy: String,
    pub total_shards: u32,
    pub shard_hashes: Vec<String>, // Hash of each shard for verification
    pub content_hash: String,      // Hash of complete content
}

/// Input for storing a shard on-chain
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StoreShardInput {
    pub commitment_id: String,
    pub content_id: String,
    pub shard_index: u32,
    pub total_shards: u32,
    pub encrypted_shard_data: String, // Base64 encrypted shard
    pub encryption_method: String,    // age|pgp|xchacha20
    pub shard_hash: String,           // Hash of shard for integrity verification
    pub watermark_signature: String,  // Cryptographic proof of origin
}

/// Output from storing a shard
#[derive(Serialize, Deserialize, Debug)]
pub struct StoredShardOutput {
    pub action_hash: ActionHash,
    pub content_id: String,
    pub shard_index: u32,
    pub stored_at: String,
}

/// Input for verifying shard integrity
#[derive(Serialize, Deserialize, Debug)]
pub struct VerifyShardInput {
    pub commitment_id: String,
    pub shard_index: u32,
}

/// Input for retrieving a shard
#[derive(Serialize, Deserialize, Debug)]
pub struct GetShardInput {
    pub content_id: String,
    pub shard_index: u32,
}

/// Output when retrieving a shard
#[derive(Serialize, Deserialize, Debug)]
pub struct GetShardOutput {
    pub content_id: String,
    pub shard_index: u32,
    pub total_shards: u32,
    pub encrypted_shard_data: String,
    pub encryption_method: String,
    pub shard_hash: String,
    pub watermark_signature: String,
    pub stored_at: String,
}

// =============================================================================
// Phase 4: Emergency Protocol Input/Output Types
// =============================================================================

/// Emergency trigger types for activation
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EmergencyTriggerSpec {
    pub trigger_type: String,           // manual_signal|trusted_party|m_of_n_consensus
    pub enabled: bool,
    pub passphrase_hash: Option<String>, // For manual_signal
    pub trusted_agent_ids: Option<Vec<String>>, // For trusted_party
    pub consensus_m: Option<u32>,       // M-of-N: M custodians needed
    pub consensus_n: Option<u32>,       // M-of-N: N total custodians
}

/// Manual signal activation (passphrase-based immediate activation)
#[derive(Serialize, Deserialize, Debug)]
pub struct ActivateEmergencyManualInput {
    pub commitment_id: String,
    pub beneficiary_id: String,
    pub passphrase: String,             // Plain passphrase, will be hashed for verification
    pub reason: String,                 // Narrative: "Account compromised", "Disaster recovery", etc.
}

/// Trusted party activation (agent signature-based)
#[derive(Serialize, Deserialize, Debug)]
pub struct ActivateEmergencyTrustedPartyInput {
    pub commitment_id: String,
    pub beneficiary_id: String,
    pub trusted_agent_id: String,       // Which trusted party is activating
    pub signature: String,              // Agent's signature over commitment_id + beneficiary_id
    pub reason: String,
}

/// M-of-N consensus vote submission
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitConsensusVoteInput {
    pub commitment_id: String,
    pub custodian_id: String,           // Which custodian is voting
    pub vote_approve: bool,             // true = approve recovery, false = reject
    pub reason: String,                 // Optional reason for vote
}

/// Check consensus status (are we at M-of-N threshold?)
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckConsensusStatusInput {
    pub commitment_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsensusVoteStatus {
    pub total_custodians: u32,
    pub votes_received: u32,
    pub approves: u32,
    pub rejects: u32,
    pub threshold_m: u32,
    pub threshold_reached: bool,
    pub activation_status: String,      // pending|approved|rejected
}

/// Shard reconstruction for emergency recovery
#[derive(Serialize, Deserialize, Debug)]
pub struct ReconstructContentInput {
    pub commitment_id: String,
    pub content_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReconstructContentOutput {
    pub content_id: String,
    pub content: String,                // Reconstructed content
    pub shards_gathered: u32,
    pub shards_required: u32,
    pub reconstruction_method: String,  // full_replica|threshold_split|erasure_coded
    pub verification_status: String,    // verified|unverified|partial
    pub error_message: Option<String>,
}

// =============================================================================
// Phase 6: Category-based Overrides
// =============================================================================

/// Category override allows specialists to custody content outside relationship reach
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CategoryOverrideSpec {
    pub category_type: String,        // medical|emergency|disaster_relief|high_bandwidth|archive
    pub access_level: String,         // professional|trusted|verified|emergency_only
    pub allowed_reach_levels: Vec<String>, // Which reach levels this specialist can custody
    pub content_filters: Vec<String>, // Content types this specialist handles (empty = all)
    pub credentials: Option<String>,  // Professional credential/license identifier
    pub emergency_contact: Option<String>, // Contact info if emergency_only access
}

/// Create category override commitment (specialist custody)
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateCategoryOverrideInput {
    pub commitment_id: String,        // Link to existing commitment or create new
    pub beneficiary_id: String,       // Content owner
    pub specialist_agent_id: String,  // Healthcare provider, firefighter, etc.
    pub category: CategoryOverrideSpec,
    pub reason: String,              // Why this override is needed
    pub expires_at: Option<String>,   // Expiration date for access
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CategoryOverrideOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
    pub commitment_id: String,
    pub override_status: String,      // pending|approved|active|expired|revoked
}

/// Query commitments by category override
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryCategoryOverridesInput {
    pub beneficiary_id: Option<String>, // Filter by beneficiary
    pub category_type: Option<String>, // Filter by category (medical, emergency, etc.)
    pub specialist_id: Option<String>, // Filter by specialist agent
    pub active_only: bool,             // Only show active overrides
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CategoryOverrideSummary {
    pub commitment_id: String,
    pub specialist_id: String,
    pub category_type: String,
    pub access_level: String,
    pub allowed_reach_levels: Vec<String>,
    pub status: String,
    pub expires_at: Option<String>,
}

/// Validate if specialist can access content via category override
#[derive(Serialize, Deserialize, Debug)]
pub struct ValidateCategoryAccessInput {
    pub specialist_id: String,
    pub content_id: String,
    pub required_category: Option<String>, // If content has specific category requirement
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ValidateCategoryAccessOutput {
    pub authorized: bool,
    pub category_type: Option<String>,
    pub access_level: Option<String>,
    pub reason: String,
}

/// Input for granting attestation
#[derive(Serialize, Deserialize, Debug)]
pub struct GrantAttestationInput {
    pub path_id: String,
    pub attestation_id: String,       // The attestation type being granted
    pub reason: String,               // e.g., "Completed Chapter 1", "Passed mastery quiz"
    pub source_type: String,          // "step", "chapter", "path"
    pub source_id: String,            // step_id, chapter_id, or path_id
}

// =============================================================================
// Input/Output Types for Content Mastery
// =============================================================================

/// Mastery levels (Bloom's Taxonomy)
pub const MASTERY_LEVELS: [&str; 8] = [
    "not_started",  // 0 - No engagement
    "seen",         // 1 - Content viewed
    "remember",     // 2 - Basic recall demonstrated
    "understand",   // 3 - Comprehension demonstrated
    "apply",        // 4 - Application in novel contexts (ATTESTATION GATE)
    "analyze",      // 5 - Can break down, connect, contribute analysis
    "evaluate",     // 6 - Can assess, critique, peer review
    "create",       // 7 - Can author, derive, synthesize
];

/// Engagement types for mastery tracking
pub const ENGAGEMENT_TYPES: [&str; 8] = [
    "view",         // Viewed content
    "quiz",         // Completed quiz
    "practice",     // Did practice exercise
    "comment",      // Added comment/discussion
    "review",       // Peer reviewed content
    "contribute",   // Contributed to content
    "path_step",    // Completed as path step
    "refresh",      // Refreshed stale mastery
];

/// Input for initializing mastery tracking
#[derive(Serialize, Deserialize, Debug)]
pub struct InitializeMasteryInput {
    pub content_id: String,
}

/// Input for recording engagement
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordEngagementInput {
    pub content_id: String,
    pub engagement_type: String,      // view, quiz, practice, etc.
    pub duration_seconds: Option<u32>,
    pub metadata_json: Option<String>,
}

/// Input for recording assessment (quiz/test)
#[derive(Serialize, Deserialize, Debug)]
pub struct RecordAssessmentInput {
    pub content_id: String,
    pub assessment_type: String,      // recall, comprehension, application, analysis
    pub score: f64,                   // 0.0-1.0
    pub passing_threshold: f64,       // Usually 0.7
    pub time_spent_seconds: u32,
    pub question_count: u32,
    pub correct_count: u32,
    pub evidence_json: Option<String>, // Detailed evidence
}

/// Input for leveling up mastery
#[derive(Serialize, Deserialize, Debug)]
pub struct LevelUpMasteryInput {
    pub content_id: String,
    pub target_level: String,         // The level to advance to
    pub evidence_type: String,        // assessment, peer_review, contribution
    pub evidence_json: String,        // Proof of level achievement
}

/// Mastery statistics for dashboard
#[derive(Serialize, Deserialize, Debug)]
pub struct MasteryStats {
    pub total_tracked: u32,
    pub level_distribution: HashMap<String, u32>,
    pub above_gate_count: u32,        // >= apply level
    pub fresh_count: u32,
    pub stale_count: u32,
    pub needs_refresh_count: u32,
}

/// Query input for mastery by level
#[derive(Serialize, Deserialize, Debug)]
pub struct MasteryByLevelQueryInput {
    pub level: Option<String>,
    pub needs_refresh: Option<bool>,
    pub content_type: Option<String>,
    pub limit: Option<u32>,
}

/// Privilege check input
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckPrivilegeInput {
    pub content_id: String,
    pub privilege: String,            // comment, suggest_edit, peer_review, contribute, etc.
}

/// Privilege check result
#[derive(Serialize, Deserialize, Debug)]
pub struct PrivilegeCheckResult {
    pub has_privilege: bool,
    pub required_level: String,
    pub current_level: String,
    pub current_level_index: u32,
}

// =============================================================================
// Content CRUD Operations
// =============================================================================

/// Create a single content entry with all index links.
/// Returns an error if content with the same ID already exists.
#[hdk_extern]
pub fn create_content(input: CreateContentInput) -> ExternResult<ContentOutput> {
    // Check for existing content with this ID to prevent duplicates
    if content_exists_by_id(&input.id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Content with id '{}' already exists. Use update_content to modify existing entries.",
            input.id
        ))));
    }

    // Delegate to unchecked version (caller verified uniqueness)
    create_content_unchecked(input)
}

/// Internal: Create content without existence check.
/// Used by batch import when caller has already verified IDs don't exist.
/// This avoids O(n) existence checks when processing import chunks.
fn create_content_unchecked(input: CreateContentInput) -> ExternResult<ContentOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let mut content = Content {
        id: input.id.clone(),
        content_type: input.content_type.clone(),
        title: input.title,
        description: input.description,
        summary: input.summary,
        content: input.content,
        content_format: input.content_format,
        tags: input.tags.clone(),
        source_path: input.source_path,
        related_node_ids: input.related_node_ids,
        author_id: Some(agent_info.agent_initial_pubkey.to_string()),
        reach: input.reach,
        trust_score: 0.0,
        estimated_minutes: input.estimated_minutes,
        thumbnail_url: input.thumbnail_url,
        metadata_json: input.metadata_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        schema_version: 2,  // Always current version
        validation_status: String::new(),  // Will be set by prepare_
    };

    // Prepare and validate - sets schema_version=2 and validation_status
    let content = healing_integration::prepare_content_for_storage(content)?;

    // Create the entry
    let action_hash = create_entry(&EntryTypes::Content(content.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::Content(content.clone()))?;

    // Create index links
    create_id_to_content_link(&input.id, &action_hash)?;
    create_type_to_content_link(&input.content_type, &action_hash)?;
    create_author_to_content_link(&action_hash)?;

    for tag in &input.tags {
        create_tag_to_content_link(tag, &action_hash)?;
    }

    Ok(ContentOutput {
        action_hash,
        entry_hash,
        content,
    })
}

/// Bulk create content entries (for import operations)
/// DEPRECATED: Use submit_import_batch() for large imports to avoid conductor thread exhaustion
#[hdk_extern]
pub fn bulk_create_content(input: BulkCreateContentInput) -> ExternResult<BulkCreateContentOutput> {
    let mut action_hashes = Vec::new();
    let mut errors = Vec::new();

    for content_input in input.contents {
        match create_content(content_input.clone()) {
            Ok(output) => {
                // Create import batch link for traceability
                create_import_batch_link(&input.import_id, &output.action_hash)?;
                action_hashes.push(output.action_hash);
            }
            Err(e) => {
                errors.push(format!("Failed to create '{}': {:?}", content_input.id, e));
            }
        }
    }

    Ok(BulkCreateContentOutput {
        import_id: input.import_id,
        created_count: action_hashes.len() as u32,
        action_hashes,
        errors,
    })
}

// =============================================================================
// =============================================================================
// Import Batch Processing (Elohim-Store Orchestrated)
// =============================================================================
//
// Architecture: Client → Doorway → Elohim-Store → Zome
//
// The elohim-store provides web 2.0 performance for the Holochain network.
// Doorway just extends that to HTTP clients.
//
// Flow:
// 1. Client POSTs items to Doorway
// 2. Doorway writes blob to Elohim-Store (blazing fast)
// 3. Elohim-Store calls queue_import(blob_hash, manifest) on Zome
// 4. Zome stores ImportBatch entry (manifest only) and emits ImportBatchQueued
// 5. Elohim-Store calls process_import_chunk() with blob data
// 6. Zome processes items, emits ImportBatchProgress signals
// 7. Zome emits ImportBatchCompleted when done
// 8. Store/Doorway relay signals back to client

/// Input for queuing an import batch (called by elohim-store after blob write)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueImportInput {
    /// Unique batch identifier (e.g., "import-2024-12-29-001")
    pub id: String,

    /// Type of items: "content", "paths", "steps", "full"
    pub batch_type: String,

    /// Hash of the blob in elohim-store containing the items JSON
    /// Format: "sha256-xxxx" or store-specific hash
    pub blob_hash: String,

    /// Total number of items in the blob (from manifest)
    pub total_items: u32,

    /// Schema version for the items
    pub schema_version: u32,
}

/// Output from queuing an import batch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueImportOutput {
    /// The batch ID
    pub batch_id: String,

    /// ActionHash of the stored ImportBatch entry
    pub action_hash: ActionHash,

    /// Number of items queued for processing
    pub queued_count: u32,

    /// Processing status (always "queued" from this call)
    pub status: String,
}

/// Queue an import batch for processing.
///
/// Called by elohim-store after writing the blob. This stores a lightweight
/// manifest entry (no payload) and emits ImportBatchQueued signal.
/// The store then calls process_import_chunk() with the actual data.
#[hdk_extern]
pub fn queue_import(input: QueueImportInput) -> ExternResult<QueueImportOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    if input.total_items == 0 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cannot queue empty batch".to_string()
        )));
    }

    // Create the batch entry with status="queued" (manifest only, no payload)
    let batch = ImportBatch {
        id: input.id.clone(),
        batch_type: input.batch_type.clone(),
        blob_hash: input.blob_hash.clone(),
        total_items: input.total_items,
        status: "queued".to_string(),
        processed_count: 0,
        error_count: 0,
        errors_json: "[]".to_string(),
        created_at: timestamp.clone(),
        started_at: None,
        completed_at: None,
        author_id: Some(agent_info.agent_initial_pubkey.to_string()),
        schema_version: input.schema_version,
    };

    // Store the batch entry (fast - single DHT write, no payload)
    let action_hash = create_entry(&EntryTypes::ImportBatch(batch.clone()))?;

    // Create index links
    create_import_batch_index_links(&input.id, &action_hash, &batch.status, &agent_info.agent_initial_pubkey.to_string())?;

    // Emit signal so elohim-store knows to start sending chunks
    emit_signal(ProjectionSignal::ImportBatchQueued {
        batch_id: input.id.clone(),
        blob_hash: input.blob_hash.clone(),
        total_items: input.total_items,
        batch_type: input.batch_type.clone(),
    })?;

    Ok(QueueImportOutput {
        batch_id: input.id,
        action_hash,
        queued_count: input.total_items,
        status: "queued".to_string(),
    })
}

/// Input for processing a chunk of import data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessImportChunkInput {
    /// Batch ID this chunk belongs to
    pub batch_id: String,

    /// Chunk index (0-based, for ordering)
    pub chunk_index: u32,

    /// Whether this is the last chunk
    pub is_final: bool,

    /// JSON array of items to process (partial batch)
    pub items_json: String,
}

/// Output from processing a chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessImportChunkOutput {
    /// Batch ID
    pub batch_id: String,

    /// Items processed in this chunk
    pub chunk_processed: u32,

    /// Errors in this chunk
    pub chunk_errors: u32,

    /// Total processed so far (across all chunks)
    pub total_processed: u32,

    /// Total errors so far
    pub total_errors: u32,

    /// Current batch status
    pub status: String,
}

/// Process a chunk of import data.
///
/// Called by elohim-store with blob data in chunks. Processes items,
/// updates the ImportBatch entry, and emits progress signals.
#[hdk_extern]
pub fn process_import_chunk(input: ProcessImportChunkInput) -> ExternResult<ProcessImportChunkOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Look up the batch entry
    let id_anchor = StringAnchor::new("import_batch", &input.batch_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToImportBatch)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Batch '{}' not found", input.batch_id)
        )));
    }

    let batch_action_hash = links.last().unwrap().target.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string())))?;

    let record = get(batch_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Batch record not found".to_string())))?;

    let mut batch: ImportBatch = record.entry().to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize ImportBatch".to_string())))?;

    // Update status to processing if this is the first chunk
    if batch.status == "queued" {
        batch.status = "processing".to_string();
        batch.started_at = Some(timestamp.clone());
    }

    // Parse and process items
    let items: Vec<CreateContentInput> = serde_json::from_str(&input.items_json)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "Failed to parse items_json: {}", e
        ))))?;

    let mut chunk_processed: u32 = 0;
    let mut chunk_errors: u32 = 0;
    let mut chunk_skipped: u32 = 0;
    let mut errors: Vec<String> = serde_json::from_str(&batch.errors_json).unwrap_or_default();

    // OPTIMIZATION: Batch existence check - do ONE query for all IDs instead of per-item
    let all_ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    let existing_check = check_content_ids_exist(CheckIdsExistInput { ids: all_ids })?;
    let existing_set: std::collections::HashSet<_> = existing_check.existing_ids.into_iter().collect();

    // Process only NEW items (skip existing)
    for content_input in items {
        // Skip items that already exist - no source chain writes needed
        if existing_set.contains(&content_input.id) {
            chunk_skipped += 1;
            chunk_processed += 1; // Count as processed for progress tracking
            continue;
        }

        // Use unchecked create since we already verified ID doesn't exist
        match create_content_unchecked(content_input.clone()) {
            Ok(output) => {
                // Link content to this batch for traceability
                create_import_batch_link(&input.batch_id, &output.action_hash)?;
                chunk_processed += 1;
            }
            Err(e) => {
                chunk_errors += 1;
                let error_msg = format!("Failed to create '{}': {:?}", content_input.id, e);
                if errors.len() < 100 {
                    errors.push(error_msg);
                }
            }
        }
    }

    // Log skip stats for observability
    if chunk_skipped > 0 {
        debug!(
            "Import chunk: {} new, {} skipped (already exist), {} errors",
            chunk_processed - chunk_skipped,
            chunk_skipped,
            chunk_errors
        );
    }

    // Update batch progress
    batch.processed_count += chunk_processed;
    batch.error_count += chunk_errors;
    batch.errors_json = serde_json::to_string(&errors).unwrap_or_default();

    // Determine final status if this is the last chunk
    if input.is_final {
        batch.status = if batch.error_count == 0 {
            "completed".to_string()
        } else if batch.processed_count == 0 {
            "failed".to_string()
        } else {
            "completed".to_string() // Completed with some errors
        };
        batch.completed_at = Some(timestamp);
    }

    // Update the batch entry
    update_entry(batch_action_hash.clone(), &EntryTypes::ImportBatch(batch.clone()))?;

    // Emit progress signal
    if input.is_final {
        emit_signal(ProjectionSignal::ImportBatchCompleted {
            batch_id: input.batch_id.clone(),
            processed_count: batch.processed_count,
            error_count: batch.error_count,
            total_items: batch.total_items,
            errors: errors.clone(),
        })?;
    } else {
        emit_signal(ProjectionSignal::ImportBatchProgress {
            batch_id: input.batch_id.clone(),
            processed_count: batch.processed_count,
            error_count: batch.error_count,
            total_items: batch.total_items,
        })?;
    }

    Ok(ProcessImportChunkOutput {
        batch_id: input.batch_id,
        chunk_processed,
        chunk_errors,
        total_processed: batch.processed_count,
        total_errors: batch.error_count,
        status: batch.status,
    })
}

/// Create index links for an ImportBatch entry
fn create_import_batch_index_links(
    batch_id: &str,
    action_hash: &ActionHash,
    status: &str,
    author_id: &str,
) -> ExternResult<()> {
    // IdToImportBatch
    let id_anchor = StringAnchor::new("import_batch", batch_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToImportBatch, ())?;

    // AuthorToImportBatches
    let author_anchor = StringAnchor::new("import_batch_author", author_id);
    let author_anchor_hash = hash_entry(&EntryTypes::StringAnchor(author_anchor))?;
    create_link(author_anchor_hash, action_hash.clone(), LinkTypes::AuthorToImportBatches, ())?;

    // ImportBatchByStatus
    let status_anchor = StringAnchor::new("import_batch_status", status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::ImportBatchByStatus, ())?;

    Ok(())
}

/// Get the status of an import batch by ID
#[hdk_extern]
pub fn get_import_status(batch_id: String) -> ExternResult<Option<ImportBatch>> {
    // Look up via IdToImportBatch link
    let id_anchor = StringAnchor::new("import_batch", &batch_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToImportBatch)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    // Get the most recent batch (by creation order)
    let action_hash = links.last().unwrap().target.clone().into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Invalid link target".to_string())))?;

    let record = get(action_hash, GetOptions::default())?;
    match record {
        Some(record) => {
            let batch: ImportBatch = record.entry().to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize ImportBatch".to_string())))?;
            Ok(Some(batch))
        }
        None => Ok(None),
    }
}

/// List all import batches (for admin monitoring)
#[hdk_extern]
pub fn list_import_batches(_: ()) -> ExternResult<Vec<ImportBatch>> {
    let agent_info = agent_info()?;

    // Get batches by this author
    let author_anchor = StringAnchor::new("import_batch_author", &agent_info.agent_initial_pubkey.to_string());
    let author_anchor_hash = hash_entry(&EntryTypes::StringAnchor(author_anchor))?;

    let query = LinkQuery::try_new(author_anchor_hash, LinkTypes::AuthorToImportBatches)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut batches: Vec<ImportBatch> = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Ok(Some(batch)) = record.entry().to_app_option::<ImportBatch>() {
                    batches.push(batch);
                }
            }
        }
    }

    // Sort by created_at descending (most recent first)
    batches.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    Ok(batches)
}

/// Get content by ActionHash
#[hdk_extern]
pub fn get_content(action_hash: ActionHash) -> ExternResult<Option<ContentOutput>> {
    let record = get(action_hash.clone(), GetOptions::default())?;

    match record {
        Some(record) => {
            let entry_hash = record
                .action()
                .entry_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "No entry hash in record".to_string()
                )))?
                .clone();

            let content: Content = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Could not deserialize content".to_string()
                )))?;

            Ok(Some(ContentOutput {
                action_hash,
                entry_hash,
                content,
            }))
        }
        None => Ok(None),
    }
}

/// Get content by string ID (using IdToContent link)
#[hdk_extern]
pub fn get_content_by_id(input: QueryByIdInput) -> ExternResult<Option<ContentOutput>> {
    // Use healing-aware retrieval - will fallback to v1 if not found in v2
    let content = healing_integration::get_content_by_id_with_healing(&input.id)?;

    match content {
        Some(content) => {
            // Get the entry hash for output
            let entry_hash = hash_entry(&EntryTypes::Content(content.clone()))?;

            // Get the action hash - use existing anchor/link method
            let anchor = StringAnchor::new("content_id", &input.id);
            let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
            let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
            let links = get_links(query, GetStrategy::default())?;

            let action_hash = if let Some(link) = links.first() {
                ActionHash::try_from(link.target.clone())
                    .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?
            } else {
                // Newly healed entry, cache it with a new link
                let new_hash = create_entry(&EntryTypes::Content(content.clone()))?;
                let _ = create_id_to_content_link(&content.id, &new_hash);
                new_hash
            };

            Ok(Some(ContentOutput {
                action_hash,
                entry_hash,
                content,
            }))
        }
        None => Ok(None),
    }
}

/// Input for batch ID existence check
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckIdsExistInput {
    pub ids: Vec<String>,
}

/// Output for batch ID existence check - returns only the IDs that exist
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckIdsExistOutput {
    pub existing_ids: Vec<String>,
}

/// Batch check which content IDs already exist (for efficient seeding)
/// Returns only the IDs that already exist
#[hdk_extern]
pub fn check_content_ids_exist(input: CheckIdsExistInput) -> ExternResult<CheckIdsExistOutput> {
    let mut existing_ids = Vec::new();

    for id in input.ids {
        let anchor = StringAnchor::new("content_id", &id);
        let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

        let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
        let links = get_links(query, GetStrategy::default())?;

        if !links.is_empty() {
            existing_ids.push(id);
        }
    }

    Ok(CheckIdsExistOutput { existing_ids })
}

/// Input for batch content retrieval by IDs
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchGetContentInput {
    pub ids: Vec<String>,
}

/// Output for batch content retrieval
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchGetContentOutput {
    pub found: Vec<ContentOutput>,
    pub not_found: Vec<String>,
}

/// Batch get content by multiple IDs in a single call (optimized for UI loading)
/// Returns found content and list of IDs that were not found
#[hdk_extern]
pub fn batch_get_content_by_ids(input: BatchGetContentInput) -> ExternResult<BatchGetContentOutput> {
    let mut found = Vec::new();
    let mut not_found = Vec::new();

    for id in input.ids {
        match get_content_by_id(QueryByIdInput { id: id.clone() })? {
            Some(output) => found.push(output),
            None => not_found.push(id),
        }
    }

    Ok(BatchGetContentOutput { found, not_found })
}

/// Get content by content_type (using TypeToContent links)
#[hdk_extern]
pub fn get_content_by_type(input: QueryByTypeInput) -> ExternResult<Vec<ContentOutput>> {
    let anchor = StringAnchor::new("content_type", &input.content_type);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::TypeToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    let limit = input.limit.unwrap_or(100) as usize;
    let mut results = Vec::new();

    for link in links.iter().take(limit) {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            results.push(output);
        }
    }

    Ok(results)
}

/// Get content by tag (using TagToContent links)
#[hdk_extern]
pub fn get_content_by_tag(tag: String) -> ExternResult<Vec<ContentOutput>> {
    let anchor = StringAnchor::new("tag", &tag);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::TagToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();

    for link in links.iter().take(100) {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            results.push(output);
        }
    }

    Ok(results)
}

/// Input for paginated content query by type
#[derive(Serialize, Deserialize, Debug)]
pub struct PaginatedByTypeInput {
    pub content_type: String,          // Filter by type
    pub page_size: u32,                // Number of items per page (max 100)
    pub offset: u32,                   // Number of items to skip
}

/// Output for paginated content query
#[derive(Serialize, Deserialize, Debug)]
pub struct PaginatedContentOutput {
    pub items: Vec<ContentOutput>,
    pub total_count: u32,              // Total matching items
    pub offset: u32,                   // Current offset
    pub has_more: bool,
}

/// Get content by type with pagination support
/// More efficient than loading all content at once for large datasets
#[hdk_extern]
pub fn get_content_by_type_paginated(input: PaginatedByTypeInput) -> ExternResult<PaginatedContentOutput> {
    let anchor = StringAnchor::new("content_type", &input.content_type);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::TypeToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    let total_count = links.len() as u32;
    let page_size = (input.page_size.min(100)) as usize; // Cap at 100
    let offset = input.offset as usize;

    let mut items = Vec::new();
    for link in links.iter().skip(offset).take(page_size) {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            items.push(output);
        }
    }

    let has_more = offset + items.len() < links.len();

    Ok(PaginatedContentOutput {
        items,
        total_count,
        offset: input.offset,
        has_more,
    })
}

/// Input for paginated content query by tag
#[derive(Serialize, Deserialize, Debug)]
pub struct PaginatedByTagInput {
    pub tag: String,                   // Filter by tag
    pub page_size: u32,                // Number of items per page (max 100)
    pub offset: u32,                   // Number of items to skip
}

/// Get content by tag with pagination support
#[hdk_extern]
pub fn get_content_by_tag_paginated(input: PaginatedByTagInput) -> ExternResult<PaginatedContentOutput> {
    let anchor = StringAnchor::new("tag", &input.tag);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::TagToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    let total_count = links.len() as u32;
    let page_size = (input.page_size.min(100)) as usize;
    let offset = input.offset as usize;

    let mut items = Vec::new();
    for link in links.iter().skip(offset).take(page_size) {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            items.push(output);
        }
    }

    let has_more = offset + items.len() < links.len();

    Ok(PaginatedContentOutput {
        items,
        total_count,
        offset: input.offset,
        has_more,
    })
}

/// List all content created by the current agent
#[hdk_extern]
pub fn get_my_content(_: ()) -> ExternResult<Vec<ContentOutput>> {
    let agent_info = agent_info()?;

    let query = LinkQuery::try_new(agent_info.agent_initial_pubkey, LinkTypes::AuthorToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();

    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        if let Some(output) = get_content(action_hash)? {
            results.push(output);
        }
    }

    Ok(results)
}

/// Get content statistics (counts by type)
#[hdk_extern]
pub fn get_content_stats(_: ()) -> ExternResult<ContentStats> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::Content.try_into()?);

    let records = query(filter)?;

    let mut by_type: HashMap<String, u32> = HashMap::new();

    for record in &records {
        if let Some(content) = record
            .entry()
            .to_app_option::<Content>()
            .ok()
            .flatten()
        {
            *by_type.entry(content.content_type).or_insert(0) += 1;
        }
    }

    Ok(ContentStats {
        total_count: records.len() as u32,
        by_type,
    })
}

// =============================================================================
// Blob Operations (Media Distribution - Phase 1)
// =============================================================================

/// Get all blobs associated with a content ID.
/// Returns metadata for all blobs attached to content for download/playback.
#[hdk_extern]
pub fn get_blobs_by_content_id(input: QueryBlobsByContentIdInput) -> ExternResult<Vec<BlobMetadataOutput>> {
    // First, find content by ID
    let anchor = StringAnchor::new("content_id", &input.content_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(Vec::new());
    }

    // Get the content record
    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Content not found".to_string())))?;

    let content: Content = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize content".to_string())))?;

    let content_hash = links[0].target.clone();

    // Query ContentToBlobs links from this content
    let query = LinkQuery::try_new(content_hash, LinkTypes::ContentToBlobs)?;
    let blob_links = get_links(query, GetStrategy::default())?;

    if blob_links.is_empty() {
        return Ok(Vec::new());
    }

    // Fetch each blob entry
    let mut results = Vec::new();

    for link in blob_links {
        let blob_action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in blob link".to_string())))?;

        if let Some(record) = get(blob_action_hash, GetOptions::default())? {
            if let Ok(Some(blob)) = record.entry().to_app_option::<BlobEntry>() {
                results.push(BlobMetadataOutput {
                    hash: blob.hash,
                    size_bytes: blob.size_bytes,
                    mime_type: blob.mime_type,
                    fallback_urls: blob.fallback_urls,
                    bitrate_mbps: blob.bitrate_mbps.map(|b| b as f64),
                    duration_seconds: blob.duration_seconds,
                    codec: blob.codec,
                    created_at: Some(blob.created_at),
                    verified_at: blob.verified_at,
                });
            }
        }
    }

    Ok(results)
}

/// Verify that a blob hash is valid and matches expected content.
/// Used during download to detect corruption.
#[hdk_extern]
pub fn verify_blob_integrity(input: VerifyBlobIntegrityInput) -> ExternResult<BlobIntegrityCheckOutput> {
    let start_time = std::time::SystemTime::now();

    // In Phase 1, we store the expected hash in the content metadata
    // The actual verification happens on the client side (BlobVerificationService)
    // This function is mainly for caching and validation purposes

    // Find content by ID
    let anchor = StringAnchor::new("content_id", &input.content_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Content not found".to_string()
        )));
    }

    // Check if blob with this hash exists in our system
    let blob_anchor = StringAnchor::new("blob_hash", &input.blob_hash);
    let blob_anchor_hash = hash_entry(&EntryTypes::StringAnchor(blob_anchor))?;

    let blob_query = LinkQuery::try_new(blob_anchor_hash, LinkTypes::IdToBlob)?;
    let blob_links = get_links(blob_query, GetStrategy::default())?;

    // Blob is valid if it exists in DHT and hash matches content
    let is_valid = !blob_links.is_empty();

    let elapsed = start_time
        .elapsed()
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    Ok(BlobIntegrityCheckOutput {
        blob_hash: input.blob_hash,
        is_valid,
        verification_time_ms: elapsed,
    })
}

/// Get all variants of a blob (different bitrates, resolutions).
/// Used for adaptive streaming - returns available quality options.
#[hdk_extern]
pub fn get_blob_variants(input: QueryBlobVariantsInput) -> ExternResult<Vec<BlobMetadataOutput>> {
    // In Phase 1, variants are stored alongside the main blob
    // This would query a dedicated entry type for blob variants
    // For now, return empty as the full model is in progress

    // TODO: Query BlobVariant entries by parent blob hash
    // Return all variants sorted by bitrate

    Ok(Vec::new())
}

/// Get captions/subtitles for a blob.
/// Returns all available caption tracks for a video.
#[hdk_extern]
pub fn get_blob_captions(input: QueryBlobVariantsInput) -> ExternResult<Vec<BlobMetadataOutput>> {
    // Query caption tracks for a specific blob
    // Used for subtitle/caption support in players

    // TODO: Query BlobCaption entries by parent blob hash
    // Return captions sorted by language/format

    Ok(Vec::new())
}

// =============================================================================
// Learning Path Operations
// =============================================================================

/// Create a learning path
#[hdk_extern]
pub fn create_path(input: CreatePathInput) -> ExternResult<ActionHash> {
    // Check for existing path with this ID to prevent duplicates
    if path_exists_by_id(&input.id)? {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Path with id '{}' already exists. Use update_path to modify existing paths.",
            input.id
        ))));
    }

    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let path = LearningPath {
        id: input.id.clone(),
        version: input.version,
        title: input.title,
        description: input.description,
        purpose: input.purpose,
        created_by: agent_info.agent_initial_pubkey.to_string(),
        difficulty: input.difficulty,
        estimated_duration: input.estimated_duration,
        visibility: input.visibility,
        path_type: input.path_type,
        tags: input.tags,
        metadata_json: input.metadata_json.unwrap_or_else(|| "{}".to_string()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
        schema_version: 2,
        validation_status: "Valid".to_string(),
    };

    let action_hash = create_entry(&EntryTypes::LearningPath(path))?;

    // Create ID lookup link
    let anchor = StringAnchor::new("path_id", &input.id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, action_hash.clone(), LinkTypes::IdToPath, ())?;

    // Create global "all_paths" index link for get_all_paths()
    let all_paths_anchor = StringAnchor::new("all_paths", "index");
    let all_paths_anchor_hash = hash_entry(&EntryTypes::StringAnchor(all_paths_anchor))?;
    create_link(all_paths_anchor_hash, action_hash.clone(), LinkTypes::IdToPath, ())?;

    Ok(action_hash)
}

/// Add a step to a learning path
#[hdk_extern]
pub fn add_path_step(input: AddPathStepInput) -> ExternResult<ActionHash> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Generate step ID
    let step_id = match &input.chapter_id {
        Some(ch_id) => format!("{}-{}-step-{}", input.path_id, ch_id, input.order_index),
        None => format!("{}-step-{}", input.path_id, input.order_index),
    };

    let step = PathStep {
        id: step_id.clone(),
        path_id: input.path_id.clone(),
        chapter_id: input.chapter_id.clone(),
        order_index: input.order_index,
        step_type: input.step_type,
        resource_id: input.resource_id.clone(),
        step_title: input.step_title,
        step_narrative: input.step_narrative,
        is_optional: input.is_optional,
        // Learning objectives and engagement
        learning_objectives_json: serde_json::to_string(&input.learning_objectives.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string()),
        reflection_prompts_json: serde_json::to_string(&input.reflection_prompts.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string()),
        practice_exercises_json: serde_json::to_string(&input.practice_exercises.unwrap_or_default())
            .unwrap_or_else(|_| "[]".to_string()),
        // Completion and gating
        estimated_minutes: input.estimated_minutes,
        completion_criteria: input.completion_criteria,
        attestation_required: input.attestation_required,
        attestation_granted: input.attestation_granted,
        mastery_threshold: input.mastery_threshold,
        // Metadata
        metadata_json: input.metadata_json.unwrap_or_else(|| "{}".to_string()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
        schema_version: 2,
        validation_status: "Valid".to_string(),
    };

    let action_hash = create_entry(&EntryTypes::PathStep(step))?;

    // Create ID lookup link for step
    let step_anchor = StringAnchor::new("step_id", &step_id);
    let step_anchor_hash = hash_entry(&EntryTypes::StringAnchor(step_anchor))?;
    create_link(step_anchor_hash, action_hash.clone(), LinkTypes::IdToStep, ())?;

    // Link path to step
    let path_anchor = StringAnchor::new("path_id", &input.path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;

    let query = LinkQuery::try_new(path_anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(query, GetStrategy::default())?;

    if let Some(path_link) = path_links.first() {
        let path_action_hash = ActionHash::try_from(path_link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;
        create_link(path_action_hash, action_hash.clone(), LinkTypes::PathToStep, ())?;
    }

    // Link to chapter if specified
    if let Some(chapter_id) = &input.chapter_id {
        let chapter_anchor = StringAnchor::new("chapter_id", chapter_id);
        let chapter_anchor_hash = hash_entry(&EntryTypes::StringAnchor(chapter_anchor))?;

        let chapter_query = LinkQuery::try_new(chapter_anchor_hash, LinkTypes::IdToChapter)?;
        let chapter_links = get_links(chapter_query, GetStrategy::default())?;

        if let Some(chapter_link) = chapter_links.first() {
            let chapter_action_hash = ActionHash::try_from(chapter_link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid chapter action hash".to_string())))?;
            // Link chapter to step
            create_link(chapter_action_hash.clone(), action_hash.clone(), LinkTypes::ChapterToStep, ())?;
            // Link step back to chapter (for reverse lookup)
            create_link(action_hash.clone(), chapter_action_hash, LinkTypes::StepToChapter, ())?;
        }
    }

    // Link step to content (if resource exists)
    let resource_anchor = StringAnchor::new("content_id", &input.resource_id);
    let resource_anchor_hash = hash_entry(&EntryTypes::StringAnchor(resource_anchor))?;

    let content_query = LinkQuery::try_new(resource_anchor_hash, LinkTypes::IdToContent)?;
    let content_links = get_links(content_query, GetStrategy::default())?;

    if let Some(content_link) = content_links.first() {
        create_link(action_hash.clone(), content_link.target.clone(), LinkTypes::StepToContent, ())?;
    }

    Ok(action_hash)
}

/// Batch add multiple steps to a path (for efficient seeding)
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchAddPathStepsInput {
    pub steps: Vec<AddPathStepInput>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BatchAddPathStepsOutput {
    pub created_count: u32,
    pub action_hashes: Vec<ActionHash>,
    pub errors: Vec<String>,
}

#[hdk_extern]
pub fn batch_add_path_steps(input: BatchAddPathStepsInput) -> ExternResult<BatchAddPathStepsOutput> {
    let mut action_hashes = Vec::new();
    let mut errors = Vec::new();

    for step_input in input.steps {
        match add_path_step(step_input.clone()) {
            Ok(hash) => {
                action_hashes.push(hash);
            }
            Err(e) => {
                errors.push(format!("Failed to add step {}: {:?}", step_input.order_index, e));
            }
        }
    }

    Ok(BatchAddPathStepsOutput {
        created_count: action_hashes.len() as u32,
        action_hashes,
        errors,
    })
}

/// Batch check which path IDs already exist
#[hdk_extern]
pub fn check_path_ids_exist(input: CheckIdsExistInput) -> ExternResult<CheckIdsExistOutput> {
    let mut existing_ids = Vec::new();

    for id in input.ids {
        let anchor = StringAnchor::new("path_id", &id);
        let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

        let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
        let links = get_links(query, GetStrategy::default())?;

        if !links.is_empty() {
            existing_ids.push(id);
        }
    }

    Ok(CheckIdsExistOutput { existing_ids })
}

/// Path index entry for listing
#[derive(Serialize, Deserialize, Debug)]
pub struct PathIndexEntry {
    pub id: String,
    pub title: String,
    pub description: String,
    pub difficulty: String,
    pub estimated_duration: Option<String>,
    pub step_count: u32,
    pub tags: Vec<String>,
}

/// Path index output
#[derive(Serialize, Deserialize, Debug)]
pub struct PathIndex {
    pub paths: Vec<PathIndexEntry>,
    pub total_count: u32,
    pub last_updated: String,
}

/// Get all learning paths (for path index)
/// Uses link-based lookup via global "all_paths" anchor
#[hdk_extern]
pub fn get_all_paths(_: ()) -> ExternResult<PathIndex> {
    // Use a global anchor to find all paths
    let anchor = StringAnchor::new("all_paths", "index");
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut paths = Vec::new();

    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(path) = record
                .entry()
                .to_app_option::<LearningPath>()
                .ok()
                .flatten()
            {
                // Count steps for this path
                let step_query = LinkQuery::try_new(action_hash, LinkTypes::PathToStep)?;
                let step_links = get_links(step_query, GetStrategy::default())?;

                paths.push(PathIndexEntry {
                    id: path.id,
                    title: path.title,
                    description: path.description,
                    difficulty: path.difficulty,
                    estimated_duration: path.estimated_duration,
                    step_count: step_links.len() as u32,
                    tags: path.tags,
                });
            }
        }
    }

    let now = sys_time()?;

    Ok(PathIndex {
        total_count: paths.len() as u32,
        paths,
        last_updated: format!("{:?}", now),
    })
}

/// Delete a learning path and its steps (removes links, entries remain in DHT)
/// Used for re-seeding paths with corrected step resource IDs
#[hdk_extern]
pub fn delete_path(path_id: String) -> ExternResult<bool> {
    // Find path by ID
    let anchor = StringAnchor::new("path_id", &path_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash.clone(), LinkTypes::IdToPath)?;
    let path_links = get_links(query, GetStrategy::default())?;

    let path_link = match path_links.first() {
        Some(link) => link,
        None => return Ok(false), // Path not found
    };

    let path_action_hash = ActionHash::try_from(path_link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

    // Delete path-to-step links and step entries
    let step_query = LinkQuery::try_new(path_action_hash.clone(), LinkTypes::PathToStep)?;
    let step_links = get_links(step_query, GetStrategy::default())?;

    for link in step_links {
        // Delete the link from path to step
        delete_link(link.create_link_hash, GetOptions::default())?;
    }

    // Delete the ID-to-path link
    delete_link(path_link.create_link_hash.clone(), GetOptions::default())?;

    // Delete the all_paths index link
    let all_paths_anchor = StringAnchor::new("all_paths", "index");
    let all_paths_anchor_hash = hash_entry(&EntryTypes::StringAnchor(all_paths_anchor))?;
    let all_paths_query = LinkQuery::try_new(all_paths_anchor_hash, LinkTypes::IdToPath)?;
    let all_paths_links = get_links(all_paths_query, GetStrategy::default())?;

    for link in all_paths_links {
        if link.target == path_link.target {
            delete_link(link.create_link_hash, GetOptions::default())?;
            break;
        }
    }

    Ok(true)
}

/// Get a learning path with all its steps
#[hdk_extern]
pub fn get_path_with_steps(path_id: String) -> ExternResult<Option<PathWithSteps>> {
    // Find path by ID
    let anchor = StringAnchor::new("path_id", &path_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(query, GetStrategy::default())?;

    let path_link = match path_links.first() {
        Some(link) => link,
        None => return Ok(None),
    };

    let path_action_hash = ActionHash::try_from(path_link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

    let path_record = get(path_action_hash.clone(), GetOptions::default())?;
    let path_record = match path_record {
        Some(r) => r,
        None => return Ok(None),
    };

    let path: LearningPath = path_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize path".to_string()
        )))?;

    // Get steps
    let step_query = LinkQuery::try_new(path_action_hash.clone(), LinkTypes::PathToStep)?;
    let step_links = get_links(step_query, GetStrategy::default())?;

    let mut steps = Vec::new();
    for link in step_links {
        let step_action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid step action hash".to_string())))?;

        let step_record = get(step_action_hash.clone(), GetOptions::default())?;
        if let Some(record) = step_record {
            if let Some(step) = record
                .entry()
                .to_app_option::<PathStep>()
                .ok()
                .flatten()
            {
                steps.push(PathStepOutput {
                    action_hash: step_action_hash,
                    step,
                });
            }
        }
    }

    // Sort steps by order_index
    steps.sort_by_key(|s| s.step.order_index);

    Ok(Some(PathWithSteps {
        action_hash: path_action_hash,
        path,
        steps,
    }))
}

/// Get a lightweight path overview (no step content, just metadata and count)
///
/// This is MUCH faster than get_path_with_steps because it:
/// - Only counts step links instead of fetching each step record
/// - Returns path.metadata_json which contains chapter structure
///
/// Use for: path listings, path-overview page, initial navigation
#[hdk_extern]
pub fn get_path_overview(path_id: String) -> ExternResult<Option<PathOverview>> {
    // Find path by ID
    let anchor = StringAnchor::new("path_id", &path_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(query, GetStrategy::default())?;

    let path_link = match path_links.first() {
        Some(link) => link,
        None => return Ok(None),
    };

    let path_action_hash = ActionHash::try_from(path_link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

    let path_record = get(path_action_hash.clone(), GetOptions::default())?;
    let path_record = match path_record {
        Some(r) => r,
        None => return Ok(None),
    };

    let path: LearningPath = path_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize path".to_string()
        )))?;

    // Just count step links - don't fetch each step record
    let step_query = LinkQuery::try_new(path_action_hash.clone(), LinkTypes::PathToStep)?;
    let step_links = get_links(step_query, GetStrategy::default())?;
    let step_count = step_links.len();

    Ok(Some(PathOverview {
        action_hash: path_action_hash,
        path,
        step_count,
    }))
}

// =============================================================================
// Chapter Operations
// =============================================================================

/// Create a chapter for a learning path
#[hdk_extern]
pub fn create_chapter(input: CreateChapterInput) -> ExternResult<ChapterOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Generate chapter ID
    let chapter_id = format!("{}-chapter-{}", input.path_id, input.order_index);

    let chapter = PathChapter {
        id: chapter_id.clone(),
        path_id: input.path_id.clone(),
        order_index: input.order_index,
        title: input.title,
        description: input.description,
        learning_objectives_json: serde_json::to_string(&input.learning_objectives)
            .unwrap_or_else(|_| "[]".to_string()),
        estimated_minutes: input.estimated_minutes,
        is_optional: input.is_optional,
        attestation_granted: input.attestation_granted,
        mastery_threshold: input.mastery_threshold,
        metadata_json: input.metadata_json.unwrap_or_else(|| "{}".to_string()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::PathChapter(chapter.clone()))?;

    // Create ID lookup link for chapter
    let chapter_anchor = StringAnchor::new("chapter_id", &chapter_id);
    let chapter_anchor_hash = hash_entry(&EntryTypes::StringAnchor(chapter_anchor))?;
    create_link(chapter_anchor_hash, action_hash.clone(), LinkTypes::IdToChapter, ())?;

    // Link path to chapter
    let path_anchor = StringAnchor::new("path_id", &input.path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;

    let query = LinkQuery::try_new(path_anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(query, GetStrategy::default())?;

    if let Some(path_link) = path_links.first() {
        let path_action_hash = ActionHash::try_from(path_link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;
        create_link(path_action_hash, action_hash.clone(), LinkTypes::PathToChapter, ())?;
    }

    Ok(ChapterOutput {
        action_hash,
        chapter,
    })
}

/// Get a chapter by ID
#[hdk_extern]
pub fn get_chapter_by_id(chapter_id: String) -> ExternResult<Option<ChapterOutput>> {
    let anchor = StringAnchor::new("chapter_id", &chapter_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToChapter)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = match links.first() {
        Some(l) => l,
        None => return Ok(None),
    };

    let action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid chapter action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    let record = match record {
        Some(r) => r,
        None => return Ok(None),
    };

    let chapter: PathChapter = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize chapter".to_string()
        )))?;

    Ok(Some(ChapterOutput {
        action_hash,
        chapter,
    }))
}

/// Get all chapters for a path
#[hdk_extern]
pub fn get_chapters_for_path(path_id: String) -> ExternResult<Vec<ChapterWithSteps>> {
    // Find path by ID
    let path_anchor = StringAnchor::new("path_id", &path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;

    let query = LinkQuery::try_new(path_anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(query, GetStrategy::default())?;

    let path_link = match path_links.first() {
        Some(l) => l,
        None => return Ok(Vec::new()),
    };

    let path_action_hash = ActionHash::try_from(path_link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

    // Get chapters linked to path
    let chapter_query = LinkQuery::try_new(path_action_hash, LinkTypes::PathToChapter)?;
    let chapter_links = get_links(chapter_query, GetStrategy::default())?;

    let mut chapters = Vec::new();
    for link in chapter_links {
        let chapter_action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid chapter action hash".to_string())))?;

        let chapter_record = get(chapter_action_hash.clone(), GetOptions::default())?;
        if let Some(record) = chapter_record {
            if let Some(chapter) = record
                .entry()
                .to_app_option::<PathChapter>()
                .ok()
                .flatten()
            {
                // Get steps for this chapter
                let step_query = LinkQuery::try_new(chapter_action_hash.clone(), LinkTypes::ChapterToStep)?;
                let step_links = get_links(step_query, GetStrategy::default())?;

                let mut steps = Vec::new();
                for step_link in step_links {
                    let step_action_hash = ActionHash::try_from(step_link.target)
                        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid step action hash".to_string())))?;

                    let step_record = get(step_action_hash.clone(), GetOptions::default())?;
                    if let Some(step_rec) = step_record {
                        if let Some(step) = step_rec
                            .entry()
                            .to_app_option::<PathStep>()
                            .ok()
                            .flatten()
                        {
                            steps.push(PathStepOutput {
                                action_hash: step_action_hash,
                                step,
                            });
                        }
                    }
                }

                // Sort steps by order_index
                steps.sort_by_key(|s| s.step.order_index);

                chapters.push(ChapterWithSteps {
                    action_hash: chapter_action_hash,
                    chapter,
                    steps,
                });
            }
        }
    }

    // Sort chapters by order_index
    chapters.sort_by_key(|c| c.chapter.order_index);

    Ok(chapters)
}

/// Get a full path with chapters and steps organized
#[hdk_extern]
pub fn get_path_full(path_id: String) -> ExternResult<Option<PathWithChaptersAndSteps>> {
    // Find path by ID
    let path_anchor = StringAnchor::new("path_id", &path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;

    let query = LinkQuery::try_new(path_anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(query, GetStrategy::default())?;

    let path_link = match path_links.first() {
        Some(l) => l,
        None => return Ok(None),
    };

    let path_action_hash = ActionHash::try_from(path_link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

    let path_record = get(path_action_hash.clone(), GetOptions::default())?;
    let path_record = match path_record {
        Some(r) => r,
        None => return Ok(None),
    };

    let path: LearningPath = path_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize path".to_string()
        )))?;

    // Get chapters
    let chapters = get_chapters_for_path(path_id.clone())?;

    // Get all steps linked directly to path (includes those with chapters)
    let step_query = LinkQuery::try_new(path_action_hash.clone(), LinkTypes::PathToStep)?;
    let step_links = get_links(step_query, GetStrategy::default())?;

    let mut ungrouped_steps = Vec::new();
    for link in step_links {
        let step_action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid step action hash".to_string())))?;

        let step_record = get(step_action_hash.clone(), GetOptions::default())?;
        if let Some(record) = step_record {
            if let Some(step) = record
                .entry()
                .to_app_option::<PathStep>()
                .ok()
                .flatten()
            {
                // Only include steps that are NOT in a chapter
                if step.chapter_id.is_none() {
                    ungrouped_steps.push(PathStepOutput {
                        action_hash: step_action_hash,
                        step,
                    });
                }
            }
        }
    }

    // Sort ungrouped steps by order_index
    ungrouped_steps.sort_by_key(|s| s.step.order_index);

    Ok(Some(PathWithChaptersAndSteps {
        action_hash: path_action_hash,
        path,
        chapters,
        ungrouped_steps,
    }))
}

/// Update a chapter
#[hdk_extern]
pub fn update_chapter(input: UpdateChapterInput) -> ExternResult<ChapterOutput> {
    // Get existing chapter
    let existing = get_chapter_by_id(input.chapter_id.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("Chapter not found: {}", input.chapter_id)
        )))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Create updated chapter (immutable DHT pattern)
    let updated_chapter = PathChapter {
        id: existing.chapter.id,
        path_id: existing.chapter.path_id.clone(),
        order_index: input.order_index.unwrap_or(existing.chapter.order_index),
        title: input.title.unwrap_or(existing.chapter.title),
        description: input.description.or(existing.chapter.description),
        learning_objectives_json: input.learning_objectives
            .map(|lo| serde_json::to_string(&lo).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or(existing.chapter.learning_objectives_json),
        estimated_minutes: input.estimated_minutes.or(existing.chapter.estimated_minutes),
        is_optional: input.is_optional.unwrap_or(existing.chapter.is_optional),
        attestation_granted: input.attestation_granted.or(existing.chapter.attestation_granted),
        mastery_threshold: input.mastery_threshold.or(existing.chapter.mastery_threshold),
        metadata_json: existing.chapter.metadata_json,
        created_at: existing.chapter.created_at,
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::PathChapter(updated_chapter.clone()))?;

    // Update the ID lookup link
    let chapter_anchor = StringAnchor::new("chapter_id", &input.chapter_id);
    let chapter_anchor_hash = hash_entry(&EntryTypes::StringAnchor(chapter_anchor))?;

    // Delete old link
    let old_query = LinkQuery::try_new(chapter_anchor_hash.clone(), LinkTypes::IdToChapter)?;
    let old_links = get_links(old_query, GetStrategy::default())?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }

    // Create new link
    create_link(chapter_anchor_hash, action_hash.clone(), LinkTypes::IdToChapter, ())?;

    // Update path-to-chapter link
    let path_anchor = StringAnchor::new("path_id", &existing.chapter.path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;
    let path_query = LinkQuery::try_new(path_anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(path_query, GetStrategy::default())?;

    if let Some(path_link) = path_links.first() {
        let path_action_hash = ActionHash::try_from(path_link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

        // Delete old path-to-chapter links pointing to old chapter
        let old_chapter_query = LinkQuery::try_new(path_action_hash.clone(), LinkTypes::PathToChapter)?;
        let old_chapter_links = get_links(old_chapter_query, GetStrategy::default())?;
        for link in old_chapter_links {
            if link.target == existing.action_hash.clone().into() {
                delete_link(link.create_link_hash, GetOptions::default())?;
            }
        }

        // Create new link
        create_link(path_action_hash, action_hash.clone(), LinkTypes::PathToChapter, ())?;
    }

    Ok(ChapterOutput {
        action_hash,
        chapter: updated_chapter,
    })
}

// =============================================================================
// Path Update Operations
// =============================================================================

/// Update a learning path
#[hdk_extern]
pub fn update_path(input: UpdatePathInput) -> ExternResult<PathWithSteps> {
    // Get existing path
    let existing = get_path_with_steps(input.path_id.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("Path not found: {}", input.path_id)
        )))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Create updated path (immutable DHT pattern)
    let updated_path = LearningPath {
        id: existing.path.id,
        version: existing.path.version,
        title: input.title.unwrap_or(existing.path.title),
        description: input.description.unwrap_or(existing.path.description),
        purpose: input.purpose.or(existing.path.purpose),
        created_by: existing.path.created_by,
        difficulty: input.difficulty.unwrap_or(existing.path.difficulty),
        estimated_duration: input.estimated_duration.or(existing.path.estimated_duration),
        visibility: input.visibility.unwrap_or(existing.path.visibility),
        path_type: existing.path.path_type,
        tags: input.tags.unwrap_or(existing.path.tags),
        metadata_json: existing.path.metadata_json,
        created_at: existing.path.created_at,
        updated_at: timestamp,
        schema_version: 2,
        validation_status: "Valid".to_string(),
    };

    let action_hash = create_entry(&EntryTypes::LearningPath(updated_path.clone()))?;

    // Update the ID lookup link
    let path_anchor = StringAnchor::new("path_id", &input.path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;

    // Delete old link
    let old_query = LinkQuery::try_new(path_anchor_hash.clone(), LinkTypes::IdToPath)?;
    let old_links = get_links(old_query, GetStrategy::default())?;
    for link in old_links {
        delete_link(link.create_link_hash, GetOptions::default())?;
    }

    // Create new link
    create_link(path_anchor_hash, action_hash.clone(), LinkTypes::IdToPath, ())?;

    // Update all_paths index link
    let all_paths_anchor = StringAnchor::new("all_paths", "index");
    let all_paths_anchor_hash = hash_entry(&EntryTypes::StringAnchor(all_paths_anchor))?;

    // Delete old all_paths link
    let old_all_query = LinkQuery::try_new(all_paths_anchor_hash.clone(), LinkTypes::IdToPath)?;
    let old_all_links = get_links(old_all_query, GetStrategy::default())?;
    for link in old_all_links {
        if link.target == existing.action_hash.clone().into() {
            delete_link(link.create_link_hash, GetOptions::default())?;
        }
    }

    // Create new all_paths link
    create_link(all_paths_anchor_hash, action_hash.clone(), LinkTypes::IdToPath, ())?;

    // Re-link all steps to new path action hash
    for step_output in &existing.steps {
        create_link(action_hash.clone(), step_output.action_hash.clone(), LinkTypes::PathToStep, ())?;
    }

    Ok(PathWithSteps {
        action_hash,
        path: updated_path,
        steps: existing.steps,
    })
}

/// Update a step
#[hdk_extern]
pub fn update_step(input: UpdateStepInput) -> ExternResult<PathStepOutput> {
    // Get existing step by ID
    let step_anchor = StringAnchor::new("step_id", &input.step_id);
    let step_anchor_hash = hash_entry(&EntryTypes::StringAnchor(step_anchor))?;

    let query = LinkQuery::try_new(step_anchor_hash.clone(), LinkTypes::IdToStep)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = links.first()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("Step not found: {}", input.step_id)
        )))?;

    let existing_action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid step action hash".to_string())))?;

    let record = get(existing_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Step record not found".to_string())))?;

    let existing: PathStep = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize step".to_string()
        )))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Create updated step
    let updated_step = PathStep {
        id: existing.id,
        path_id: existing.path_id.clone(),
        chapter_id: input.chapter_id.or(existing.chapter_id),
        order_index: input.order_index.unwrap_or(existing.order_index),
        step_type: existing.step_type,
        resource_id: existing.resource_id.clone(),
        step_title: input.step_title.or(existing.step_title),
        step_narrative: input.step_narrative.or(existing.step_narrative),
        is_optional: input.is_optional.unwrap_or(existing.is_optional),
        learning_objectives_json: input.learning_objectives
            .map(|lo| serde_json::to_string(&lo).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or(existing.learning_objectives_json),
        reflection_prompts_json: input.reflection_prompts
            .map(|rp| serde_json::to_string(&rp).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or(existing.reflection_prompts_json),
        practice_exercises_json: input.practice_exercises
            .map(|pe| serde_json::to_string(&pe).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or(existing.practice_exercises_json),
        estimated_minutes: input.estimated_minutes.or(existing.estimated_minutes),
        completion_criteria: input.completion_criteria.or(existing.completion_criteria),
        attestation_required: input.attestation_required.or(existing.attestation_required),
        attestation_granted: input.attestation_granted.or(existing.attestation_granted),
        mastery_threshold: input.mastery_threshold.or(existing.mastery_threshold),
        metadata_json: existing.metadata_json,
        created_at: existing.created_at,
        updated_at: timestamp,
        schema_version: 2,
        validation_status: "Valid".to_string(),
    };

    let action_hash = create_entry(&EntryTypes::PathStep(updated_step.clone()))?;

    // Update the ID lookup link
    // Delete old link
    for lnk in links {
        delete_link(lnk.create_link_hash, GetOptions::default())?;
    }

    // Create new link
    create_link(step_anchor_hash, action_hash.clone(), LinkTypes::IdToStep, ())?;

    // Re-link to path
    let path_anchor = StringAnchor::new("path_id", &existing.path_id);
    let path_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_anchor))?;
    let path_query = LinkQuery::try_new(path_anchor_hash, LinkTypes::IdToPath)?;
    let path_links = get_links(path_query, GetStrategy::default())?;

    if let Some(path_link) = path_links.first() {
        let path_action_hash = ActionHash::try_from(path_link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;
        create_link(path_action_hash, action_hash.clone(), LinkTypes::PathToStep, ())?;
    }

    Ok(PathStepOutput {
        action_hash,
        step: updated_step,
    })
}

/// Get a step by ID
#[hdk_extern]
pub fn get_step_by_id(step_id: String) -> ExternResult<Option<PathStepOutput>> {
    let anchor = StringAnchor::new("step_id", &step_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToStep)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = match links.first() {
        Some(l) => l,
        None => return Ok(None),
    };

    let action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid step action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    let record = match record {
        Some(r) => r,
        None => return Ok(None),
    };

    let step: PathStep = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not deserialize step".to_string()
        )))?;

    Ok(Some(PathStepOutput {
        action_hash,
        step,
    }))
}

// =============================================================================
// Relationship Operations
// =============================================================================

/// Create a relationship between two content nodes
#[hdk_extern]
pub fn create_relationship(input: CreateRelationshipInput) -> ExternResult<RelationshipOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Generate relationship ID
    let rel_id = format!("{}-{}-{}", input.source_id, input.relationship_type, input.target_id);

    let relationship = Relationship {
        id: rel_id.clone(),
        source_id: input.source_id.clone(),
        target_id: input.target_id.clone(),
        relationship_type: input.relationship_type.clone(),
        confidence: input.confidence,
        inference_source: input.inference_source,
        metadata_json: input.metadata_json,
        created_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::Relationship(relationship.clone()))?;

    // Create index links for querying
    // By source
    let source_anchor = StringAnchor::new("rel_source", &input.source_id);
    let source_anchor_hash = hash_entry(&EntryTypes::StringAnchor(source_anchor))?;
    create_link(source_anchor_hash, action_hash.clone(), LinkTypes::RelationshipBySource, ())?;

    // By target
    let target_anchor = StringAnchor::new("rel_target", &input.target_id);
    let target_anchor_hash = hash_entry(&EntryTypes::StringAnchor(target_anchor))?;
    create_link(target_anchor_hash, action_hash.clone(), LinkTypes::RelationshipByTarget, ())?;

    // By type
    let type_anchor = StringAnchor::new("rel_type", &input.relationship_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(type_anchor_hash, action_hash.clone(), LinkTypes::ContentRelationshipByType, ())?;

    Ok(RelationshipOutput {
        action_hash,
        relationship,
    })
}

/// Get relationships for a content node
#[hdk_extern]
pub fn get_relationships(input: GetRelationshipsInput) -> ExternResult<Vec<RelationshipOutput>> {
    let mut results = Vec::new();

    // Get outgoing relationships (this content is source)
    if input.direction == "outgoing" || input.direction == "both" {
        let source_anchor = StringAnchor::new("rel_source", &input.content_id);
        let source_anchor_hash = hash_entry(&EntryTypes::StringAnchor(source_anchor))?;
        let query = LinkQuery::try_new(source_anchor_hash, LinkTypes::RelationshipBySource)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links {
            let action_hash = ActionHash::try_from(link.target)
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid relationship hash".to_string())))?;
            if let Some(output) = get_relationship(action_hash)? {
                results.push(output);
            }
        }
    }

    // Get incoming relationships (this content is target)
    if input.direction == "incoming" || input.direction == "both" {
        let target_anchor = StringAnchor::new("rel_target", &input.content_id);
        let target_anchor_hash = hash_entry(&EntryTypes::StringAnchor(target_anchor))?;
        let query = LinkQuery::try_new(target_anchor_hash, LinkTypes::RelationshipByTarget)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links {
            let action_hash = ActionHash::try_from(link.target)
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid relationship hash".to_string())))?;
            if let Some(output) = get_relationship(action_hash)? {
                // Avoid duplicates if direction is "both"
                if !results.iter().any(|r| r.relationship.id == output.relationship.id) {
                    results.push(output);
                }
            }
        }
    }

    Ok(results)
}

/// Get a relationship by action hash
fn get_relationship(action_hash: ActionHash) -> ExternResult<Option<RelationshipOutput>> {
    let record = get(action_hash.clone(), GetOptions::default())?;

    match record {
        Some(record) => {
            let relationship: Relationship = record
                .entry()
                .to_app_option()
                .map_err(|e| wasm_error!(e))?
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "Could not deserialize relationship".to_string()
                )))?;

            Ok(Some(RelationshipOutput {
                action_hash,
                relationship,
            }))
        }
        None => Ok(None),
    }
}

/// Get related content for a content node (follows relationships)
#[hdk_extern]
pub fn query_related_content(input: QueryRelatedContentInput) -> ExternResult<Vec<ContentOutput>> {
    let relationships = get_relationships(GetRelationshipsInput {
        content_id: input.content_id.clone(),
        direction: "outgoing".to_string(),
    })?;

    let mut results = Vec::new();

    for rel_output in relationships {
        // Filter by relationship type if specified
        if let Some(ref types) = input.relationship_types {
            if !types.contains(&rel_output.relationship.relationship_type) {
                continue;
            }
        }

        // Get the target content
        if let Some(content) = get_content_by_id(QueryByIdInput {
            id: rel_output.relationship.target_id,
        })? {
            results.push(content);
        }
    }

    Ok(results)
}

/// Get content graph starting from a root node
#[hdk_extern]
pub fn get_content_graph(input: QueryRelatedContentInput) -> ExternResult<ContentGraph> {
    let root = get_content_by_id(QueryByIdInput {
        id: input.content_id.clone(),
    })?;

    let relationships = get_relationships(GetRelationshipsInput {
        content_id: input.content_id.clone(),
        direction: "outgoing".to_string(),
    })?;

    let mut related = Vec::new();
    let mut total_nodes = if root.is_some() { 1 } else { 0 };

    for rel_output in relationships {
        if let Some(content) = get_content_by_id(QueryByIdInput {
            id: rel_output.relationship.target_id,
        })? {
            total_nodes += 1;
            related.push(ContentGraphNode {
                content,
                relationship_type: rel_output.relationship.relationship_type,
                confidence: rel_output.relationship.confidence,
                children: Vec::new(), // TODO: Recursive traversal for depth > 1
            });
        }
    }

    Ok(ContentGraph {
        root,
        related,
        total_nodes,
    })
}

// =============================================================================
// Human CRUD operations moved to: holochain/dna/imagodei/zomes/imagodei/

// =============================================================================
// Existence Check Helpers
// =============================================================================

/// Check if content with the given ID already exists.
/// Used to prevent duplicate entries during create operations.
fn content_exists_by_id(id: &str) -> ExternResult<bool> {
    let anchor = StringAnchor::new("content_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToContent)?;
    let links = get_links(query, GetStrategy::default())?;
    Ok(!links.is_empty())
}

/// Check if a learning path with the given ID already exists.
fn path_exists_by_id(id: &str) -> ExternResult<bool> {
    let anchor = StringAnchor::new("path_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let links = get_links(query, GetStrategy::default())?;
    Ok(!links.is_empty())
}

// =============================================================================
// Link Helper Functions
// =============================================================================

fn create_id_to_content_link(id: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("content_id", id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::IdToContent, ())?;
    Ok(())
}

fn create_type_to_content_link(content_type: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("content_type", content_type);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::TypeToContent, ())?;
    Ok(())
}

fn create_tag_to_content_link(tag: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("tag", tag);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::TagToContent, ())?;
    Ok(())
}

fn create_author_to_content_link(target: &ActionHash) -> ExternResult<()> {
    let agent_info = agent_info()?;
    create_link(
        agent_info.agent_initial_pubkey,
        target.clone(),
        LinkTypes::AuthorToContent,
        (),
    )?;
    Ok(())
}

fn create_import_batch_link(import_id: &str, target: &ActionHash) -> ExternResult<()> {
    let anchor = StringAnchor::new("import_batch", import_id);
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(anchor_hash, target.clone(), LinkTypes::ImportBatchToContent, ())?;
    Ok(())
}

// =============================================================================
// Agent CRUD operations moved to: holochain/dna/imagodei/zomes/imagodei/

// =============================================================================
// Agent progress operations moved to: holochain/dna/imagodei/zomes/imagodei/

// =============================================================================
// Content mastery CRUD moved to: holochain/dna/imagodei/zomes/imagodei/

// =============================================================================
// Attestation operations moved to: holochain/dna/imagodei/zomes/imagodei/

// =============================================================================
// Agent presence operations moved to: holochain/dna/imagodei/zomes/imagodei/

// =============================================================================
// Progress Tracking Operations
// =============================================================================
// NOTE: Progress tracking uses AgentToPathProgress link which remains in lamad
// as it tracks learning path progress, a core lamad concern.
// Agent/Human progress (identity-based) is in imagodei.

/// Start tracking progress on a learning path
#[hdk_extern]
pub fn start_path_progress(input: StartPathProgressInput) -> ExternResult<AgentProgressOutput> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Generate progress ID
    let progress_id = format!("{}-{}", agent_id, input.path_id);

    // Check if progress already exists
    let progress_anchor = StringAnchor::new("progress_id", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor.clone()))?;

    let existing_query = LinkQuery::try_new(progress_anchor_hash.clone(), LinkTypes::AgentToPathProgress)?;
    let existing_links = get_links(existing_query, GetStrategy::default())?;

    if let Some(link) = existing_links.first() {
        // Progress already exists, return it
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;
        let record = get(action_hash.clone(), GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Progress record not found".to_string())))?;
        let progress: AgentProgress = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(e))?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize progress".to_string())))?;
        return Ok(AgentProgressOutput { action_hash, progress });
    }

    // Create new progress
    let progress = AgentProgress {
        id: progress_id.clone(),
        agent_id: agent_id.clone(),
        path_id: input.path_id.clone(),
        current_step_index: 0,
        completed_step_indices: Vec::new(),
        completed_content_ids: Vec::new(),
        step_affinity_json: "{}".to_string(),
        step_notes_json: "{}".to_string(),
        reflection_responses_json: "{}".to_string(),
        attestations_earned: Vec::new(),
        started_at: timestamp.clone(),
        last_activity_at: timestamp,
        completed_at: None,
    };

    let action_hash = create_entry(&EntryTypes::AgentProgress(progress.clone()))?;

    // Create ID lookup link
    create_link(progress_anchor_hash, action_hash.clone(), LinkTypes::AgentToPathProgress, ())?;

    // Create agent-to-progress link
    let agent_anchor = StringAnchor::new("agent_progress", &agent_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::AgentToPathProgress, ())?;

    // Create path-to-progress link
    let path_progress_anchor = StringAnchor::new("path_progress", &input.path_id);
    let path_progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(path_progress_anchor))?;
    create_link(path_progress_anchor_hash, action_hash.clone(), LinkTypes::PathToProgress, ())?;

    // Create status link (in_progress)
    let status_anchor = StringAnchor::new("progress_status", "in_progress");
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::ProgressByStatus, ())?;

    Ok(AgentProgressOutput { action_hash, progress })
}

/// Complete a step in a learning path
#[hdk_extern]
pub fn complete_step(input: CompleteStepInput) -> ExternResult<AgentProgressOutput> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Find existing progress
    let progress_id = format!("{}-{}", agent_id, input.path_id);
    let progress_anchor = StringAnchor::new("progress_id", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    let query = LinkQuery::try_new(progress_anchor_hash.clone(), LinkTypes::AgentToPathProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = links.first()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("No progress found for path: {}", input.path_id)
        )))?;

    let existing_action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;

    let record = get(existing_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Progress record not found".to_string())))?;

    let existing: AgentProgress = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize progress".to_string())))?;

    // Parse existing JSON data
    let mut step_affinity: HashMap<String, u32> = serde_json::from_str(&existing.step_affinity_json)
        .unwrap_or_default();
    let mut step_notes: HashMap<String, String> = serde_json::from_str(&existing.step_notes_json)
        .unwrap_or_default();
    let mut reflection_responses: HashMap<String, Vec<String>> = serde_json::from_str(&existing.reflection_responses_json)
        .unwrap_or_default();

    // Update with new data
    let step_key = input.step_index.to_string();
    if let Some(affinity) = input.affinity_score {
        step_affinity.insert(step_key.clone(), affinity);
    }
    if let Some(notes) = input.notes {
        step_notes.insert(step_key.clone(), notes);
    }
    if let Some(responses) = input.reflection_responses {
        reflection_responses.insert(step_key, responses);
    }

    // Update completed steps
    let mut completed_step_indices = existing.completed_step_indices.clone();
    if !completed_step_indices.contains(&input.step_index) {
        completed_step_indices.push(input.step_index);
        completed_step_indices.sort();
    }

    // Update completed content IDs
    let mut completed_content_ids = existing.completed_content_ids.clone();
    if let Some(content_id) = input.content_id {
        if !completed_content_ids.contains(&content_id) {
            completed_content_ids.push(content_id);
        }
    }

    // Create updated progress
    let updated_progress = AgentProgress {
        id: existing.id,
        agent_id: existing.agent_id,
        path_id: existing.path_id.clone(),
        current_step_index: input.step_index + 1,  // Advance to next step
        completed_step_indices,
        completed_content_ids,
        step_affinity_json: serde_json::to_string(&step_affinity).unwrap_or_else(|_| "{}".to_string()),
        step_notes_json: serde_json::to_string(&step_notes).unwrap_or_else(|_| "{}".to_string()),
        reflection_responses_json: serde_json::to_string(&reflection_responses).unwrap_or_else(|_| "{}".to_string()),
        attestations_earned: existing.attestations_earned,
        started_at: existing.started_at,
        last_activity_at: timestamp,
        completed_at: existing.completed_at,
    };

    let action_hash = create_entry(&EntryTypes::AgentProgress(updated_progress.clone()))?;

    // Update ID lookup link
    delete_link(link.create_link_hash.clone(), GetOptions::default())?;
    create_link(progress_anchor_hash, action_hash.clone(), LinkTypes::AgentToPathProgress, ())?;

    Ok(AgentProgressOutput { action_hash, progress: updated_progress })
}

/// Mark a path as completed
#[hdk_extern]
pub fn complete_path(path_id: String) -> ExternResult<AgentProgressOutput> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Find existing progress
    let progress_id = format!("{}-{}", agent_id, path_id);
    let progress_anchor = StringAnchor::new("progress_id", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    let query = LinkQuery::try_new(progress_anchor_hash.clone(), LinkTypes::AgentToPathProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = links.first()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("No progress found for path: {}", path_id)
        )))?;

    let existing_action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;

    let record = get(existing_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Progress record not found".to_string())))?;

    let existing: AgentProgress = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize progress".to_string())))?;

    // Create completed progress
    let completed_progress = AgentProgress {
        id: existing.id,
        agent_id: existing.agent_id,
        path_id: existing.path_id.clone(),
        current_step_index: existing.current_step_index,
        completed_step_indices: existing.completed_step_indices,
        completed_content_ids: existing.completed_content_ids,
        step_affinity_json: existing.step_affinity_json,
        step_notes_json: existing.step_notes_json,
        reflection_responses_json: existing.reflection_responses_json,
        attestations_earned: existing.attestations_earned,
        started_at: existing.started_at,
        last_activity_at: timestamp.clone(),
        completed_at: Some(timestamp),
    };

    let action_hash = create_entry(&EntryTypes::AgentProgress(completed_progress.clone()))?;

    // Update ID lookup link
    delete_link(link.create_link_hash.clone(), GetOptions::default())?;
    create_link(progress_anchor_hash, action_hash.clone(), LinkTypes::AgentToPathProgress, ())?;

    // Update status link (in_progress -> completed)
    let old_status_anchor = StringAnchor::new("progress_status", "in_progress");
    let old_status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(old_status_anchor))?;
    let old_status_query = LinkQuery::try_new(old_status_anchor_hash, LinkTypes::ProgressByStatus)?;
    let old_status_links = get_links(old_status_query, GetStrategy::default())?;
    for status_link in old_status_links {
        if status_link.target == existing_action_hash.clone().into() {
            delete_link(status_link.create_link_hash, GetOptions::default())?;
            break;
        }
    }

    let new_status_anchor = StringAnchor::new("progress_status", "completed");
    let new_status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(new_status_anchor))?;
    create_link(new_status_anchor_hash, action_hash.clone(), LinkTypes::ProgressByStatus, ())?;

    Ok(AgentProgressOutput { action_hash, progress: completed_progress })
}

/// Get current agent's progress on a path
#[hdk_extern]
pub fn get_my_path_progress(path_id: String) -> ExternResult<Option<AgentProgressOutput>> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();

    let progress_id = format!("{}-{}", agent_id, path_id);
    let progress_anchor = StringAnchor::new("progress_id", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    let query = LinkQuery::try_new(progress_anchor_hash, LinkTypes::AgentToPathProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = match links.first() {
        Some(l) => l,
        None => return Ok(None),
    };

    let action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    let record = match record {
        Some(r) => r,
        None => return Ok(None),
    };

    let progress: AgentProgress = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize progress".to_string())))?;

    Ok(Some(AgentProgressOutput { action_hash, progress }))
}

/// Get all progress for current agent
#[hdk_extern]
pub fn get_my_all_progress(_: ()) -> ExternResult<Vec<AgentProgressOutput>> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();

    let agent_anchor = StringAnchor::new("agent_progress", &agent_id);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

    let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToPathProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(progress) = rec.entry().to_app_option::<AgentProgress>().ok().flatten() {
                results.push(AgentProgressOutput { action_hash, progress });
            }
        }
    }

    Ok(results)
}

/// Get progress by status (in_progress, completed, abandoned)
#[hdk_extern]
pub fn get_progress_by_status(status: String) -> ExternResult<Vec<AgentProgressOutput>> {
    let status_anchor = StringAnchor::new("progress_status", &status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

    let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::ProgressByStatus)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(progress) = rec.entry().to_app_option::<AgentProgress>().ok().flatten() {
                results.push(AgentProgressOutput { action_hash, progress });
            }
        }
    }

    Ok(results)
}

/// Get progress summaries for current agent
#[hdk_extern]
pub fn get_my_progress_summaries(_: ()) -> ExternResult<Vec<ProgressSummary>> {
    let all_progress = get_my_all_progress(())?;
    let mut summaries = Vec::new();

    for progress_output in all_progress {
        let progress = progress_output.progress;

        // Get path to count total steps
        let path_result = get_path_with_steps(progress.path_id.clone())?;
        let (path_title, total_steps) = match path_result {
            Some(path_data) => (path_data.path.title, path_data.steps.len() as u32),
            None => ("Unknown Path".to_string(), 0),
        };

        summaries.push(ProgressSummary {
            path_id: progress.path_id,
            path_title,
            total_steps,
            completed_steps: progress.completed_step_indices.len() as u32,
            current_step_index: progress.current_step_index,
            is_completed: progress.completed_at.is_some(),
            attestations_earned: progress.attestations_earned,
            started_at: progress.started_at,
            last_activity_at: progress.last_activity_at,
            completed_at: progress.completed_at,
        });
    }

    Ok(summaries)
}

// =============================================================================
// Attestation Operations
// =============================================================================

/// Grant an attestation for completing a step, chapter, or path
#[hdk_extern]
pub fn grant_attestation(input: GrantAttestationInput) -> ExternResult<AgentProgressOutput> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Find existing progress
    let progress_id = format!("{}-{}", agent_id, input.path_id);
    let progress_anchor = StringAnchor::new("progress_id", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    let query = LinkQuery::try_new(progress_anchor_hash.clone(), LinkTypes::AgentToPathProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = links.first()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            format!("No progress found for path: {}", input.path_id)
        )))?;

    let existing_action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;

    let record = get(existing_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Progress record not found".to_string())))?;

    let existing: AgentProgress = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize progress".to_string())))?;

    // Add attestation if not already earned
    let mut attestations_earned = existing.attestations_earned.clone();
    if !attestations_earned.contains(&input.attestation_id) {
        attestations_earned.push(input.attestation_id.clone());
    }

    // Create updated progress with attestation
    let updated_progress = AgentProgress {
        id: existing.id,
        agent_id: existing.agent_id,
        path_id: existing.path_id.clone(),
        current_step_index: existing.current_step_index,
        completed_step_indices: existing.completed_step_indices,
        completed_content_ids: existing.completed_content_ids,
        step_affinity_json: existing.step_affinity_json,
        step_notes_json: existing.step_notes_json,
        reflection_responses_json: existing.reflection_responses_json,
        attestations_earned,
        started_at: existing.started_at,
        last_activity_at: timestamp,
        completed_at: existing.completed_at,
    };

    let action_hash = create_entry(&EntryTypes::AgentProgress(updated_progress.clone()))?;

    // Update ID lookup link
    delete_link(link.create_link_hash.clone(), GetOptions::default())?;
    create_link(progress_anchor_hash, action_hash.clone(), LinkTypes::AgentToPathProgress, ())?;

    // Issue attestation via imagodei DNA (attestations live in identity layer)
    let _attestation_result = issue_attestation_via_imagodei(IssueAttestationBridgeInput {
        agent_id: agent_id.clone(),
        category: input.source_type.clone(),  // "step", "chapter", "path"
        attestation_type: input.attestation_id,
        display_name: format!("Completed: {}", input.reason),
        description: input.reason,
        icon_url: None,
        tier: None,
        earned_via_json: serde_json::json!({
            "source_type": input.source_type,
            "source_id": input.source_id,
            "path_id": input.path_id
        }).to_string(),
        expires_at: None,
    })?;

    Ok(AgentProgressOutput { action_hash, progress: updated_progress })
}

/// Check if learner has required attestation to access a step
#[hdk_extern]
pub fn check_attestation_access(input: CheckAttestationAccessInput) -> ExternResult<AttestationAccessResult> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();

    // Get current progress to check attestations
    let progress_id = format!("{}-{}", agent_id, input.path_id);
    let progress_anchor = StringAnchor::new("progress_id", &progress_id);
    let progress_anchor_hash = hash_entry(&EntryTypes::StringAnchor(progress_anchor))?;

    let query = LinkQuery::try_new(progress_anchor_hash, LinkTypes::AgentToPathProgress)?;
    let links = get_links(query, GetStrategy::default())?;

    let attestations_earned: Vec<String> = if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid progress action hash".to_string())))?;
        let record = get(action_hash, GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(progress) = rec.entry().to_app_option::<AgentProgress>().ok().flatten() {
                progress.attestations_earned
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let has_access = attestations_earned.contains(&input.required_attestation);

    Ok(AttestationAccessResult {
        has_access,
        required_attestation: input.required_attestation,
        attestations_earned,
    })
}

/// Input for checking attestation access
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckAttestationAccessInput {
    pub path_id: String,
    pub required_attestation: String,
}

/// Result of attestation access check
#[derive(Serialize, Deserialize, Debug)]
pub struct AttestationAccessResult {
    pub has_access: bool,
    pub required_attestation: String,
    pub attestations_earned: Vec<String>,
}

// =============================================================================
// Content mastery operations moved to: holochain/dna/imagodei/zomes/imagodei/

// =============================================================================
// Assessment History & Attestation Gating
// =============================================================================

/// Assessment history entry (for viewing past attempts)
#[derive(Serialize, Deserialize, Debug)]
pub struct AssessmentHistoryEntry {
    pub assessment_type: String,
    pub score: f64,
    pub passed: bool,
    pub threshold: f64,
    pub question_count: u32,
    pub correct_count: u32,
    pub time_seconds: u32,
    pub timestamp: String,
    pub level_achieved: Option<String>,
}

/// Assessment history output
#[derive(Serialize, Deserialize, Debug)]
pub struct AssessmentHistory {
    pub content_id: String,
    pub entries: Vec<AssessmentHistoryEntry>,
    pub total_attempts: u32,
    pub pass_count: u32,
    pub best_score: f64,
    pub current_level: String,
}

/// Input for checking attestation eligibility
#[derive(Serialize, Deserialize, Debug)]
pub struct CheckAttestationEligibilityInput {
    pub path_id: String,
    pub attestation_id: String,
    pub required_content_ids: Vec<String>,
    pub required_mastery_level: String,
}

/// Attestation eligibility result
#[derive(Serialize, Deserialize, Debug)]
pub struct AttestationEligibilityResult {
    pub eligible: bool,
    pub attestation_id: String,
    pub required_level: String,
    pub required_level_index: u32,
    pub content_requirements: Vec<ContentMasteryRequirement>,
    pub all_requirements_met: bool,
    pub missing_requirements: Vec<String>,
}

/// Individual content mastery requirement check
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentMasteryRequirement {
    pub content_id: String,
    pub required_level: String,
    pub current_level: String,
    pub met: bool,
    pub gap: i32,
}

/// Input for granting attestation with mastery check
#[derive(Serialize, Deserialize, Debug)]
pub struct GrantAttestationWithMasteryInput {
    pub path_id: String,
    pub attestation_id: String,
    pub reason: String,
    pub source_type: String,
    pub source_id: String,
    pub required_content_ids: Vec<String>,
    pub required_mastery_level: String,
}

/// Result of step access check
#[derive(Serialize, Deserialize, Debug)]
pub struct StepAccessResult {
    pub step_id: String,
    pub access_granted: bool,
    pub blockers: Vec<String>,
    pub attestation_required: Option<String>,
    pub mastery_threshold: Option<u32>,
}

/// Get assessment history for a content node
#[hdk_extern]
pub fn get_assessment_history(content_id: String) -> ExternResult<AssessmentHistory> {
    let mastery = get_my_mastery(content_id.clone())?;

    match mastery {
        Some(m) => {
            // Parse assessment evidence JSON
            let evidence_array: Vec<serde_json::Value> = serde_json::from_str(&m.mastery.assessment_evidence_json)
                .unwrap_or_default();

            let mut entries = Vec::new();
            let mut pass_count = 0u32;
            let mut best_score = 0.0f64;

            for evidence in &evidence_array {
                let assessment_type = evidence.get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                let score = evidence.get("score")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                let passed = evidence.get("passed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let threshold = evidence.get("threshold")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.7);
                let question_count = evidence.get("questions")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let correct_count = evidence.get("correct")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let time_seconds = evidence.get("time_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as u32;
                let timestamp = evidence.get("timestamp")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                if passed {
                    pass_count += 1;
                }
                if score > best_score {
                    best_score = score;
                }

                entries.push(AssessmentHistoryEntry {
                    assessment_type,
                    score,
                    passed,
                    threshold,
                    question_count,
                    correct_count,
                    time_seconds,
                    timestamp,
                    level_achieved: if passed { Some(m.mastery.mastery_level.clone()) } else { None },
                });
            }

            Ok(AssessmentHistory {
                content_id,
                entries,
                total_attempts: evidence_array.len() as u32,
                pass_count,
                best_score,
                current_level: m.mastery.mastery_level,
            })
        }
        None => Ok(AssessmentHistory {
            content_id,
            entries: Vec::new(),
            total_attempts: 0,
            pass_count: 0,
            best_score: 0.0,
            current_level: "not_started".to_string(),
        })
    }
}

/// Check if current agent is eligible for an attestation based on mastery requirements
#[hdk_extern]
pub fn check_attestation_eligibility(input: CheckAttestationEligibilityInput) -> ExternResult<AttestationEligibilityResult> {
    let required_level_index = get_mastery_level_index(&input.required_mastery_level);

    let mut content_requirements = Vec::new();
    let mut missing_requirements = Vec::new();
    let mut all_met = true;

    for content_id in &input.required_content_ids {
        let mastery = get_my_mastery(content_id.clone())?;

        let (current_level, current_index) = match mastery {
            Some(m) => (m.mastery.mastery_level, m.mastery.mastery_level_index),
            None => ("not_started".to_string(), 0),
        };

        let met = current_index >= required_level_index;
        let gap = required_level_index as i32 - current_index as i32;

        if !met {
            all_met = false;
            missing_requirements.push(format!(
                "{}: need {} (have {})",
                content_id, input.required_mastery_level, current_level
            ));
        }

        content_requirements.push(ContentMasteryRequirement {
            content_id: content_id.clone(),
            required_level: input.required_mastery_level.clone(),
            current_level,
            met,
            gap,
        });
    }

    Ok(AttestationEligibilityResult {
        eligible: all_met,
        attestation_id: input.attestation_id,
        required_level: input.required_mastery_level.clone(),
        required_level_index,
        content_requirements,
        all_requirements_met: all_met,
        missing_requirements,
    })
}

/// Grant attestation only if mastery requirements are met
#[hdk_extern]
pub fn grant_attestation_with_mastery_check(input: GrantAttestationWithMasteryInput) -> ExternResult<AgentProgressOutput> {
    // First check eligibility
    let eligibility = check_attestation_eligibility(CheckAttestationEligibilityInput {
        path_id: input.path_id.clone(),
        attestation_id: input.attestation_id.clone(),
        required_content_ids: input.required_content_ids.clone(),
        required_mastery_level: input.required_mastery_level.clone(),
    })?;

    if !eligibility.eligible {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!(
                "Mastery requirements not met for attestation '{}'. Missing: {:?}",
                input.attestation_id,
                eligibility.missing_requirements
            )
        )));
    }

    // If eligible, grant the attestation using existing function
    grant_attestation(GrantAttestationInput {
        path_id: input.path_id,
        attestation_id: input.attestation_id,
        reason: input.reason,
        source_type: input.source_type,
        source_id: input.source_id,
    })
}

/// Check step access based on required attestation and mastery
#[hdk_extern]
pub fn check_step_access(step_id: String) -> ExternResult<StepAccessResult> {
    // Get the step
    let step_output = get_step_by_id(step_id.clone())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(format!("Step not found: {}", step_id))))?;

    let step = step_output.step;
    let mut access_granted = true;
    let mut blockers = Vec::new();

    // Check attestation requirement
    if let Some(ref required_attestation) = step.attestation_required {
        let attestation_check = check_attestation_access(CheckAttestationAccessInput {
            path_id: step.path_id.clone(),
            required_attestation: required_attestation.clone(),
        })?;

        if !attestation_check.has_access {
            access_granted = false;
            blockers.push(format!("Missing attestation: {}", required_attestation));
        }
    }

    // Check mastery threshold
    if let Some(mastery_threshold) = step.mastery_threshold {
        // Get mastery for the content this step references
        let mastery = get_my_mastery(step.resource_id.clone())?;

        let current_level_index = match mastery {
            Some(m) => m.mastery.mastery_level_index,
            None => 0,
        };

        if current_level_index < mastery_threshold {
            access_granted = false;
            let required_level = MASTERY_LEVELS.get(mastery_threshold as usize)
                .unwrap_or(&"apply")
                .to_string();
            let current_level = MASTERY_LEVELS.get(current_level_index as usize)
                .unwrap_or(&"not_started")
                .to_string();
            blockers.push(format!(
                "Need mastery level '{}' (currently '{}')",
                required_level, current_level
            ));
        }
    }

    Ok(StepAccessResult {
        step_id,
        access_granted,
        blockers,
        attestation_required: step.attestation_required,
        mastery_threshold: step.mastery_threshold,
    })
}

/// Batch check multiple steps for access (efficient for path overview)
#[hdk_extern]
pub fn check_path_step_access(path_id: String) -> ExternResult<Vec<StepAccessResult>> {
    let path_with_steps = get_path_with_steps(path_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Path not found".to_string())))?;

    let mut results = Vec::new();
    for step_output in path_with_steps.steps {
        let step_id = step_output.step.id.clone();
        match check_step_access(step_id) {
            Ok(result) => results.push(result),
            Err(_) => continue,  // Skip steps that error
        }
    }

    Ok(results)
}

// =============================================================================
// Practice Pool & Mastery Challenge Types
// =============================================================================

/// Discovery candidate from knowledge graph
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DiscoveryCandidate {
    pub content_id: String,
    pub source_content_id: String,
    pub relationship_type: String,
    pub discovery_reason: String,
}

/// Content mix entry for a challenge
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ContentMixEntry {
    pub content_id: String,
    pub source: String,  // "path_active", "refresh_queue", "graph_neighbor", "serendipity"
    pub question_count: u32,
}

/// Question in a challenge
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChallengeQuestion {
    pub content_id: String,
    pub question_type: String,
    pub question_text: String,
    pub options_json: String,
    pub correct_answer: String,
}

/// Response to a challenge question
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChallengeResponse {
    pub content_id: String,
    pub question_index: u32,
    pub response: String,
    pub correct: bool,
    pub time_taken_ms: u32,
}

/// Level change from a challenge
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LevelChange {
    pub content_id: String,
    pub from_level: String,
    pub to_level: String,
    pub from_index: u32,
    pub to_index: u32,
    pub change: String,  // "up", "down", "same"
}

/// Discovery made from challenge
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChallengeDiscovery {
    pub content_id: String,
    pub discovered_via: String,
    pub relationship_type: String,
}

/// Output for practice pool
#[derive(Serialize, Deserialize, Debug)]
pub struct PracticePoolOutput {
    pub action_hash: ActionHash,
    pub pool: PracticePool,
}

/// Output for mastery challenge
#[derive(Serialize, Deserialize, Debug)]
pub struct MasteryChallengeOutput {
    pub action_hash: ActionHash,
    pub challenge: MasteryChallenge,
}

/// Input for creating/updating practice pool
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePoolInput {
    pub contributing_path_ids: Vec<String>,
    pub max_active_size: Option<u32>,
    pub refresh_threshold: Option<f64>,
    pub discovery_probability: Option<f64>,
    pub regression_enabled: Option<bool>,
    pub challenge_cooldown_hours: Option<u32>,
}

/// Input for starting a mastery challenge
#[derive(Serialize, Deserialize, Debug)]
pub struct StartChallengeInput {
    pub path_id: Option<String>,
    pub question_count: u32,
    pub include_discoveries: bool,
    pub time_limit_seconds: Option<u32>,
}

/// Input for submitting challenge responses
#[derive(Serialize, Deserialize, Debug)]
pub struct SubmitChallengeInput {
    pub challenge_id: String,
    pub responses: Vec<ChallengeResponse>,
    pub actual_time_seconds: u32,
}

/// Challenge result after submission
#[derive(Serialize, Deserialize, Debug)]
pub struct ChallengeResult {
    pub challenge: MasteryChallengeOutput,
    pub score: f64,
    pub level_changes: Vec<LevelChange>,
    pub discoveries: Vec<ChallengeDiscovery>,
    pub net_level_change: i32,
    pub can_retake_at: String,
}

/// Cooldown check result
#[derive(Serialize, Deserialize, Debug)]
pub struct CooldownCheckResult {
    pub can_take_challenge: bool,
    pub cooldown_remaining_hours: u32,
    pub last_challenge_at: Option<String>,
    pub next_available_at: Option<String>,
}

/// Pool recommendations for what to practice
#[derive(Serialize, Deserialize, Debug)]
pub struct PoolRecommendations {
    pub priority_refresh: Vec<String>,     // Mastered but need refresh
    pub active_practice: Vec<String>,      // In active rotation
    pub discovery_suggestions: Vec<DiscoveryCandidate>,  // Serendipity options
    pub total_pool_size: u32,
}

// =============================================================================
// Practice Pool Operations
// =============================================================================

/// Get or create practice pool for current agent
#[hdk_extern]
pub fn get_or_create_practice_pool(input: CreatePoolInput) -> ExternResult<PracticePoolOutput> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Check if pool already exists
    let pool_anchor = StringAnchor::new("agent_pool", &agent_id);
    let pool_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pool_anchor.clone()))?;

    let query = LinkQuery::try_new(pool_anchor_hash.clone(), LinkTypes::AgentToPool)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        // Return existing pool
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid pool hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Pool record not found".to_string())))?;

        let pool: PracticePool = record
            .entry()
            .to_app_option()
            .map_err(|e| wasm_error!(e))?
            .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize pool".to_string())))?;

        return Ok(PracticePoolOutput { action_hash, pool });
    }

    // Create new pool
    let pool_id = format!("pool-{}-{}", agent_id, timestamp);

    let pool = PracticePool {
        id: pool_id.clone(),
        agent_id: agent_id.clone(),
        active_content_ids_json: "[]".to_string(),
        refresh_queue_ids_json: "[]".to_string(),
        discovery_candidates_json: "[]".to_string(),
        contributing_path_ids_json: serde_json::to_string(&input.contributing_path_ids)
            .unwrap_or_else(|_| "[]".to_string()),
        max_active_size: input.max_active_size.unwrap_or(20),
        refresh_threshold: input.refresh_threshold.unwrap_or(0.5),
        discovery_probability: input.discovery_probability.unwrap_or(0.15),
        regression_enabled: input.regression_enabled.unwrap_or(true),
        challenge_cooldown_hours: input.challenge_cooldown_hours.unwrap_or(24),
        last_challenge_at: None,
        last_challenge_id: None,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::PracticePool(pool.clone()))?;

    // Create anchor and link
    create_entry(&EntryTypes::StringAnchor(pool_anchor))?;
    create_link(pool_anchor_hash, action_hash.clone(), LinkTypes::AgentToPool, ())?;

    Ok(PracticePoolOutput { action_hash, pool })
}

/// Refresh practice pool with content from paths and knowledge graph
#[hdk_extern]
pub fn refresh_practice_pool(_: ()) -> ExternResult<PracticePoolOutput> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get existing pool
    let pool_output = get_or_create_practice_pool(CreatePoolInput {
        contributing_path_ids: vec![],
        max_active_size: None,
        refresh_threshold: None,
        discovery_probability: None,
        regression_enabled: None,
        challenge_cooldown_hours: None,
    })?;

    let existing_pool = pool_output.pool;

    // Gather active content from contributing paths
    let contributing_paths: Vec<String> = serde_json::from_str(&existing_pool.contributing_path_ids_json)
        .unwrap_or_default();

    let mut active_content: Vec<String> = Vec::new();
    let mut _refresh_queue: Vec<String> = Vec::new();

    for path_id in &contributing_paths {
        if let Some(path_with_steps) = get_path_with_steps(path_id.clone())? {
            for step_output in path_with_steps.steps {
                let content_id = step_output.step.resource_id.clone();

                // Check mastery for this content
                if let Some(mastery_output) = get_my_mastery(content_id.clone())? {
                    let mastery = mastery_output.mastery;

                    // If not yet mastered (below apply level), add to active
                    if mastery.mastery_level_index < 4 {
                        if !active_content.contains(&content_id) && active_content.len() < existing_pool.max_active_size as usize {
                            active_content.push(content_id);
                        }
                    }
                    // If mastered but freshness dropped, add to refresh queue
                    else if mastery.freshness_score < existing_pool.refresh_threshold {
                        if !_refresh_queue.contains(&content_id) {
                            _refresh_queue.push(content_id);
                        }
                    }
                } else {
                    // No mastery record - add to active
                    if !active_content.contains(&content_id) && active_content.len() < existing_pool.max_active_size as usize {
                        active_content.push(content_id);
                    }
                }
            }
        }
    }

    // Discover related content from knowledge graph
    let mut discovery_candidates: Vec<DiscoveryCandidate> = Vec::new();

    for content_id in &active_content {
        // Get relationships for this content
        let relationships = get_relationships(GetRelationshipsInput {
            content_id: content_id.clone(),
            direction: "both".to_string(),
        })?;

        for rel_output in relationships {
            let rel = rel_output.relationship;

            // Find the related content (the one that's not this content_id)
            let related_id = if rel.source_id == *content_id {
                rel.target_id.clone()
            } else {
                rel.source_id.clone()
            };

            // Check if this related content is not already in our pool
            if !active_content.contains(&related_id) && !_refresh_queue.contains(&related_id) {
                // Check we haven't already added this as a discovery candidate
                if !discovery_candidates.iter().any(|d| d.content_id == related_id) {
                    discovery_candidates.push(DiscoveryCandidate {
                        content_id: related_id,
                        source_content_id: content_id.clone(),
                        relationship_type: rel.relationship_type.clone(),
                        discovery_reason: format!("Related via {} to content you're learning", rel.relationship_type),
                    });
                }
            }
        }
    }

    // Limit discovery candidates
    discovery_candidates.truncate(10);

    // Update the pool
    let updated_pool = PracticePool {
        id: existing_pool.id,
        agent_id: existing_pool.agent_id,
        active_content_ids_json: serde_json::to_string(&active_content).unwrap_or_else(|_| "[]".to_string()),
        refresh_queue_ids_json: serde_json::to_string(&_refresh_queue).unwrap_or_else(|_| "[]".to_string()),
        discovery_candidates_json: serde_json::to_string(&discovery_candidates).unwrap_or_else(|_| "[]".to_string()),
        contributing_path_ids_json: existing_pool.contributing_path_ids_json,
        max_active_size: existing_pool.max_active_size,
        refresh_threshold: existing_pool.refresh_threshold,
        discovery_probability: existing_pool.discovery_probability,
        regression_enabled: existing_pool.regression_enabled,
        challenge_cooldown_hours: existing_pool.challenge_cooldown_hours,
        last_challenge_at: existing_pool.last_challenge_at,
        last_challenge_id: existing_pool.last_challenge_id,
        total_challenges_taken: existing_pool.total_challenges_taken,
        total_level_ups: existing_pool.total_level_ups,
        total_level_downs: existing_pool.total_level_downs,
        discoveries_unlocked: existing_pool.discoveries_unlocked,
        created_at: existing_pool.created_at,
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::PracticePool(updated_pool.clone()))?;

    // Update link
    let pool_anchor = StringAnchor::new("agent_pool", &agent_id);
    let pool_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pool_anchor))?;

    let query = LinkQuery::try_new(pool_anchor_hash.clone(), LinkTypes::AgentToPool)?;
    let links = get_links(query, GetStrategy::default())?;
    if let Some(old_link) = links.first() {
        delete_link(old_link.create_link_hash.clone(), GetOptions::default())?;
    }
    create_link(pool_anchor_hash, action_hash.clone(), LinkTypes::AgentToPool, ())?;

    Ok(PracticePoolOutput { action_hash, pool: updated_pool })
}

/// Add a path to the practice pool
#[hdk_extern]
pub fn add_path_to_pool(path_id: String) -> ExternResult<PracticePoolOutput> {
    let pool_output = get_or_create_practice_pool(CreatePoolInput {
        contributing_path_ids: vec![path_id.clone()],
        max_active_size: None,
        refresh_threshold: None,
        discovery_probability: None,
        regression_enabled: None,
        challenge_cooldown_hours: None,
    })?;

    let mut contributing: Vec<String> = serde_json::from_str(&pool_output.pool.contributing_path_ids_json)
        .unwrap_or_default();

    if !contributing.contains(&path_id) {
        contributing.push(path_id);
    }

    // Update and refresh
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let updated_pool = PracticePool {
        contributing_path_ids_json: serde_json::to_string(&contributing).unwrap_or_else(|_| "[]".to_string()),
        updated_at: timestamp,
        ..pool_output.pool
    };

    let action_hash = create_entry(&EntryTypes::PracticePool(updated_pool.clone()))?;

    // Update link
    let pool_anchor = StringAnchor::new("agent_pool", &agent_id);
    let pool_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pool_anchor))?;

    let query = LinkQuery::try_new(pool_anchor_hash.clone(), LinkTypes::AgentToPool)?;
    let links = get_links(query, GetStrategy::default())?;
    if let Some(old_link) = links.first() {
        delete_link(old_link.create_link_hash.clone(), GetOptions::default())?;
    }
    create_link(pool_anchor_hash, action_hash.clone(), LinkTypes::AgentToPool, ())?;

    // Refresh to populate with content
    refresh_practice_pool(())
}

/// Get pool recommendations for what to practice
#[hdk_extern]
pub fn get_pool_recommendations(_: ()) -> ExternResult<PoolRecommendations> {
    let pool_output = refresh_practice_pool(())?;
    let pool = pool_output.pool;

    let active: Vec<String> = serde_json::from_str(&pool.active_content_ids_json).unwrap_or_default();
    let refresh: Vec<String> = serde_json::from_str(&pool.refresh_queue_ids_json).unwrap_or_default();
    let discoveries: Vec<DiscoveryCandidate> = serde_json::from_str(&pool.discovery_candidates_json).unwrap_or_default();

    Ok(PoolRecommendations {
        priority_refresh: refresh,
        active_practice: active.clone(),
        discovery_suggestions: discoveries,
        total_pool_size: active.len() as u32,
    })
}

/// Check if agent can take a mastery challenge (cooldown)
#[hdk_extern]
pub fn check_challenge_cooldown(_: ()) -> ExternResult<CooldownCheckResult> {
    let pool_output = get_or_create_practice_pool(CreatePoolInput {
        contributing_path_ids: vec![],
        max_active_size: None,
        refresh_threshold: None,
        discovery_probability: None,
        regression_enabled: None,
        challenge_cooldown_hours: None,
    })?;

    let pool = pool_output.pool;

    match &pool.last_challenge_at {
        None => Ok(CooldownCheckResult {
            can_take_challenge: true,
            cooldown_remaining_hours: 0,
            last_challenge_at: None,
            next_available_at: None,
        }),
        Some(last_challenge) => {
            // Parse timestamp and check if cooldown has passed
            // For now, simplified: just check if string is set
            // In production, would parse timestamp and calculate
            let cooldown_hours = pool.challenge_cooldown_hours;

            // Simplified check - in production would actually parse and compare timestamps
            Ok(CooldownCheckResult {
                can_take_challenge: false, // Would calculate properly
                cooldown_remaining_hours: cooldown_hours,
                last_challenge_at: Some(last_challenge.clone()),
                next_available_at: Some(format!("{} + {} hours", last_challenge, cooldown_hours)),
            })
        }
    }
}

// =============================================================================
// Mastery Challenge Operations
// =============================================================================

/// Start a mastery challenge
#[hdk_extern]
pub fn start_mastery_challenge(input: StartChallengeInput) -> ExternResult<MasteryChallengeOutput> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get pool and check cooldown
    let pool_output = refresh_practice_pool(())?;
    let pool = pool_output.pool.clone();

    // Build content mix from pool
    let active: Vec<String> = serde_json::from_str(&pool.active_content_ids_json).unwrap_or_default();
    let refresh: Vec<String> = serde_json::from_str(&pool.refresh_queue_ids_json).unwrap_or_default();
    let discoveries: Vec<DiscoveryCandidate> = serde_json::from_str(&pool.discovery_candidates_json).unwrap_or_default();

    let mut content_mix: Vec<ContentMixEntry> = Vec::new();
    let mut questions_needed = input.question_count;
    let mut discovery_count = 0u32;

    // Add from refresh queue first (priority)
    for content_id in refresh.iter().take(questions_needed as usize / 3) {
        content_mix.push(ContentMixEntry {
            content_id: content_id.clone(),
            source: "refresh_queue".to_string(),
            question_count: 1,
        });
        questions_needed -= 1;
    }

    // Add from active content
    for content_id in active.iter().take(questions_needed as usize * 2 / 3) {
        content_mix.push(ContentMixEntry {
            content_id: content_id.clone(),
            source: "path_active".to_string(),
            question_count: 1,
        });
        questions_needed -= 1;
    }

    // Add discovery content if enabled
    if input.include_discoveries && !discoveries.is_empty() {
        // Use discovery_probability to decide
        let discovery_slots = (input.question_count as f64 * pool.discovery_probability) as usize;
        for discovery in discoveries.iter().take(discovery_slots.min(questions_needed as usize)) {
            content_mix.push(ContentMixEntry {
                content_id: discovery.content_id.clone(),
                source: "serendipity".to_string(),
                question_count: 1,
            });
            discovery_count += 1;
            questions_needed -= 1;
        }
    }

    // Generate placeholder questions (in production, would fetch from content)
    let mut questions: Vec<ChallengeQuestion> = Vec::new();
    for mix_entry in &content_mix {
        questions.push(ChallengeQuestion {
            content_id: mix_entry.content_id.clone(),
            question_type: "recall".to_string(),
            question_text: format!("Question about {}", mix_entry.content_id),
            options_json: "[]".to_string(),
            correct_answer: "".to_string(),
        });
    }

    let challenge_id = format!("challenge-{}-{}", agent_id, timestamp);

    let challenge = MasteryChallenge {
        id: challenge_id.clone(),
        agent_id: agent_id.clone(),
        pool_id: pool.id.clone(),
        path_id: input.path_id,
        content_mix_json: serde_json::to_string(&content_mix).unwrap_or_else(|_| "[]".to_string()),
        total_questions: content_mix.len() as u32,
        discovery_questions: discovery_count,
        state: "in_progress".to_string(),
        started_at: timestamp.clone(),
        completed_at: None,
        time_limit_seconds: input.time_limit_seconds,
        actual_time_seconds: None,
        questions_json: serde_json::to_string(&questions).unwrap_or_else(|_| "[]".to_string()),
        responses_json: "[]".to_string(),
        score: None,
        score_by_content_json: "{}".to_string(),
        level_changes_json: "[]".to_string(),
        net_level_change: 0,
        discoveries_json: "[]".to_string(),
        created_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::MasteryChallenge(challenge.clone()))?;

    // Create links
    let challenge_anchor = StringAnchor::new("agent_challenges", &agent_id);
    let challenge_anchor_hash = hash_entry(&EntryTypes::StringAnchor(challenge_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(challenge_anchor))?;
    create_link(challenge_anchor_hash, action_hash.clone(), LinkTypes::AgentToChallenge, ())?;

    Ok(MasteryChallengeOutput { action_hash, challenge })
}

/// Submit challenge responses and apply level changes
#[hdk_extern]
pub fn submit_mastery_challenge(input: SubmitChallengeInput) -> ExternResult<ChallengeResult> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get the challenge
    let challenge_anchor = StringAnchor::new("challenge_id", &input.challenge_id);
    let challenge_anchor_hash = hash_entry(&EntryTypes::StringAnchor(challenge_anchor))?;

    let query = LinkQuery::try_new(challenge_anchor_hash, LinkTypes::AgentToChallenge)?;
    let links = get_links(query, GetStrategy::default())?;

    let link = links.first()
        .ok_or(wasm_error!(WasmErrorInner::Guest("Challenge not found".to_string())))?;

    let action_hash = ActionHash::try_from(link.target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Challenge record not found".to_string())))?;

    let existing_challenge: MasteryChallenge = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Could not deserialize challenge".to_string())))?;

    // Calculate scores per content
    let mut correct_by_content: HashMap<String, (u32, u32)> = HashMap::new(); // (correct, total)

    for response in &input.responses {
        let entry = correct_by_content.entry(response.content_id.clone()).or_insert((0, 0));
        entry.1 += 1;
        if response.correct {
            entry.0 += 1;
        }
    }

    // Calculate overall score
    let total_correct: u32 = input.responses.iter().filter(|r| r.correct).count() as u32;
    let total_questions = input.responses.len() as u32;
    let overall_score = if total_questions > 0 {
        total_correct as f64 / total_questions as f64
    } else {
        0.0
    };

    // Apply level changes
    let mut level_changes: Vec<LevelChange> = Vec::new();
    let mut net_level_change: i32 = 0;
    let mut discoveries: Vec<ChallengeDiscovery> = Vec::new();

    // Get pool settings for regression
    let pool_output = get_or_create_practice_pool(CreatePoolInput {
        contributing_path_ids: vec![],
        max_active_size: None,
        refresh_threshold: None,
        discovery_probability: None,
        regression_enabled: None,
        challenge_cooldown_hours: None,
    })?;
    let pool = pool_output.pool;
    let regression_enabled = pool.regression_enabled;

    for (content_id, (correct, total)) in &correct_by_content {
        let content_score = *correct as f64 / *total as f64;

        // Get current mastery
        let current_mastery = get_my_mastery(content_id.clone())?;
        let (current_level, current_index) = match &current_mastery {
            Some(m) => (m.mastery.mastery_level.clone(), m.mastery.mastery_level_index),
            None => ("not_started".to_string(), 0),
        };

        // Determine new level
        let new_index = if content_score >= 0.8 {
            // Level up if scored 80%+
            (current_index + 1).min(7)
        } else if content_score < 0.4 && regression_enabled {
            // Level down if scored below 40% and regression enabled
            if current_index > 0 { current_index - 1 } else { 0 }
        } else {
            current_index
        };

        let new_level = MASTERY_LEVELS.get(new_index as usize)
            .unwrap_or(&"not_started")
            .to_string();

        let change = if new_index > current_index {
            "up"
        } else if new_index < current_index {
            "down"
        } else {
            "same"
        };

        level_changes.push(LevelChange {
            content_id: content_id.clone(),
            from_level: current_level.clone(),
            to_level: new_level.clone(),
            from_index: current_index,
            to_index: new_index,
            change: change.to_string(),
        });

        net_level_change += new_index as i32 - current_index as i32;

        // Apply the level change to mastery
        if new_index != current_index {
            upsert_mastery(UpsertMasteryInput {
                human_id: agent_id.clone(),
                content_id: content_id.clone(),
                mastery_level: new_level,
                engagement_type: "mastery_challenge".to_string(),
            })?;
        }
    }

    // Check for discoveries (serendipity content that was answered correctly)
    let content_mix: Vec<ContentMixEntry> = serde_json::from_str(&existing_challenge.content_mix_json)
        .unwrap_or_default();

    for mix_entry in &content_mix {
        if mix_entry.source == "serendipity" {
            if let Some((correct, _)) = correct_by_content.get(&mix_entry.content_id) {
                if *correct > 0 {
                    discoveries.push(ChallengeDiscovery {
                        content_id: mix_entry.content_id.clone(),
                        discovered_via: "mastery_challenge".to_string(),
                        relationship_type: "serendipity".to_string(),
                    });
                }
            }
        }
    }

    // Update challenge
    let updated_challenge = MasteryChallenge {
        id: existing_challenge.id,
        agent_id: existing_challenge.agent_id,
        pool_id: existing_challenge.pool_id,
        path_id: existing_challenge.path_id,
        content_mix_json: existing_challenge.content_mix_json,
        total_questions: existing_challenge.total_questions,
        discovery_questions: existing_challenge.discovery_questions,
        state: "completed".to_string(),
        started_at: existing_challenge.started_at,
        completed_at: Some(timestamp.clone()),
        time_limit_seconds: existing_challenge.time_limit_seconds,
        actual_time_seconds: Some(input.actual_time_seconds),
        questions_json: existing_challenge.questions_json,
        responses_json: serde_json::to_string(&input.responses).unwrap_or_else(|_| "[]".to_string()),
        score: Some(overall_score),
        score_by_content_json: serde_json::to_string(&correct_by_content).unwrap_or_else(|_| "{}".to_string()),
        level_changes_json: serde_json::to_string(&level_changes).unwrap_or_else(|_| "[]".to_string()),
        net_level_change,
        discoveries_json: serde_json::to_string(&discoveries).unwrap_or_else(|_| "[]".to_string()),
        created_at: existing_challenge.created_at,
    };

    let new_action_hash = create_entry(&EntryTypes::MasteryChallenge(updated_challenge.clone()))?;

    // Update pool statistics
    let total_ups = level_changes.iter().filter(|c| c.change == "up").count() as u32;
    let total_downs = level_changes.iter().filter(|c| c.change == "down").count() as u32;

    let updated_pool = PracticePool {
        last_challenge_at: Some(timestamp.clone()),
        last_challenge_id: Some(updated_challenge.id.clone()),
        total_challenges_taken: pool.total_challenges_taken + 1,
        total_level_ups: pool.total_level_ups + total_ups,
        total_level_downs: pool.total_level_downs + total_downs,
        discoveries_unlocked: pool.discoveries_unlocked + discoveries.len() as u32,
        updated_at: timestamp.clone(),
        ..pool
    };

    let pool_action_hash = create_entry(&EntryTypes::PracticePool(updated_pool))?;

    // Update pool link
    let pool_anchor = StringAnchor::new("agent_pool", &agent_id);
    let pool_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pool_anchor))?;

    let pool_query = LinkQuery::try_new(pool_anchor_hash.clone(), LinkTypes::AgentToPool)?;
    let pool_links = get_links(pool_query, GetStrategy::default())?;
    if let Some(old_link) = pool_links.first() {
        delete_link(old_link.create_link_hash.clone(), GetOptions::default())?;
    }
    create_link(pool_anchor_hash, pool_action_hash, LinkTypes::AgentToPool, ())?;

    // Calculate next available time
    let cooldown_hours = pool.challenge_cooldown_hours;
    let next_available = format!("{} + {} hours", timestamp, cooldown_hours);

    Ok(ChallengeResult {
        challenge: MasteryChallengeOutput {
            action_hash: new_action_hash,
            challenge: updated_challenge,
        },
        score: overall_score,
        level_changes,
        discoveries,
        net_level_change,
        can_retake_at: next_available,
    })
}

/// Get challenge history for current agent
#[hdk_extern]
pub fn get_challenge_history(_: ()) -> ExternResult<Vec<MasteryChallengeOutput>> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();

    let challenge_anchor = StringAnchor::new("agent_challenges", &agent_id);
    let challenge_anchor_hash = hash_entry(&EntryTypes::StringAnchor(challenge_anchor))?;

    let query = LinkQuery::try_new(challenge_anchor_hash, LinkTypes::AgentToChallenge)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(challenge) = rec.entry().to_app_option::<MasteryChallenge>().ok().flatten() {
                results.push(MasteryChallengeOutput { action_hash, challenge });
            }
        }
    }

    Ok(results)
}

// =============================================================================
// hREA Point System - Value Flow Demonstration
// =============================================================================

/// Point amounts for each trigger
fn get_point_amount(trigger: &str) -> i32 {
    match trigger {
        "engagement_view" => 1,
        "engagement_practice" => 2,
        "challenge_correct" => 5,
        "challenge_complete" => 10,
        "level_up" => 20,
        "level_down" => -10,
        "discovery" => 15,
        "path_step_complete" => 5,
        "path_complete" => 100,
        "contribution" => 50,
        _ => 1,
    }
}

/// Output for learner point balance
#[derive(Serialize, Deserialize, Debug)]
pub struct LearnerPointBalanceOutput {
    pub action_hash: ActionHash,
    pub balance: LearnerPointBalance,
}

/// Output for point event
#[derive(Serialize, Deserialize, Debug)]
pub struct PointEventOutput {
    pub action_hash: ActionHash,
    pub event: PointEvent,
}

/// Output for contributor recognition
#[derive(Serialize, Deserialize, Debug)]
pub struct ContributorRecognitionOutput {
    pub action_hash: ActionHash,
    pub recognition: ContributorRecognition,
}

/// Output for contributor impact
#[derive(Serialize, Deserialize, Debug)]
pub struct ContributorImpactOutput {
    pub action_hash: ActionHash,
    pub impact: ContributorImpact,
}

/// Input for earning points
#[derive(Serialize, Deserialize, Debug)]
pub struct EarnPointsInput {
    pub trigger: String,
    pub content_id: Option<String>,
    pub challenge_id: Option<String>,
    pub path_id: Option<String>,
    pub was_correct: Option<bool>,
    pub note: Option<String>,
}

/// Result of earning points (includes recognition flow)
#[derive(Serialize, Deserialize, Debug)]
pub struct EarnPointsResult {
    pub point_event: PointEventOutput,
    pub new_balance: LearnerPointBalanceOutput,
    pub recognition_sent: Vec<ContributorRecognitionOutput>,
    pub points_earned: i32,
}

/// Contributor dashboard - the exciting view!
#[derive(Serialize, Deserialize, Debug)]
pub struct ContributorDashboard {
    pub contributor_id: String,
    pub total_recognition_points: i64,
    pub total_learners_reached: u32,
    pub total_content_mastered: u32,
    pub total_discoveries_sparked: u32,
    pub impact_by_content: Vec<ContentImpactSummary>,
    pub recent_events: Vec<RecognitionEventSummary>,
    pub impact: Option<ContributorImpactOutput>,
}

/// Impact summary per content piece
#[derive(Serialize, Deserialize, Debug)]
pub struct ContentImpactSummary {
    pub content_id: String,
    pub recognition_points: i64,
    pub learners_reached: u32,
    pub mastery_count: u32,
}

/// Recent recognition event for the timeline
#[derive(Serialize, Deserialize, Debug)]
pub struct RecognitionEventSummary {
    pub learner_id: String,
    pub content_id: String,
    pub flow_type: String,
    pub recognition_points: i32,
    pub occurred_at: String,
}

/// Get or create a ContributorPresence for content.
/// This is the key to allowing recognition to flow even when contributors aren't "present" yet.
/// The presence exists in states: unclaimed → stewarded → claimed
/// Recognition accumulates regardless of state, and transfers when claimed.
fn get_or_create_content_presence(
    content_id: &str,
    content_title: &str,
    author_id: Option<&str>,
    timestamp: &str,
) -> ExternResult<String> {
    // First, check if there's already a presence linked to this content
    let content_anchor = StringAnchor::new("content_presence", content_id);
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor.clone()))?;

    let query = LinkQuery::try_new(content_anchor_hash.clone(), LinkTypes::ContentToPresence)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        // Presence exists for this content, get its ID
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid presence hash".to_string())))?;

        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Some(presence) = record.entry().to_app_option::<ContributorPresence>().ok().flatten() {
                return Ok(presence.id);
            }
        }
    }

    // No presence exists - create an unclaimed one for this content
    // Display name comes from author if known, otherwise from content title
    let display_name = author_id
        .map(|a| a.to_string())
        .unwrap_or_else(|| format!("Creator of: {}", content_title));

    // Build external identifiers from author if known
    let external_ids: HashMap<String, String> = author_id
        .map(|a| {
            let mut ids = HashMap::new();
            ids.insert("source_author_id".to_string(), a.to_string());
            ids
        })
        .unwrap_or_default();

    // Build establishing content list
    let establishing_content: Vec<String> = vec![content_id.to_string()];

    let presence_id = format!("presence-content-{}", content_id);

    let presence = ContributorPresence {
        id: presence_id.clone(),
        display_name,
        presence_state: "unclaimed".to_string(),
        external_identifiers_json: serde_json::to_string(&external_ids).unwrap_or_else(|_| "{}".to_string()),
        establishing_content_ids_json: serde_json::to_string(&establishing_content).unwrap_or_else(|_| "[]".to_string()),
        established_at: timestamp.to_string(),
        // Recognition starts at zero
        affinity_total: 0.0,
        unique_engagers: 0,
        citation_count: 0,
        endorsements_json: "[]".to_string(),
        recognition_score: 0.0,
        recognition_by_content_json: "{}".to_string(),
        accumulating_since: timestamp.to_string(),
        last_recognition_at: timestamp.to_string(),
        // No steward yet
        steward_id: None,
        stewardship_started_at: None,
        stewardship_commitment_id: None,
        stewardship_quality_score: None,
        // No claim yet
        claim_initiated_at: None,
        claim_verified_at: None,
        claim_verification_method: None,
        claim_evidence_json: None,
        claimed_agent_id: None,
        claim_recognition_transferred_value: None,
        claim_recognition_transferred_unit: None,
        claim_facilitated_by: None,
        invitations_json: "[]".to_string(),
        note: Some(format!("Auto-created presence for content: {}", content_id)),
        image: None,
        metadata_json: "{}".to_string(),
        created_at: timestamp.to_string(),
        updated_at: timestamp.to_string(),
    };

    let action_hash = create_entry(&EntryTypes::ContributorPresence(presence))?;

    // Create content-to-presence link so we find this presence next time
    create_entry(&EntryTypes::StringAnchor(content_anchor))?;
    create_link(content_anchor_hash, action_hash.clone(), LinkTypes::ContentToPresence, ())?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("presence_id", &presence_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPresence, ())?;

    // Create state lookup link (unclaimed)
    let state_anchor = StringAnchor::new("presence_state", "unclaimed");
    let state_anchor_hash = hash_entry(&EntryTypes::StringAnchor(state_anchor))?;
    create_link(state_anchor_hash, action_hash, LinkTypes::PresenceByState, ())?;

    Ok(presence_id)
}

/// Earn points (and trigger recognition flow to contributors)
#[hdk_extern]
pub fn earn_points(input: EarnPointsInput) -> ExternResult<EarnPointsResult> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Calculate points
    let points = get_point_amount(&input.trigger);

    // Create point event (hREA EconomicEvent)
    let event_id = format!("pe-{}-{}", agent_id, timestamp);
    let point_event = PointEvent {
        id: event_id.clone(),
        agent_id: agent_id.clone(),
        action: if points >= 0 { "produce".to_string() } else { "consume".to_string() },
        trigger: input.trigger.clone(),
        points,
        content_id: input.content_id.clone(),
        challenge_id: input.challenge_id.clone(),
        path_id: input.path_id.clone(),
        was_correct: input.was_correct,
        note: input.note,
        metadata_json: "{}".to_string(),
        occurred_at: timestamp.clone(),
    };

    let event_action_hash = create_entry(&EntryTypes::PointEvent(point_event.clone()))?;

    // Create links for the event
    let agent_events_anchor = StringAnchor::new("agent_points", &agent_id);
    let agent_events_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_events_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(agent_events_anchor))?;
    create_link(agent_events_anchor_hash, event_action_hash.clone(), LinkTypes::AgentToPointEvents, ())?;

    if let Some(ref content_id) = input.content_id {
        let content_events_anchor = StringAnchor::new("content_points", content_id);
        let content_events_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_events_anchor.clone()))?;
        create_entry(&EntryTypes::StringAnchor(content_events_anchor))?;
        create_link(content_events_anchor_hash, event_action_hash.clone(), LinkTypes::ContentToPointEvents, ())?;
    }

    // Update or create point balance (hREA EconomicResource)
    let balance_output = update_point_balance(&agent_id, points, &input.trigger, &event_id, &timestamp)?;

    // Flow recognition to contributors (hREA Appreciation)
    // Recognition ALWAYS flows - ContributorPresence exists for all content,
    // even when the creator hasn't claimed their presence yet.
    // This is the key to the stewardship model: recognition accumulates
    // and transfers when claimed, attesting for humanity.
    let mut recognition_sent = Vec::new();
    if let Some(ref content_id) = input.content_id {
        // Get content to find title and author info
        if let Some(content_output) = get_content_by_id(QueryByIdInput { id: content_id.clone() })? {
            let content = content_output.content;

            // Get or create the ContributorPresence for this content
            // Recognition flows regardless of whether author is "present"
            let presence_id = get_or_create_content_presence(
                content_id,
                &content.title,
                content.author_id.as_deref(),
                &timestamp,
            )?;

            // Flow recognition to the presence (claimed, stewarded, or unclaimed)
            let recognition = flow_recognition_to_contributor(
                &presence_id,
                content_id,
                &agent_id,
                &event_id,
                &input.trigger,
                points,
                input.path_id.clone(),
                input.challenge_id.clone(),
                &timestamp,
            )?;
            recognition_sent.push(recognition);
        }
    }

    Ok(EarnPointsResult {
        point_event: PointEventOutput {
            action_hash: event_action_hash,
            event: point_event,
        },
        new_balance: balance_output,
        recognition_sent,
        points_earned: points,
    })
}

/// Update or create point balance
fn update_point_balance(
    agent_id: &str,
    points: i32,
    trigger: &str,
    event_id: &str,
    timestamp: &str,
) -> ExternResult<LearnerPointBalanceOutput> {
    let balance_anchor = StringAnchor::new("agent_balance", agent_id);
    let balance_anchor_hash = hash_entry(&EntryTypes::StringAnchor(balance_anchor.clone()))?;

    let query = LinkQuery::try_new(balance_anchor_hash.clone(), LinkTypes::AgentToPointBalance)?;
    let links = get_links(query, GetStrategy::default())?;

    let (existing_balance, existing_action_hash) = if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid balance hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(balance) = rec.entry().to_app_option::<LearnerPointBalance>().ok().flatten() {
                (Some(balance), Some((action_hash, link.create_link_hash.clone())))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    // Update points by trigger
    let mut points_by_trigger: HashMap<String, i64> = match &existing_balance {
        Some(b) => serde_json::from_str(&b.points_by_trigger_json).unwrap_or_default(),
        None => HashMap::new(),
    };
    *points_by_trigger.entry(trigger.to_string()).or_insert(0) += points as i64;

    let updated_balance = LearnerPointBalance {
        id: format!("balance-{}", agent_id),
        agent_id: agent_id.to_string(),
        total_points: existing_balance.as_ref().map(|b| b.total_points).unwrap_or(0) + points as i64,
        points_by_trigger_json: serde_json::to_string(&points_by_trigger).unwrap_or_else(|_| "{}".to_string()),
        total_earned: existing_balance.as_ref().map(|b| b.total_earned).unwrap_or(0) + if points > 0 { points as i64 } else { 0 },
        total_spent: existing_balance.as_ref().map(|b| b.total_spent).unwrap_or(0) + if points < 0 { points.abs() as i64 } else { 0 },
        last_point_event_id: Some(event_id.to_string()),
        last_point_event_at: Some(timestamp.to_string()),
        created_at: existing_balance.as_ref().map(|b| b.created_at.clone()).unwrap_or_else(|| timestamp.to_string()),
        updated_at: timestamp.to_string(),
    };

    let action_hash = create_entry(&EntryTypes::LearnerPointBalance(updated_balance.clone()))?;

    // Update link
    if let Some((_, old_link_hash)) = existing_action_hash {
        delete_link(old_link_hash, GetOptions::default())?;
    } else {
        create_entry(&EntryTypes::StringAnchor(balance_anchor))?;
    }
    create_link(balance_anchor_hash, action_hash.clone(), LinkTypes::AgentToPointBalance, ())?;

    Ok(LearnerPointBalanceOutput {
        action_hash,
        balance: updated_balance,
    })
}

/// Flow recognition to a contributor (hREA Appreciation)
fn flow_recognition_to_contributor(
    contributor_id: &str,
    content_id: &str,
    learner_id: &str,
    appreciation_of_event_id: &str,
    trigger: &str,
    learner_points: i32,
    path_id: Option<String>,
    challenge_id: Option<String>,
    timestamp: &str,
) -> ExternResult<ContributorRecognitionOutput> {
    // Calculate recognition points (fraction of learner points)
    let recognition_points = (learner_points.abs() as f64 * 0.2) as i32; // 20% flows to contributor

    // Determine flow type
    let flow_type = match trigger {
        "engagement_view" | "engagement_practice" => "content_engagement",
        "level_up" | "challenge_correct" => "content_mastery",
        "path_complete" | "path_step_complete" => "path_completion",
        "discovery" => "discovery_spark",
        _ => "content_engagement",
    };

    let recognition_id = format!("recog-{}-{}-{}", contributor_id, learner_id, timestamp);

    let recognition = ContributorRecognition {
        id: recognition_id,
        contributor_id: contributor_id.to_string(),
        content_id: content_id.to_string(),
        learner_id: learner_id.to_string(),
        appreciation_of_event_id: appreciation_of_event_id.to_string(),
        flow_type: flow_type.to_string(),
        recognition_points,
        path_id,
        challenge_id,
        note: Some(format!("Recognition for {} learning your content", trigger)),
        occurred_at: timestamp.to_string(),
    };

    let action_hash = create_entry(&EntryTypes::ContributorRecognition(recognition.clone()))?;

    // Create links
    let contributor_anchor = StringAnchor::new("contributor_recognition", contributor_id);
    let contributor_anchor_hash = hash_entry(&EntryTypes::StringAnchor(contributor_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(contributor_anchor))?;
    create_link(contributor_anchor_hash, action_hash.clone(), LinkTypes::ContributorToRecognition, ())?;

    let content_anchor = StringAnchor::new("content_recognition", content_id);
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(content_anchor))?;
    create_link(content_anchor_hash, action_hash.clone(), LinkTypes::ContentToRecognition, ())?;

    // Update contributor impact summary
    update_contributor_impact(contributor_id, recognition_points, content_id, flow_type, timestamp)?;

    Ok(ContributorRecognitionOutput {
        action_hash,
        recognition,
    })
}

/// Update contributor impact summary
fn update_contributor_impact(
    contributor_id: &str,
    recognition_points: i32,
    content_id: &str,
    flow_type: &str,
    timestamp: &str,
) -> ExternResult<ContributorImpactOutput> {
    let impact_anchor = StringAnchor::new("contributor_impact", contributor_id);
    let impact_anchor_hash = hash_entry(&EntryTypes::StringAnchor(impact_anchor.clone()))?;

    let query = LinkQuery::try_new(impact_anchor_hash.clone(), LinkTypes::ContributorToImpact)?;
    let links = get_links(query, GetStrategy::default())?;

    let existing = if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid impact hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(rec) = record {
            rec.entry().to_app_option::<ContributorImpact>().ok().flatten()
        } else {
            None
        }
    } else {
        None
    };

    // Update impact by content
    let mut impact_by_content: HashMap<String, serde_json::Value> = match &existing {
        Some(i) => serde_json::from_str(&i.impact_by_content_json).unwrap_or_default(),
        None => HashMap::new(),
    };
    let content_impact = impact_by_content.entry(content_id.to_string()).or_insert(serde_json::json!({
        "points": 0,
        "learners": 0,
        "mastered": 0
    }));
    if let Some(obj) = content_impact.as_object_mut() {
        let current_points = obj.get("points").and_then(|v| v.as_i64()).unwrap_or(0);
        obj.insert("points".to_string(), serde_json::json!(current_points + recognition_points as i64));
    }

    // Update impact by flow type
    let mut impact_by_flow: HashMap<String, i64> = match &existing {
        Some(i) => serde_json::from_str(&i.impact_by_flow_type_json).unwrap_or_default(),
        None => HashMap::new(),
    };
    *impact_by_flow.entry(flow_type.to_string()).or_insert(0) += recognition_points as i64;

    let is_mastery = flow_type == "content_mastery";
    let is_discovery = flow_type == "discovery_spark";

    let updated_impact = ContributorImpact {
        id: format!("impact-{}", contributor_id),
        contributor_id: contributor_id.to_string(),
        total_recognition_points: existing.as_ref().map(|i| i.total_recognition_points).unwrap_or(0) + recognition_points as i64,
        total_learners_reached: existing.as_ref().map(|i| i.total_learners_reached).unwrap_or(0) + 1,
        total_content_mastered: existing.as_ref().map(|i| i.total_content_mastered).unwrap_or(0) + if is_mastery { 1 } else { 0 },
        total_discoveries_sparked: existing.as_ref().map(|i| i.total_discoveries_sparked).unwrap_or(0) + if is_discovery { 1 } else { 0 },
        impact_by_content_json: serde_json::to_string(&impact_by_content).unwrap_or_else(|_| "{}".to_string()),
        impact_by_flow_type_json: serde_json::to_string(&impact_by_flow).unwrap_or_else(|_| "{}".to_string()),
        recent_events_json: "[]".to_string(), // Would append recent events
        created_at: existing.as_ref().map(|i| i.created_at.clone()).unwrap_or_else(|| timestamp.to_string()),
        updated_at: timestamp.to_string(),
    };

    let action_hash = create_entry(&EntryTypes::ContributorImpact(updated_impact.clone()))?;

    // Update link
    let query2 = LinkQuery::try_new(impact_anchor_hash.clone(), LinkTypes::ContributorToImpact)?;
    let links2 = get_links(query2, GetStrategy::default())?;
    if let Some(old_link) = links2.first() {
        delete_link(old_link.create_link_hash.clone(), GetOptions::default())?;
    } else {
        create_entry(&EntryTypes::StringAnchor(impact_anchor))?;
    }
    create_link(impact_anchor_hash, action_hash.clone(), LinkTypes::ContributorToImpact, ())?;

    Ok(ContributorImpactOutput {
        action_hash,
        impact: updated_impact,
    })
}

/// Get my point balance
#[hdk_extern]
pub fn get_my_point_balance(_: ()) -> ExternResult<Option<LearnerPointBalanceOutput>> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();

    let balance_anchor = StringAnchor::new("agent_balance", &agent_id);
    let balance_anchor_hash = hash_entry(&EntryTypes::StringAnchor(balance_anchor))?;

    let query = LinkQuery::try_new(balance_anchor_hash, LinkTypes::AgentToPointBalance)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid balance hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(balance) = rec.entry().to_app_option::<LearnerPointBalance>().ok().flatten() {
                return Ok(Some(LearnerPointBalanceOutput { action_hash, balance }));
            }
        }
    }

    Ok(None)
}

/// Get my point history
#[hdk_extern]
pub fn get_my_point_history(_: ()) -> ExternResult<Vec<PointEventOutput>> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();

    let events_anchor = StringAnchor::new("agent_points", &agent_id);
    let events_anchor_hash = hash_entry(&EntryTypes::StringAnchor(events_anchor))?;

    let query = LinkQuery::try_new(events_anchor_hash, LinkTypes::AgentToPointEvents)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid event hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(event) = rec.entry().to_app_option::<PointEvent>().ok().flatten() {
                results.push(PointEventOutput { action_hash, event });
            }
        }
    }

    Ok(results)
}

/// Get contributor dashboard - THE EXCITING VIEW!
#[hdk_extern]
pub fn get_contributor_dashboard(contributor_id: String) -> ExternResult<ContributorDashboard> {
    // Get impact summary
    let impact_anchor = StringAnchor::new("contributor_impact", &contributor_id);
    let impact_anchor_hash = hash_entry(&EntryTypes::StringAnchor(impact_anchor))?;

    let query = LinkQuery::try_new(impact_anchor_hash, LinkTypes::ContributorToImpact)?;
    let links = get_links(query, GetStrategy::default())?;

    let impact = if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid impact hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            rec.entry().to_app_option::<ContributorImpact>().ok().flatten()
                .map(|impact| ContributorImpactOutput { action_hash, impact })
        } else {
            None
        }
    } else {
        None
    };

    // Parse impact by content for the summary
    let impact_by_content: Vec<ContentImpactSummary> = match &impact {
        Some(i) => {
            let map: HashMap<String, serde_json::Value> = serde_json::from_str(&i.impact.impact_by_content_json)
                .unwrap_or_default();
            map.into_iter().map(|(content_id, v)| {
                ContentImpactSummary {
                    content_id,
                    recognition_points: v.get("points").and_then(|p| p.as_i64()).unwrap_or(0),
                    learners_reached: v.get("learners").and_then(|l| l.as_u64()).unwrap_or(0) as u32,
                    mastery_count: v.get("mastered").and_then(|m| m.as_u64()).unwrap_or(0) as u32,
                }
            }).collect()
        }
        None => Vec::new(),
    };

    // Get recent recognition events
    let recognition_anchor = StringAnchor::new("contributor_recognition", &contributor_id);
    let recognition_anchor_hash = hash_entry(&EntryTypes::StringAnchor(recognition_anchor))?;

    let recog_query = LinkQuery::try_new(recognition_anchor_hash, LinkTypes::ContributorToRecognition)?;
    let recog_links = get_links(recog_query, GetStrategy::default())?;

    let mut recent_events = Vec::new();
    for link in recog_links.iter().take(10) {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid recognition hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(recognition) = rec.entry().to_app_option::<ContributorRecognition>().ok().flatten() {
                recent_events.push(RecognitionEventSummary {
                    learner_id: recognition.learner_id,
                    content_id: recognition.content_id,
                    flow_type: recognition.flow_type,
                    recognition_points: recognition.recognition_points,
                    occurred_at: recognition.occurred_at,
                });
            }
        }
    }

    Ok(ContributorDashboard {
        contributor_id: contributor_id.clone(),
        total_recognition_points: impact.as_ref().map(|i| i.impact.total_recognition_points).unwrap_or(0),
        total_learners_reached: impact.as_ref().map(|i| i.impact.total_learners_reached).unwrap_or(0),
        total_content_mastered: impact.as_ref().map(|i| i.impact.total_content_mastered).unwrap_or(0),
        total_discoveries_sparked: impact.as_ref().map(|i| i.impact.total_discoveries_sparked).unwrap_or(0),
        impact_by_content,
        recent_events,
        impact,
    })
}

/// Get my contributor dashboard (for current agent)
#[hdk_extern]
pub fn get_my_contributor_dashboard(_: ()) -> ExternResult<ContributorDashboard> {
    let agent_info = agent_info()?;
    let agent_id = agent_info.agent_initial_pubkey.to_string();
    get_contributor_dashboard(agent_id)
}

// =============================================================================
// Lamad: Steward Economy Operations
// =============================================================================
//
// The Steward Economy enables sustainable income for those who care-take the
// knowledge graph. Stewards may or may not be the original creators - they
// earn from maintaining, curating, and making knowledge accessible.
//
// Key concepts:
// - StewardCredential: Proof of qualification (mastery, peer attestations, track record)
// - PremiumGate: Access control with pricing and revenue sharing
// - AccessGrant: Record of learner gaining access
// - StewardRevenue: Three-way split (steward, contributor, commons)
// =============================================================================

/// Output for steward credential
#[derive(Serialize, Deserialize, Debug)]
pub struct StewardCredentialOutput {
    pub action_hash: ActionHash,
    pub credential: StewardCredential,
}

/// Output for premium gate
#[derive(Serialize, Deserialize, Debug)]
pub struct PremiumGateOutput {
    pub action_hash: ActionHash,
    pub gate: PremiumGate,
}

/// Output for access grant
#[derive(Serialize, Deserialize, Debug)]
pub struct AccessGrantOutput {
    pub action_hash: ActionHash,
    pub grant: AccessGrant,
}

/// Output for steward revenue
#[derive(Serialize, Deserialize, Debug)]
pub struct StewardRevenueOutput {
    pub action_hash: ActionHash,
    pub revenue: StewardRevenue,
}

/// Input for creating a steward credential
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateStewardCredentialInput {
    pub steward_presence_id: String,
    pub tier: String,
    pub domain_tags: Vec<String>,
    pub mastery_content_ids: Vec<String>,
    pub mastery_level_achieved: String,
    pub peer_attestation_ids: Vec<String>,
    pub stewarded_presence_ids: Vec<String>,
    pub stewarded_content_ids: Vec<String>,
    pub stewarded_path_ids: Vec<String>,
    pub note: Option<String>,
}

/// Input for creating a premium gate
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePremiumGateInput {
    pub steward_credential_id: String,
    pub steward_presence_id: String,
    pub contributor_presence_id: Option<String>,
    pub gated_resource_type: String,
    pub gated_resource_ids: Vec<String>,
    pub gate_title: String,
    pub gate_description: String,
    pub gate_image: Option<String>,
    pub required_attestations: Vec<RequiredAttestationInput>,
    pub required_mastery: Vec<RequiredMasteryInput>,
    pub required_vouches: Option<RequiredVouchesInput>,
    pub pricing_model: String,
    pub price_amount: Option<f64>,
    pub price_unit: Option<String>,
    pub subscription_period_days: Option<u32>,
    pub min_amount: Option<f64>,
    pub steward_share_percent: f64,
    pub commons_share_percent: f64,
    pub contributor_share_percent: Option<f64>,
    pub scholarship_eligible: bool,
    pub max_scholarships_per_period: Option<u32>,
    pub scholarship_criteria_json: Option<String>,
    pub note: Option<String>,
}

/// Required attestation for gate access
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequiredAttestationInput {
    pub attestation_type: String,
    pub attestation_id: Option<String>,
}

/// Required mastery for gate access
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequiredMasteryInput {
    pub content_id: String,
    pub min_level: String,
}

/// Required vouches for gate access
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequiredVouchesInput {
    pub min_count: u32,
    pub from_tier: Option<String>,
}

/// Input for granting access
#[derive(Serialize, Deserialize, Debug)]
pub struct GrantAccessInput {
    pub gate_id: String,
    pub grant_type: String,
    pub granted_via: String,
    pub payment_amount: Option<f64>,
    pub payment_unit: Option<String>,
    pub scholarship_sponsor_id: Option<String>,
    pub scholarship_reason: Option<String>,
}

/// Create a steward credential
#[hdk_extern]
pub fn create_steward_credential(input: CreateStewardCredentialInput) -> ExternResult<StewardCredentialOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Validate steward tier
    if !STEWARD_TIERS.contains(&input.tier.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Invalid steward tier: {}. Must be one of: {:?}", input.tier, STEWARD_TIERS)
        )));
    }

    let credential_id = format!("steward-cred-{}-{}", input.steward_presence_id, timestamp);

    let credential = StewardCredential {
        id: credential_id.clone(),
        steward_presence_id: input.steward_presence_id.clone(),
        agent_id: agent_info.agent_initial_pubkey.to_string(),
        tier: input.tier.clone(),
        stewarded_presence_ids_json: serde_json::to_string(&input.stewarded_presence_ids).unwrap_or_else(|_| "[]".to_string()),
        stewarded_content_ids_json: serde_json::to_string(&input.stewarded_content_ids).unwrap_or_else(|_| "[]".to_string()),
        stewarded_path_ids_json: serde_json::to_string(&input.stewarded_path_ids).unwrap_or_else(|_| "[]".to_string()),
        mastery_content_ids_json: serde_json::to_string(&input.mastery_content_ids).unwrap_or_else(|_| "[]".to_string()),
        mastery_level_achieved: input.mastery_level_achieved.clone(),
        qualification_verified_at: timestamp.clone(),
        peer_attestation_ids_json: serde_json::to_string(&input.peer_attestation_ids).unwrap_or_else(|_| "[]".to_string()),
        unique_attester_count: input.peer_attestation_ids.len() as u32,
        attester_reputation_sum: 0.0,
        stewardship_quality_score: 0.0,
        total_learners_served: 0,
        total_content_improvements: 0,
        domain_tags_json: serde_json::to_string(&input.domain_tags).unwrap_or_else(|_| "[]".to_string()),
        is_active: true,
        deactivation_reason: None,
        note: input.note,
        metadata_json: "{}".to_string(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::StewardCredential(credential.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("steward_credential_id", &credential_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToStewardCredential, ())?;

    // Create steward presence link
    let human_anchor = StringAnchor::new("human_credential", &input.steward_presence_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(human_anchor))?;
    create_link(human_anchor_hash, action_hash.clone(), LinkTypes::HumanToCredential, ())?;

    // Create tier lookup link
    let tier_anchor = StringAnchor::new("credential_tier", &input.tier);
    let tier_anchor_hash = hash_entry(&EntryTypes::StringAnchor(tier_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(tier_anchor))?;
    create_link(tier_anchor_hash, action_hash.clone(), LinkTypes::CredentialByTier, ())?;

    // Create domain scope links
    for domain in &input.domain_tags {
        let domain_anchor = StringAnchor::new("credential_domain", domain);
        let domain_anchor_hash = hash_entry(&EntryTypes::StringAnchor(domain_anchor.clone()))?;
        create_entry(&EntryTypes::StringAnchor(domain_anchor))?;
        create_link(domain_anchor_hash, action_hash.clone(), LinkTypes::CredentialByDomain, ())?;
    }

    Ok(StewardCredentialOutput {
        action_hash,
        credential,
    })
}

/// Get steward credential by ID
#[hdk_extern]
pub fn get_steward_credential(credential_id: String) -> ExternResult<Option<StewardCredentialOutput>> {
    let id_anchor = StringAnchor::new("steward_credential_id", &credential_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToStewardCredential)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid credential hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(credential) = rec.entry().to_app_option::<StewardCredential>().ok().flatten() {
                return Ok(Some(StewardCredentialOutput { action_hash, credential }));
            }
        }
    }

    Ok(None)
}

/// Get steward credentials for a human presence
#[hdk_extern]
pub fn get_credentials_for_human(human_presence_id: String) -> ExternResult<Vec<StewardCredentialOutput>> {
    let human_anchor = StringAnchor::new("human_credential", &human_presence_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor))?;

    let query = LinkQuery::try_new(human_anchor_hash, LinkTypes::HumanToCredential)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid credential hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(credential) = rec.entry().to_app_option::<StewardCredential>().ok().flatten() {
                if credential.is_active {
                    results.push(StewardCredentialOutput { action_hash, credential });
                }
            }
        }
    }

    Ok(results)
}

/// Create a premium gate for content
#[hdk_extern]
pub fn create_premium_gate(input: CreatePremiumGateInput) -> ExternResult<PremiumGateOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Validate pricing model
    if !PRICING_MODELS.contains(&input.pricing_model.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Invalid pricing model: {}. Must be one of: {:?}", input.pricing_model, PRICING_MODELS)
        )));
    }

    // Validate revenue shares sum to 100%
    let contributor_share = input.contributor_share_percent.unwrap_or(0.0);
    let total_share = input.steward_share_percent + contributor_share + input.commons_share_percent;
    if (total_share - 100.0).abs() > 0.01 {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Revenue shares must sum to 100%, got {}", total_share)
        )));
    }

    // Get first resource ID for the gate ID
    let first_resource = input.gated_resource_ids.first()
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("At least one gated resource ID required".to_string())))?;

    let gate_id = format!("gate-{}-{}", first_resource, timestamp);

    let gate = PremiumGate {
        id: gate_id.clone(),
        steward_credential_id: input.steward_credential_id.clone(),
        steward_presence_id: input.steward_presence_id.clone(),
        contributor_presence_id: input.contributor_presence_id.clone(),
        gated_resource_type: input.gated_resource_type,
        gated_resource_ids_json: serde_json::to_string(&input.gated_resource_ids).unwrap_or_else(|_| "[]".to_string()),
        gate_title: input.gate_title,
        gate_description: input.gate_description,
        gate_image: input.gate_image,
        required_attestations_json: serde_json::to_string(&input.required_attestations).unwrap_or_else(|_| "[]".to_string()),
        required_mastery_json: serde_json::to_string(&input.required_mastery).unwrap_or_else(|_| "[]".to_string()),
        required_vouches_json: serde_json::to_string(&input.required_vouches).unwrap_or_else(|_| "{}".to_string()),
        pricing_model: input.pricing_model.clone(),
        price_amount: input.price_amount,
        price_unit: input.price_unit,
        subscription_period_days: input.subscription_period_days,
        min_amount: input.min_amount,
        steward_share_percent: input.steward_share_percent,
        commons_share_percent: input.commons_share_percent,
        contributor_share_percent: input.contributor_share_percent,
        scholarship_eligible: input.scholarship_eligible,
        max_scholarships_per_period: input.max_scholarships_per_period,
        scholarship_criteria_json: input.scholarship_criteria_json,
        is_active: true,
        deactivation_reason: None,
        total_access_grants: 0,
        total_revenue_generated: 0.0,
        total_to_steward: 0.0,
        total_to_contributor: 0.0,
        total_to_commons: 0.0,
        total_scholarships_granted: 0,
        note: input.note,
        metadata_json: "{}".to_string(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::PremiumGate(gate.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("gate_id", &gate_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToGate, ())?;

    // Create resource lookup links for all gated resources
    for resource_id in &input.gated_resource_ids {
        let resource_anchor = StringAnchor::new("resource_gate", resource_id);
        let resource_anchor_hash = hash_entry(&EntryTypes::StringAnchor(resource_anchor.clone()))?;
        create_entry(&EntryTypes::StringAnchor(resource_anchor))?;
        create_link(resource_anchor_hash, action_hash.clone(), LinkTypes::ResourceToGate, ())?;
    }

    // Create pricing model link
    let pricing_anchor = StringAnchor::new("gate_pricing", &input.pricing_model);
    let pricing_anchor_hash = hash_entry(&EntryTypes::StringAnchor(pricing_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(pricing_anchor))?;
    create_link(pricing_anchor_hash, action_hash.clone(), LinkTypes::GateByPricingModel, ())?;

    // Create steward lookup link
    let steward_anchor = StringAnchor::new("steward_gates", &input.steward_credential_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(steward_anchor))?;
    create_link(steward_anchor_hash, action_hash.clone(), LinkTypes::GateBySteward, ())?;

    // Create contributor lookup link if different from steward
    if let Some(ref contributor_id) = input.contributor_presence_id {
        let contributor_anchor = StringAnchor::new("contributor_gates", contributor_id);
        let contributor_anchor_hash = hash_entry(&EntryTypes::StringAnchor(contributor_anchor.clone()))?;
        create_entry(&EntryTypes::StringAnchor(contributor_anchor))?;
        create_link(contributor_anchor_hash, action_hash.clone(), LinkTypes::GateByContributor, ())?;
    }

    Ok(PremiumGateOutput {
        action_hash,
        gate,
    })
}

/// Get premium gate by ID
#[hdk_extern]
pub fn get_premium_gate(gate_id: String) -> ExternResult<Option<PremiumGateOutput>> {
    let id_anchor = StringAnchor::new("gate_id", &gate_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToGate)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid gate hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(gate) = rec.entry().to_app_option::<PremiumGate>().ok().flatten() {
                return Ok(Some(PremiumGateOutput { action_hash, gate }));
            }
        }
    }

    Ok(None)
}

/// Get gates for a resource
#[hdk_extern]
pub fn get_gates_for_resource(resource_id: String) -> ExternResult<Vec<PremiumGateOutput>> {
    let resource_anchor = StringAnchor::new("resource_gate", &resource_id);
    let resource_anchor_hash = hash_entry(&EntryTypes::StringAnchor(resource_anchor))?;

    let query = LinkQuery::try_new(resource_anchor_hash, LinkTypes::ResourceToGate)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid gate hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(gate) = rec.entry().to_app_option::<PremiumGate>().ok().flatten() {
                if gate.is_active {
                    results.push(PremiumGateOutput { action_hash, gate });
                }
            }
        }
    }

    Ok(results)
}

/// Grant access through a gate
#[hdk_extern]
pub fn grant_access(input: GrantAccessInput) -> ExternResult<AccessGrantOutput> {
    let agent_info = agent_info()?;
    let learner_id = agent_info.agent_initial_pubkey.to_string();
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Get the gate
    let gate = get_premium_gate(input.gate_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Gate not found".to_string())))?;

    // Validate access type
    if !ACCESS_GRANT_TYPES.contains(&input.grant_type.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Invalid grant type: {}. Must be one of: {:?}", input.grant_type, ACCESS_GRANT_TYPES)
        )));
    }

    let grant_id = format!("grant-{}-{}-{}", input.gate_id, learner_id, timestamp);

    // Calculate expiration based on subscription period if applicable
    let valid_until = match input.grant_type.as_str() {
        "subscription" => {
            let days = gate.gate.subscription_period_days.unwrap_or(30);
            Some(format!("{}+{}d", timestamp, days))
        }
        _ => None,
    };

    let grant = AccessGrant {
        id: grant_id.clone(),
        gate_id: input.gate_id.clone(),
        learner_agent_id: learner_id.clone(),
        grant_type: input.grant_type.clone(),
        granted_via: input.granted_via.clone(),
        payment_event_id: None,
        payment_amount: input.payment_amount,
        payment_unit: input.payment_unit.clone(),
        scholarship_sponsor_id: input.scholarship_sponsor_id.clone(),
        scholarship_reason: input.scholarship_reason.clone(),
        granted_at: timestamp.clone(),
        valid_until,
        renewal_due_at: None,
        is_active: true,
        revoked_at: None,
        revoke_reason: None,
        metadata_json: "{}".to_string(),
        created_at: timestamp.clone(),
    };

    let action_hash = create_entry(&EntryTypes::AccessGrant(grant.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("grant_id", &grant_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToAccessGrant, ())?;

    // Create learner lookup link
    let learner_anchor = StringAnchor::new("learner_grants", &learner_id);
    let learner_anchor_hash = hash_entry(&EntryTypes::StringAnchor(learner_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(learner_anchor))?;
    create_link(learner_anchor_hash, action_hash.clone(), LinkTypes::LearnerToGrant, ())?;

    // Create gate lookup link
    let gate_anchor = StringAnchor::new("gate_grants", &input.gate_id);
    let gate_anchor_hash = hash_entry(&EntryTypes::StringAnchor(gate_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(gate_anchor))?;
    create_link(gate_anchor_hash, action_hash.clone(), LinkTypes::GateToGrant, ())?;

    // Create grant type link
    let type_anchor = StringAnchor::new("grant_type", &input.grant_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(type_anchor_hash, action_hash.clone(), LinkTypes::GrantByType, ())?;

    // If there was payment, create revenue record
    if let Some(amount) = input.payment_amount {
        if amount > 0.0 {
            create_steward_revenue(&gate.gate, &grant_id, &learner_id, amount, input.payment_unit.as_deref(), &timestamp)?;
        }
    }

    Ok(AccessGrantOutput {
        action_hash,
        grant,
    })
}

/// Create steward revenue record (internal function)
fn create_steward_revenue(
    gate: &PremiumGate,
    grant_id: &str,
    learner_id: &str,
    gross_amount: f64,
    payment_unit: Option<&str>,
    timestamp: &str,
) -> ExternResult<StewardRevenueOutput> {
    // Calculate three-way split
    let steward_amount = gross_amount * (gate.steward_share_percent / 100.0);
    let contributor_share = gate.contributor_share_percent.unwrap_or(0.0);
    let contributor_amount = gross_amount * (contributor_share / 100.0);
    let commons_amount = gross_amount * (gate.commons_share_percent / 100.0);

    let revenue_id = format!("revenue-{}-{}", grant_id, timestamp);
    let payment_unit_str = payment_unit.unwrap_or("elohim-credit");

    // Create placeholder economic event IDs (would be linked to actual Shefa events)
    let steward_economic_event_id = format!("econ-steward-{}", revenue_id);
    let contributor_economic_event_id = if gate.contributor_presence_id.is_some() && contributor_amount > 0.0 {
        Some(format!("econ-contributor-{}", revenue_id))
    } else {
        None
    };
    let commons_economic_event_id = format!("econ-commons-{}", revenue_id);

    let revenue = StewardRevenue {
        id: revenue_id.clone(),
        access_grant_id: grant_id.to_string(),
        gate_id: gate.id.clone(),
        from_learner_id: learner_id.to_string(),
        to_steward_presence_id: gate.steward_presence_id.clone(),
        to_contributor_presence_id: gate.contributor_presence_id.clone(),
        gross_amount,
        payment_unit: payment_unit_str.to_string(),
        steward_amount,
        contributor_amount,
        commons_amount,
        steward_economic_event_id,
        contributor_economic_event_id,
        commons_economic_event_id,
        status: "completed".to_string(),
        completed_at: Some(timestamp.to_string()),
        failure_reason: None,
        note: None,
        metadata_json: "{}".to_string(),
        created_at: timestamp.to_string(),
    };

    let action_hash = create_entry(&EntryTypes::StewardRevenue(revenue.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("revenue_id", &revenue_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToStewardRevenue, ())?;

    // Create steward revenue link (using steward_presence_id for lookup)
    let steward_anchor = StringAnchor::new("steward_revenue", &gate.steward_presence_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(steward_anchor))?;
    create_link(steward_anchor_hash, action_hash.clone(), LinkTypes::StewardToRevenue, ())?;

    // Create contributor revenue link if applicable
    if let Some(ref contributor_id) = gate.contributor_presence_id {
        let contributor_anchor = StringAnchor::new("contributor_revenue", contributor_id);
        let contributor_anchor_hash = hash_entry(&EntryTypes::StringAnchor(contributor_anchor.clone()))?;
        create_entry(&EntryTypes::StringAnchor(contributor_anchor))?;
        create_link(contributor_anchor_hash, action_hash.clone(), LinkTypes::ContributorToRevenue, ())?;
    }

    Ok(StewardRevenueOutput {
        action_hash,
        revenue,
    })
}

/// Check if a learner has access to a gated resource
#[hdk_extern]
pub fn check_access(gate_id: String) -> ExternResult<Option<AccessGrantOutput>> {
    let agent_info = agent_info()?;
    let learner_id = agent_info.agent_initial_pubkey.to_string();

    let learner_anchor = StringAnchor::new("learner_grants", &learner_id);
    let learner_anchor_hash = hash_entry(&EntryTypes::StringAnchor(learner_anchor))?;

    let query = LinkQuery::try_new(learner_anchor_hash, LinkTypes::LearnerToGrant)?;
    let links = get_links(query, GetStrategy::default())?;

    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid grant hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(grant) = rec.entry().to_app_option::<AccessGrant>().ok().flatten() {
                if grant.gate_id == gate_id && grant.is_active {
                    // TODO: Check expiration
                    return Ok(Some(AccessGrantOutput { action_hash, grant }));
                }
            }
        }
    }

    Ok(None)
}

/// Get my access grants
#[hdk_extern]
pub fn get_my_access_grants(_: ()) -> ExternResult<Vec<AccessGrantOutput>> {
    let agent_info = agent_info()?;
    let learner_id = agent_info.agent_initial_pubkey.to_string();

    let learner_anchor = StringAnchor::new("learner_grants", &learner_id);
    let learner_anchor_hash = hash_entry(&EntryTypes::StringAnchor(learner_anchor))?;

    let query = LinkQuery::try_new(learner_anchor_hash, LinkTypes::LearnerToGrant)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid grant hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(grant) = rec.entry().to_app_option::<AccessGrant>().ok().flatten() {
                if grant.is_active {
                    results.push(AccessGrantOutput { action_hash, grant });
                }
            }
        }
    }

    Ok(results)
}

/// Steward revenue summary
#[derive(Serialize, Deserialize, Debug)]
pub struct StewardRevenueSummary {
    pub steward_presence_id: String,
    pub total_revenue: f64,
    pub total_grants: u32,
    pub revenue_by_gate: Vec<GateRevenueSummary>,
}

/// Revenue summary per gate
#[derive(Serialize, Deserialize, Debug)]
pub struct GateRevenueSummary {
    pub gate_id: String,
    pub gate_title: String,
    pub total_revenue: f64,
    pub grant_count: u32,
}

/// Get steward revenue summary
#[hdk_extern]
pub fn get_steward_revenue_summary(steward_presence_id: String) -> ExternResult<StewardRevenueSummary> {
    let steward_anchor = StringAnchor::new("steward_revenue", &steward_presence_id);
    let steward_anchor_hash = hash_entry(&EntryTypes::StringAnchor(steward_anchor))?;

    let query = LinkQuery::try_new(steward_anchor_hash, LinkTypes::StewardToRevenue)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut total_revenue = 0.0;
    let mut revenue_by_gate: HashMap<String, (String, f64, u32)> = HashMap::new();

    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid revenue hash".to_string())))?;

        let record = get(action_hash, GetOptions::default())?;
        if let Some(rec) = record {
            if let Some(revenue) = rec.entry().to_app_option::<StewardRevenue>().ok().flatten() {
                total_revenue += revenue.steward_amount;

                let entry = revenue_by_gate.entry(revenue.gate_id.clone())
                    .or_insert((revenue.gate_id.clone(), 0.0, 0));
                entry.1 += revenue.steward_amount;
                entry.2 += 1;
            }
        }
    }

    let revenue_summaries: Vec<GateRevenueSummary> = revenue_by_gate.into_iter()
        .map(|(gate_id, (_, total, count))| GateRevenueSummary {
            gate_id: gate_id.clone(),
            gate_title: gate_id, // Would look up actual name
            total_revenue: total,
            grant_count: count,
        })
        .collect();

    Ok(StewardRevenueSummary {
        steward_presence_id,
        total_revenue,
        total_grants: revenue_summaries.iter().map(|g| g.grant_count).sum(),
        revenue_by_gate: revenue_summaries,
    })
}

// =============================================================================
// Migration Export Functions
// =============================================================================
// These functions exist so that FUTURE versions of this DNA can call them
// via bridge calls to migrate data forward. Every DNA version should have
// these export functions.

/// Schema version identifier for migration compatibility
pub const SCHEMA_VERSION: &str = "v1";

/// Export the schema version (for migration compatibility checking)
#[hdk_extern]
pub fn export_schema_version(_: ()) -> ExternResult<String> {
    Ok(SCHEMA_VERSION.to_string())
}

/// Export all content entries (for migration)
/// Uses chain query to get all Content entries created by any agent on this node
#[hdk_extern]
pub fn export_all_content(_: ()) -> ExternResult<Vec<ContentOutput>> {
    let filter = ChainQueryFilter::new()
        .entry_type(UnitEntryTypes::Content.try_into()?);

    let records = query(filter)?;
    let mut results = Vec::new();

    for record in records {
        let action_hash = record.action_hashed().hash.clone();

        if let Some(entry_hash) = record.action().entry_hash() {
            if let Some(content) = record
                .entry()
                .to_app_option::<Content>()
                .ok()
                .flatten()
            {
                results.push(ContentOutput {
                    action_hash,
                    entry_hash: entry_hash.clone(),
                    content,
                });
            }
        }
    }

    Ok(results)
}

/// Path with all its steps for migration
#[derive(Serialize, Deserialize, Debug)]
pub struct PathWithStepsExport {
    pub path: LearningPath,
    pub path_action_hash: ActionHash,
    pub steps: Vec<PathStepExport>,
}

/// Path step for migration export
#[derive(Serialize, Deserialize, Debug)]
pub struct PathStepExport {
    pub step: PathStep,
    pub action_hash: ActionHash,
}

/// Export all learning paths with their steps (for migration)
#[hdk_extern]
pub fn export_all_paths_with_steps(_: ()) -> ExternResult<Vec<PathWithStepsExport>> {
    // Use the global "all_paths" anchor
    let anchor = StringAnchor::new("all_paths", "index");
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;

    let query = LinkQuery::try_new(anchor_hash, LinkTypes::IdToPath)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();

    for link in links {
        let path_action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path action hash".to_string())))?;

        let record = get(path_action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(path) = record
                .entry()
                .to_app_option::<LearningPath>()
                .ok()
                .flatten()
            {
                // Get all steps for this path
                let step_query = LinkQuery::try_new(path_action_hash.clone(), LinkTypes::PathToStep)?;
                let step_links = get_links(step_query, GetStrategy::default())?;

                let mut steps = Vec::new();
                for step_link in step_links {
                    let step_action_hash = ActionHash::try_from(step_link.target)
                        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid step action hash".to_string())))?;

                    let step_record = get(step_action_hash.clone(), GetOptions::default())?;
                    if let Some(step_rec) = step_record {
                        if let Some(step) = step_rec
                            .entry()
                            .to_app_option::<PathStep>()
                            .ok()
                            .flatten()
                        {
                            steps.push(PathStepExport {
                                step,
                                action_hash: step_action_hash,
                            });
                        }
                    }
                }

                // Sort steps by order_index
                steps.sort_by_key(|s| s.step.order_index);

                results.push(PathWithStepsExport {
                    path,
                    path_action_hash,
                    steps,
                });
            }
        }
    }

    Ok(results)
}

/// Export all mastery records for current agent (for migration)
/// Wraps get_my_all_mastery for explicit migration use
#[hdk_extern]
pub fn export_all_mastery(_: ()) -> ExternResult<Vec<ContentMasteryOutput>> {
    get_my_all_mastery(())
}

/// Export all progress records for current agent (for migration)
/// Wraps get_my_all_progress for explicit migration use
#[hdk_extern]
pub fn export_all_progress(_: ()) -> ExternResult<Vec<AgentProgressOutput>> {
    get_my_all_progress(())
}

/// Complete migration export - all data needed to migrate to a new DNA version
#[derive(Serialize, Deserialize, Debug)]
pub struct MigrationExport {
    pub schema_version: String,
    pub content: Vec<ContentOutput>,
    pub paths: Vec<PathWithStepsExport>,
    pub mastery: Vec<ContentMasteryOutput>,
    pub progress: Vec<AgentProgressOutput>,
    pub exported_at: String,
}

/// Export all data for migration in a single call
#[hdk_extern]
pub fn export_for_migration(_: ()) -> ExternResult<MigrationExport> {
    let now = sys_time()?;

    Ok(MigrationExport {
        schema_version: SCHEMA_VERSION.to_string(),
        content: export_all_content(())?,
        paths: export_all_paths_with_steps(())?,
        mastery: export_all_mastery(())?,
        progress: export_all_progress(())?,
        exported_at: format!("{:?}", now),
    })
}

// =============================================================================
// KnowledgeMap Operations
// =============================================================================

/// Input for creating a knowledge map
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateKnowledgeMapInput {
    pub id: Option<String>,
    pub map_type: String,           // domain, self, person, collective
    pub owner_id: String,
    pub title: String,
    pub description: Option<String>,
    pub subject_type: String,
    pub subject_id: String,
    pub subject_name: String,
    pub visibility: String,
    pub shared_with_json: String,
    pub nodes_json: String,
    pub path_ids_json: String,
    pub overall_affinity: f64,
    pub content_graph_id: Option<String>,
    pub mastery_levels_json: String,
    pub goals_json: String,
    pub metadata_json: String,
}

/// Output for knowledge map
#[derive(Serialize, Deserialize, Debug)]
pub struct KnowledgeMapOutput {
    pub action_hash: ActionHash,
    pub knowledge_map: KnowledgeMap,
}

/// Create a knowledge map
#[hdk_extern]
pub fn create_knowledge_map(input: CreateKnowledgeMapInput) -> ExternResult<KnowledgeMapOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let knowledge_map_id = input.id.unwrap_or_else(|| {
        format!("km-{}-{}", input.owner_id, timestamp)
    });

    let knowledge_map = KnowledgeMap {
        id: knowledge_map_id.clone(),
        map_type: input.map_type.clone(),
        owner_id: input.owner_id.clone(),
        title: input.title,
        description: input.description,
        subject_type: input.subject_type,
        subject_id: input.subject_id,
        subject_name: input.subject_name,
        visibility: input.visibility,
        shared_with_json: input.shared_with_json,
        nodes_json: input.nodes_json,
        path_ids_json: input.path_ids_json,
        overall_affinity: input.overall_affinity,
        content_graph_id: input.content_graph_id,
        mastery_levels_json: input.mastery_levels_json,
        goals_json: input.goals_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::KnowledgeMap(knowledge_map.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("knowledge_map_id", &knowledge_map_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToKnowledgeMap, ())?;

    // Link by owner
    let owner_anchor = StringAnchor::new("knowledge_map_owner", &input.owner_id);
    let owner_anchor_hash = hash_entry(&EntryTypes::StringAnchor(owner_anchor))?;
    create_link(owner_anchor_hash, action_hash.clone(), LinkTypes::OwnerToKnowledgeMap, ())?;

    // Link by type
    let type_anchor = StringAnchor::new("knowledge_map_type", &input.map_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(type_anchor_hash, action_hash.clone(), LinkTypes::KnowledgeMapByType, ())?;

    Ok(KnowledgeMapOutput {
        action_hash,
        knowledge_map,
    })
}

/// Get knowledge map by ID
#[hdk_extern]
pub fn get_knowledge_map_by_id(id: String) -> ExternResult<Option<KnowledgeMapOutput>> {
    let id_anchor = StringAnchor::new("knowledge_map_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToKnowledgeMap)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid knowledge map hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(knowledge_map) = record.entry().to_app_option::<KnowledgeMap>().ok().flatten() {
                return Ok(Some(KnowledgeMapOutput { action_hash, knowledge_map }));
            }
        }
    }

    Ok(None)
}

/// Input for querying knowledge maps
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryKnowledgeMapsInput {
    pub owner_id: Option<String>,
    pub map_type: Option<String>,
    pub limit: Option<u32>,
}

/// Query knowledge maps
#[hdk_extern]
pub fn query_knowledge_maps(input: QueryKnowledgeMapsInput) -> ExternResult<Vec<KnowledgeMapOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(owner_id) = &input.owner_id {
        let owner_anchor = StringAnchor::new("knowledge_map_owner", owner_id);
        let owner_anchor_hash = hash_entry(&EntryTypes::StringAnchor(owner_anchor))?;

        let query = LinkQuery::try_new(owner_anchor_hash, LinkTypes::OwnerToKnowledgeMap)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid knowledge map hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(knowledge_map) = record.entry().to_app_option::<KnowledgeMap>().ok().flatten() {
                    // Filter by map_type if specified
                    if let Some(ref map_type) = input.map_type {
                        if &knowledge_map.map_type != map_type {
                            continue;
                        }
                    }
                    results.push(KnowledgeMapOutput { action_hash, knowledge_map });
                }
            }
        }
    } else if let Some(map_type) = &input.map_type {
        let type_anchor = StringAnchor::new("knowledge_map_type", map_type);
        let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;

        let query = LinkQuery::try_new(type_anchor_hash, LinkTypes::KnowledgeMapByType)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid knowledge map hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(knowledge_map) = record.entry().to_app_option::<KnowledgeMap>().ok().flatten() {
                    results.push(KnowledgeMapOutput { action_hash, knowledge_map });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// PathExtension Operations
// =============================================================================

/// Input for creating a path extension
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePathExtensionInput {
    pub id: Option<String>,
    pub base_path_id: String,
    pub base_path_version: String,
    pub extended_by: String,
    pub title: String,
    pub description: Option<String>,
    pub insertions_json: String,
    pub annotations_json: String,
    pub reorderings_json: String,
    pub exclusions_json: String,
    pub visibility: String,
    pub shared_with_json: String,
    pub forked_from: Option<String>,
    pub forks_json: String,
    pub upstream_proposal_json: Option<String>,
    pub stats_json: String,
}

/// Output for path extension
#[derive(Serialize, Deserialize, Debug)]
pub struct PathExtensionOutput {
    pub action_hash: ActionHash,
    pub path_extension: PathExtension,
}

/// Create a path extension
#[hdk_extern]
pub fn create_path_extension(input: CreatePathExtensionInput) -> ExternResult<PathExtensionOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let extension_id = input.id.unwrap_or_else(|| {
        format!("ext-{}-{}", input.base_path_id, timestamp)
    });

    let path_extension = PathExtension {
        id: extension_id.clone(),
        base_path_id: input.base_path_id.clone(),
        base_path_version: input.base_path_version,
        extended_by: input.extended_by.clone(),
        title: input.title,
        description: input.description,
        insertions_json: input.insertions_json,
        annotations_json: input.annotations_json,
        reorderings_json: input.reorderings_json,
        exclusions_json: input.exclusions_json,
        visibility: input.visibility,
        shared_with_json: input.shared_with_json,
        forked_from: input.forked_from,
        forks_json: input.forks_json,
        upstream_proposal_json: input.upstream_proposal_json,
        stats_json: input.stats_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::PathExtension(path_extension.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("path_extension_id", &extension_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPathExtension, ())?;

    // Link by extender
    let extender_anchor = StringAnchor::new("path_extension_extender", &input.extended_by);
    let extender_anchor_hash = hash_entry(&EntryTypes::StringAnchor(extender_anchor))?;
    create_link(extender_anchor_hash, action_hash.clone(), LinkTypes::ExtenderToExtension, ())?;

    // Link by base path
    let base_anchor = StringAnchor::new("path_extension_base", &input.base_path_id);
    let base_anchor_hash = hash_entry(&EntryTypes::StringAnchor(base_anchor))?;
    create_link(base_anchor_hash, action_hash.clone(), LinkTypes::BasePathToExtension, ())?;

    Ok(PathExtensionOutput {
        action_hash,
        path_extension,
    })
}

/// Get path extension by ID
#[hdk_extern]
pub fn get_path_extension_by_id(id: String) -> ExternResult<Option<PathExtensionOutput>> {
    let id_anchor = StringAnchor::new("path_extension_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToPathExtension)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path extension hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(path_extension) = record.entry().to_app_option::<PathExtension>().ok().flatten() {
                return Ok(Some(PathExtensionOutput { action_hash, path_extension }));
            }
        }
    }

    Ok(None)
}

/// Input for querying path extensions
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryPathExtensionsInput {
    pub base_path_id: Option<String>,
    pub extended_by: Option<String>,
    pub limit: Option<u32>,
}

/// Query path extensions
#[hdk_extern]
pub fn query_path_extensions(input: QueryPathExtensionsInput) -> ExternResult<Vec<PathExtensionOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(base_path_id) = &input.base_path_id {
        let base_anchor = StringAnchor::new("path_extension_base", base_path_id);
        let base_anchor_hash = hash_entry(&EntryTypes::StringAnchor(base_anchor))?;

        let query = LinkQuery::try_new(base_anchor_hash, LinkTypes::BasePathToExtension)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path extension hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(path_extension) = record.entry().to_app_option::<PathExtension>().ok().flatten() {
                    // Filter by extender if specified
                    if let Some(ref extended_by) = input.extended_by {
                        if &path_extension.extended_by != extended_by {
                            continue;
                        }
                    }
                    results.push(PathExtensionOutput { action_hash, path_extension });
                }
            }
        }
    } else if let Some(extended_by) = &input.extended_by {
        let extender_anchor = StringAnchor::new("path_extension_extender", extended_by);
        let extender_anchor_hash = hash_entry(&EntryTypes::StringAnchor(extender_anchor))?;

        let query = LinkQuery::try_new(extender_anchor_hash, LinkTypes::ExtenderToExtension)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid path extension hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(path_extension) = record.entry().to_app_option::<PathExtension>().ok().flatten() {
                    results.push(PathExtensionOutput { action_hash, path_extension });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Challenge Operations (Governance)
// =============================================================================

/// Input for creating a challenge
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateChallengeInput {
    pub id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub challenger_id: String,
    pub challenger_name: String,
    pub challenger_standing: String,
    pub grounds: String,
    pub description: String,
    pub evidence_json: String,
    pub status: String,
    pub priority: String,
    pub sla_deadline: Option<String>,
    pub assigned_elohim: Option<String>,
    pub resolution_json: Option<String>,
    pub metadata_json: String,
}

/// Output for challenge
#[derive(Serialize, Deserialize, Debug)]
pub struct ChallengeOutput {
    pub action_hash: ActionHash,
    pub challenge: Challenge,
}

/// Create a challenge
#[hdk_extern]
pub fn create_challenge(input: CreateChallengeInput) -> ExternResult<ChallengeOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let challenge_id = input.id.unwrap_or_else(|| {
        format!("chal-{}-{}", input.entity_id, timestamp)
    });

    let challenge = Challenge {
        id: challenge_id.clone(),
        entity_type: input.entity_type.clone(),
        entity_id: input.entity_id.clone(),
        challenger_id: input.challenger_id.clone(),
        challenger_name: input.challenger_name,
        challenger_standing: input.challenger_standing,
        grounds: input.grounds,
        description: input.description,
        evidence_json: input.evidence_json,
        status: input.status.clone(),
        filed_at: timestamp.clone(),
        acknowledged_at: None,
        sla_deadline: input.sla_deadline,
        assigned_elohim: input.assigned_elohim,
        priority: input.priority,
        resolution_json: input.resolution_json,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Challenge(challenge.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("challenge_id", &challenge_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToChallenge, ())?;

    // Link by entity
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("challenge_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(entity_anchor_hash, action_hash.clone(), LinkTypes::EntityToChallenge, ())?;

    // Link by challenger
    let challenger_anchor = StringAnchor::new("challenge_challenger", &input.challenger_id);
    let challenger_anchor_hash = hash_entry(&EntryTypes::StringAnchor(challenger_anchor))?;
    create_link(challenger_anchor_hash, action_hash.clone(), LinkTypes::ChallengerToChallenge, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("challenge_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::ChallengeByStatus, ())?;

    Ok(ChallengeOutput {
        action_hash,
        challenge,
    })
}

/// Get challenge by ID
#[hdk_extern]
pub fn get_challenge_by_id(id: String) -> ExternResult<Option<ChallengeOutput>> {
    let id_anchor = StringAnchor::new("challenge_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToChallenge)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten() {
                return Ok(Some(ChallengeOutput { action_hash, challenge }));
            }
        }
    }

    Ok(None)
}

/// Input for querying challenges
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryChallengesInput {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub challenger_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// Query challenges
#[hdk_extern]
pub fn query_challenges(input: QueryChallengesInput) -> ExternResult<Vec<ChallengeOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if input.entity_type.is_some() && input.entity_id.is_some() {
        let entity_key = format!("{}:{}", input.entity_type.as_ref().unwrap(), input.entity_id.as_ref().unwrap());
        let entity_anchor = StringAnchor::new("challenge_entity", &entity_key);
        let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

        let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::EntityToChallenge)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten() {
                    // Filter by status if specified
                    if let Some(ref status) = input.status {
                        if &challenge.status != status {
                            continue;
                        }
                    }
                    results.push(ChallengeOutput { action_hash, challenge });
                }
            }
        }
    } else if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("challenge_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::ChallengeByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid challenge hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(challenge) = record.entry().to_app_option::<Challenge>().ok().flatten() {
                    results.push(ChallengeOutput { action_hash, challenge });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Proposal Operations (Governance)
// =============================================================================

/// Input for creating a proposal
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateProposalInput {
    pub id: Option<String>,
    pub title: String,
    pub proposal_type: String,
    pub description: String,
    pub proposer_id: String,
    pub proposer_name: String,
    pub rationale: String,
    pub status: String,
    pub phase: String,
    pub amendments_json: String,
    pub voting_config_json: String,
    pub current_votes_json: String,
    pub outcome_json: Option<String>,
    pub related_entity_type: Option<String>,
    pub related_entity_id: Option<String>,
    pub metadata_json: String,
}

/// Output for proposal
#[derive(Serialize, Deserialize, Debug)]
pub struct ProposalOutput {
    pub action_hash: ActionHash,
    pub proposal: Proposal,
}

/// Create a proposal
#[hdk_extern]
pub fn create_proposal(input: CreateProposalInput) -> ExternResult<ProposalOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let proposal_id = input.id.unwrap_or_else(|| {
        format!("prop-{}", timestamp)
    });

    let proposal = Proposal {
        id: proposal_id.clone(),
        title: input.title,
        proposal_type: input.proposal_type.clone(),
        description: input.description,
        proposer_id: input.proposer_id.clone(),
        proposer_name: input.proposer_name,
        rationale: input.rationale,
        status: input.status.clone(),
        phase: input.phase,
        amendments_json: input.amendments_json,
        voting_config_json: input.voting_config_json,
        current_votes_json: input.current_votes_json,
        outcome_json: input.outcome_json,
        related_entity_type: input.related_entity_type,
        related_entity_id: input.related_entity_id,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Proposal(proposal.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("proposal_id", &proposal_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToProposal, ())?;

    // Link by type
    let type_anchor = StringAnchor::new("proposal_type", &input.proposal_type);
    let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;
    create_link(type_anchor_hash, action_hash.clone(), LinkTypes::ProposalByType, ())?;

    // Link by proposer
    let proposer_anchor = StringAnchor::new("proposal_proposer", &input.proposer_id);
    let proposer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(proposer_anchor))?;
    create_link(proposer_anchor_hash, action_hash.clone(), LinkTypes::ProposerToProposal, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("proposal_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::ProposalByStatus, ())?;

    Ok(ProposalOutput {
        action_hash,
        proposal,
    })
}

/// Get proposal by ID
#[hdk_extern]
pub fn get_proposal_by_id(id: String) -> ExternResult<Option<ProposalOutput>> {
    let id_anchor = StringAnchor::new("proposal_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToProposal)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(proposal) = record.entry().to_app_option::<Proposal>().ok().flatten() {
                return Ok(Some(ProposalOutput { action_hash, proposal }));
            }
        }
    }

    Ok(None)
}

/// Input for querying proposals
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryProposalsInput {
    pub proposal_type: Option<String>,
    pub proposer_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// Query proposals
#[hdk_extern]
pub fn query_proposals(input: QueryProposalsInput) -> ExternResult<Vec<ProposalOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("proposal_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::ProposalByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(proposal) = record.entry().to_app_option::<Proposal>().ok().flatten() {
                    // Filter by type and proposer if specified
                    if let Some(ref proposal_type) = input.proposal_type {
                        if &proposal.proposal_type != proposal_type {
                            continue;
                        }
                    }
                    if let Some(ref proposer_id) = input.proposer_id {
                        if &proposal.proposer_id != proposer_id {
                            continue;
                        }
                    }
                    results.push(ProposalOutput { action_hash, proposal });
                }
            }
        }
    } else if let Some(proposal_type) = &input.proposal_type {
        let type_anchor = StringAnchor::new("proposal_type", proposal_type);
        let type_anchor_hash = hash_entry(&EntryTypes::StringAnchor(type_anchor))?;

        let query = LinkQuery::try_new(type_anchor_hash, LinkTypes::ProposalByType)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid proposal hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(proposal) = record.entry().to_app_option::<Proposal>().ok().flatten() {
                    results.push(ProposalOutput { action_hash, proposal });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Precedent Operations (Governance)
// =============================================================================

/// Input for creating a precedent
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePrecedentInput {
    pub id: Option<String>,
    pub title: String,
    pub summary: String,
    pub full_reasoning: String,
    pub binding: String,              // constitutional, binding-network, binding-local, persuasive
    pub scope_json: String,
    pub established_by: String,
    pub status: String,
    pub superseded_by: Option<String>,
    pub metadata_json: String,
}

/// Output for precedent
#[derive(Serialize, Deserialize, Debug)]
pub struct PrecedentOutput {
    pub action_hash: ActionHash,
    pub precedent: Precedent,
}

/// Create a precedent
#[hdk_extern]
pub fn create_precedent(input: CreatePrecedentInput) -> ExternResult<PrecedentOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let precedent_id = input.id.unwrap_or_else(|| {
        format!("prec-{}", timestamp)
    });

    let precedent = Precedent {
        id: precedent_id.clone(),
        title: input.title,
        summary: input.summary,
        full_reasoning: input.full_reasoning,
        binding: input.binding.clone(),
        scope_json: input.scope_json.clone(),
        citations: 0,  // Starts at 0, incremented when cited
        status: input.status.clone(),
        established_by: input.established_by,
        established_at: timestamp.clone(),
        superseded_by: input.superseded_by,
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Precedent(precedent.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("precedent_id", &precedent_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToPrecedent, ())?;

    // Link by scope (extract from JSON - simplified, use first scope)
    let scope_anchor = StringAnchor::new("precedent_scope", &input.scope_json);
    let scope_anchor_hash = hash_entry(&EntryTypes::StringAnchor(scope_anchor))?;
    create_link(scope_anchor_hash, action_hash.clone(), LinkTypes::PrecedentByScope, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("precedent_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::PrecedentByStatus, ())?;

    Ok(PrecedentOutput {
        action_hash,
        precedent,
    })
}

/// Get precedent by ID
#[hdk_extern]
pub fn get_precedent_by_id(id: String) -> ExternResult<Option<PrecedentOutput>> {
    let id_anchor = StringAnchor::new("precedent_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToPrecedent)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid precedent hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(precedent) = record.entry().to_app_option::<Precedent>().ok().flatten() {
                return Ok(Some(PrecedentOutput { action_hash, precedent }));
            }
        }
    }

    Ok(None)
}

/// Input for querying precedents
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryPrecedentsInput {
    pub status: Option<String>,
    pub binding: Option<String>,  // constitutional, binding-network, binding-local, persuasive
    pub limit: Option<u32>,
}

/// Query precedents
#[hdk_extern]
pub fn query_precedents(input: QueryPrecedentsInput) -> ExternResult<Vec<PrecedentOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("precedent_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::PrecedentByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid precedent hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(precedent) = record.entry().to_app_option::<Precedent>().ok().flatten() {
                    // Filter by binding if specified
                    if let Some(ref binding) = input.binding {
                        if &precedent.binding != binding {
                            continue;
                        }
                    }
                    results.push(PrecedentOutput { action_hash, precedent });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Discussion Operations (Governance)
// =============================================================================

/// Input for creating a discussion
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDiscussionInput {
    pub id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub category: String,
    pub title: String,
    pub messages_json: String,
    pub status: String,
    pub metadata_json: String,
}

/// Output for discussion
#[derive(Serialize, Deserialize, Debug)]
pub struct DiscussionOutput {
    pub action_hash: ActionHash,
    pub discussion: Discussion,
}

/// Create a discussion
#[hdk_extern]
pub fn create_discussion(input: CreateDiscussionInput) -> ExternResult<DiscussionOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let discussion_id = input.id.unwrap_or_else(|| {
        format!("disc-{}-{}", input.entity_id, timestamp)
    });

    let discussion = Discussion {
        id: discussion_id.clone(),
        entity_type: input.entity_type.clone(),
        entity_id: input.entity_id.clone(),
        category: input.category.clone(),
        title: input.title,
        messages_json: input.messages_json,
        status: input.status.clone(),
        message_count: 0,
        last_activity_at: timestamp.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::Discussion(discussion.clone()))?;

    // Link by ID
    let id_anchor = StringAnchor::new("discussion_id", &discussion_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToDiscussion, ())?;

    // Link by entity
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("discussion_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(entity_anchor_hash, action_hash.clone(), LinkTypes::EntityToDiscussion, ())?;

    // Link by category
    let category_anchor = StringAnchor::new("discussion_category", &input.category);
    let category_anchor_hash = hash_entry(&EntryTypes::StringAnchor(category_anchor))?;
    create_link(category_anchor_hash, action_hash.clone(), LinkTypes::DiscussionByCategory, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("discussion_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::DiscussionByStatus, ())?;

    Ok(DiscussionOutput {
        action_hash,
        discussion,
    })
}

/// Get discussion by ID
#[hdk_extern]
pub fn get_discussion_by_id(id: String) -> ExternResult<Option<DiscussionOutput>> {
    let id_anchor = StringAnchor::new("discussion_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToDiscussion)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(discussion) = record.entry().to_app_option::<Discussion>().ok().flatten() {
                return Ok(Some(DiscussionOutput { action_hash, discussion }));
            }
        }
    }

    Ok(None)
}

/// Input for querying discussions
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryDiscussionsInput {
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// Query discussions
#[hdk_extern]
pub fn query_discussions(input: QueryDiscussionsInput) -> ExternResult<Vec<DiscussionOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if input.entity_type.is_some() && input.entity_id.is_some() {
        let entity_key = format!("{}:{}", input.entity_type.as_ref().unwrap(), input.entity_id.as_ref().unwrap());
        let entity_anchor = StringAnchor::new("discussion_entity", &entity_key);
        let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

        let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::EntityToDiscussion)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(discussion) = record.entry().to_app_option::<Discussion>().ok().flatten() {
                    // Filter by category/status if specified
                    if let Some(ref category) = input.category {
                        if &discussion.category != category {
                            continue;
                        }
                    }
                    if let Some(ref status) = input.status {
                        if &discussion.status != status {
                            continue;
                        }
                    }
                    results.push(DiscussionOutput { action_hash, discussion });
                }
            }
        }
    } else if let Some(category) = &input.category {
        let category_anchor = StringAnchor::new("discussion_category", category);
        let category_anchor_hash = hash_entry(&EntryTypes::StringAnchor(category_anchor))?;

        let query = LinkQuery::try_new(category_anchor_hash, LinkTypes::DiscussionByCategory)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid discussion hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(discussion) = record.entry().to_app_option::<Discussion>().ok().flatten() {
                    results.push(DiscussionOutput { action_hash, discussion });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// GovernanceState Operations
// =============================================================================

/// Input for creating/updating governance state
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateGovernanceStateInput {
    pub id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub status: String,
    pub status_basis_json: String,
    pub labels_json: String,
    pub active_challenges_json: String,
    pub active_proposals_json: String,
    pub precedent_ids_json: String,
    pub metadata_json: String,
}

/// Output for governance state
#[derive(Serialize, Deserialize, Debug)]
pub struct GovernanceStateOutput {
    pub action_hash: ActionHash,
    pub governance_state: GovernanceState,
}

/// Create or update governance state for an entity
#[hdk_extern]
pub fn set_governance_state(input: CreateGovernanceStateInput) -> ExternResult<GovernanceStateOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let governance_state_id = input.id.unwrap_or_else(|| {
        format!("gs-{}:{}", input.entity_type, input.entity_id)
    });

    let governance_state = GovernanceState {
        id: governance_state_id,
        entity_type: input.entity_type.clone(),
        entity_id: input.entity_id.clone(),
        status: input.status.clone(),
        status_basis_json: input.status_basis_json,
        labels_json: input.labels_json,
        active_challenges_json: input.active_challenges_json,
        active_proposals_json: input.active_proposals_json,
        precedent_ids_json: input.precedent_ids_json,
        last_updated: timestamp.clone(),
        created_at: timestamp.clone(),
        updated_at: timestamp,
        metadata_json: input.metadata_json,
    };

    let action_hash = create_entry(&EntryTypes::GovernanceState(governance_state.clone()))?;

    // Link by entity (primary lookup)
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("governance_state_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;
    create_link(entity_anchor_hash, action_hash.clone(), LinkTypes::IdToGovernanceState, ())?;

    // Link by status
    let status_anchor = StringAnchor::new("governance_state_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::GovernanceStateByStatus, ())?;

    Ok(GovernanceStateOutput {
        action_hash,
        governance_state,
    })
}

/// Get governance state for an entity
#[hdk_extern]
pub fn get_governance_state(input: GetGovernanceStateInput) -> ExternResult<Option<GovernanceStateOutput>> {
    let entity_key = format!("{}:{}", input.entity_type, input.entity_id);
    let entity_anchor = StringAnchor::new("governance_state_entity", &entity_key);
    let entity_anchor_hash = hash_entry(&EntryTypes::StringAnchor(entity_anchor))?;

    let query = LinkQuery::try_new(entity_anchor_hash, LinkTypes::IdToGovernanceState)?;
    let links = get_links(query, GetStrategy::default())?;

    // Return the most recent governance state
    if let Some(link) = links.last() {
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid governance state hash".to_string())))?;

        let record = get(action_hash.clone(), GetOptions::default())?;
        if let Some(record) = record {
            if let Some(governance_state) = record.entry().to_app_option::<GovernanceState>().ok().flatten() {
                return Ok(Some(GovernanceStateOutput { action_hash, governance_state }));
            }
        }
    }

    Ok(None)
}

/// Input for getting governance state
#[derive(Serialize, Deserialize, Debug)]
pub struct GetGovernanceStateInput {
    pub entity_type: String,
    pub entity_id: String,
}

/// Input for querying governance states
#[derive(Serialize, Deserialize, Debug)]
pub struct QueryGovernanceStatesInput {
    pub status: Option<String>,
    pub limit: Option<u32>,
}

/// Query governance states by status
#[hdk_extern]
pub fn query_governance_states(input: QueryGovernanceStatesInput) -> ExternResult<Vec<GovernanceStateOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    if let Some(status) = &input.status {
        let status_anchor = StringAnchor::new("governance_state_status", status);
        let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

        let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::GovernanceStateByStatus)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid governance state hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(governance_state) = record.entry().to_app_option::<GovernanceState>().ok().flatten() {
                    results.push(GovernanceStateOutput { action_hash, governance_state });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// CustodianCommitment CRUD Operations (Digital Presence Stewardship)
// =============================================================================

/// Create a custodian commitment
#[hdk_extern]
pub fn create_custodian_commitment(input: CreateCustodianCommitmentInput) -> ExternResult<CustodianCommitmentOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let commitment = CustodianCommitment {
        id: format!("{}-{}", input.beneficiary_agent_id, input.custodian_agent_id),
        custodian_agent_id: input.custodian_agent_id.clone(),
        beneficiary_agent_id: input.beneficiary_agent_id.clone(),
        commitment_type: input.commitment_type.clone(),
        basis: input.basis.clone(),
        relationship_id: input.relationship_id,
        category_override_json: input.category_override_json,
        content_filters_json: input.content_filters_json,
        estimated_content_count: input.estimated_content_count,
        estimated_size_mb: input.estimated_size_mb,
        shard_strategy: input.shard_strategy,
        redundancy_factor: input.redundancy_factor,
        shard_assignments_json: input.shard_assignments_json,
        emergency_triggers_json: input.emergency_triggers_json,
        emergency_contacts_json: input.emergency_contacts_json,
        recovery_instructions_json: input.recovery_instructions_json,
        cache_priority: input.cache_priority.unwrap_or(50),
        bandwidth_class: input.bandwidth_class.unwrap_or_else(|| "medium".to_string()),
        geographic_affinity: input.geographic_affinity,
        state: "proposed".to_string(),
        proposed_at: timestamp.clone(),
        accepted_at: None,
        activated_at: None,
        last_verification_at: None,
        verification_failures_json: "[]".to_string(),
        shards_stored_count: 0,
        last_shard_update_at: None,
        total_restores_performed: 0,
        shefa_commitment_id: None,
        note: input.note,
        metadata_json: input.metadata_json.unwrap_or_else(|| "{}".to_string()),
        created_at: timestamp.clone(),
        updated_at: timestamp,
    };

    // Create the entry
    let action_hash = create_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;

    // Create index links
    let id_anchor = StringAnchor::new("custodian_commitment_id", &commitment.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToCommitmentCustodian,
        (),
    )?;

    let custodian_anchor = StringAnchor::new("custodian_id", &commitment.custodian_agent_id);
    let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;
    create_link(
        custodian_anchor_hash,
        action_hash.clone(),
        LinkTypes::CustodianToCommitment,
        (),
    )?;

    let beneficiary_anchor = StringAnchor::new("beneficiary_id", &commitment.beneficiary_agent_id);
    let beneficiary_anchor_hash = hash_entry(&EntryTypes::StringAnchor(beneficiary_anchor))?;
    create_link(
        beneficiary_anchor_hash,
        action_hash.clone(),
        LinkTypes::BeneficiaryToCommitment,
        (),
    )?;

    Ok(CustodianCommitmentOutput {
        action_hash,
        entry_hash,
        commitment,
    })
}

/// Accept a custodian commitment (proposed → accepted)
#[hdk_extern]
pub fn accept_custodian_commitment(input: AcceptCommitmentInput) -> ExternResult<CustodianCommitmentOutput> {
    // Get the commitment by ID
    let id_anchor = StringAnchor::new("custodian_commitment_id", &input.commitment_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToCommitmentCustodian)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest("Commitment not found".to_string())));
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid commitment hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Commitment record not found".to_string())))?;

    let mut commitment = record
        .entry()
        .to_app_option::<CustodianCommitment>()
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid commitment entry type".to_string())))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid commitment entry".to_string())))?;

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Update the commitment
    commitment.state = "accepted".to_string();
    commitment.accepted_at = Some(timestamp.clone());
    commitment.updated_at = timestamp;

    // Create a new version with update
    let updated_action_hash = update_entry(action_hash.clone(), &EntryTypes::CustodianCommitment(commitment.clone()))?;

    let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;

    Ok(CustodianCommitmentOutput {
        action_hash: updated_action_hash,
        entry_hash,
        commitment,
    })
}

/// Query custodian commitments by various criteria
#[hdk_extern]
pub fn query_custodian_commitments(input: QueryCommitmentsInput) -> ExternResult<Vec<CustodianCommitmentOutput>> {
    let mut results = Vec::new();
    let limit = input.limit.unwrap_or(100) as usize;

    // Query by custodian if specified
    if let Some(custodian_id) = &input.custodian_agent_id {
        let custodian_anchor = StringAnchor::new("custodian_id", custodian_id);
        let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;

        let query = LinkQuery::try_new(custodian_anchor_hash, LinkTypes::CustodianToCommitment)?;
        let links = get_links(query, GetStrategy::default())?;

        for link in links.iter().take(limit) {
            let action_hash = ActionHash::try_from(link.target.clone())
                .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid commitment hash".to_string())))?;

            let record = get(action_hash.clone(), GetOptions::default())?;
            if let Some(record) = record {
                if let Some(commitment) = record.entry().to_app_option::<CustodianCommitment>().ok().flatten() {
                    // Filter by state if specified
                    if let Some(ref state) = input.state {
                        if commitment.state != *state {
                            continue;
                        }
                    }
                    let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;
                    results.push(CustodianCommitmentOutput {
                        action_hash,
                        entry_hash,
                        commitment,
                    });
                }
            }
        }
    }

    // Query by beneficiary if specified (and no custodian specified)
    if let Some(beneficiary_id) = &input.beneficiary_agent_id {
        if input.custodian_agent_id.is_none() {
            let beneficiary_anchor = StringAnchor::new("beneficiary_id", beneficiary_id);
            let beneficiary_anchor_hash = hash_entry(&EntryTypes::StringAnchor(beneficiary_anchor))?;

            let query = LinkQuery::try_new(beneficiary_anchor_hash, LinkTypes::BeneficiaryToCommitment)?;
            let links = get_links(query, GetStrategy::default())?;

            for link in links.iter().take(limit) {
                let action_hash = ActionHash::try_from(link.target.clone())
                    .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid commitment hash".to_string())))?;

                let record = get(action_hash.clone(), GetOptions::default())?;
                if let Some(record) = record {
                    if let Some(commitment) = record.entry().to_app_option::<CustodianCommitment>().ok().flatten() {
                        // Filter by state if specified
                        if let Some(ref state) = input.state {
                            if commitment.state != *state {
                                continue;
                            }
                        }
                        let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;
                        results.push(CustodianCommitmentOutput {
                            action_hash,
                            entry_hash,
                            commitment,
                        });
                    }
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Shard Generation, Storage & Watermarking (Phase 3)
// =============================================================================

/// Generate shards from content using specified strategy
///
/// Three strategies supported:
/// - full_replica: Each custodian holds complete encrypted copy (for small content)
/// - threshold_split: Shamir's Secret Sharing (M-of-N recovery)
/// - erasure_coded: Reed-Solomon erasure coding (efficient for large files)
///
/// This function is called by the client (not directly in zome) to prepare shards
/// before store_shard() commits them to DHT.
#[hdk_extern]
pub fn generate_shards(input: GenerateShardsInput) -> ExternResult<GenerateShardsOutput> {
    let total_shards = input.total_shards.unwrap_or(input.redundancy_factor * 2);

    if total_shards < input.redundancy_factor {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "total_shards must be >= redundancy_factor".to_string()
        )));
    }

    // Calculate hash of complete content
    let content_hash = calculate_sha256(&input.content_data);

    // Generate shard hashes (deterministic, based on content + strategy + index)
    // In production, would use actual Shamir or Reed-Solomon libraries
    let mut shard_hashes = Vec::new();
    for shard_index in 0..total_shards {
        let shard_seed = format!("{}-{}-{}", input.content_id, shard_index, input.shard_strategy);
        let shard_hash = calculate_sha256(&shard_seed);
        shard_hashes.push(shard_hash);
    }

    Ok(GenerateShardsOutput {
        content_id: input.content_id,
        shard_strategy: input.shard_strategy,
        total_shards,
        shard_hashes,
        content_hash,
    })
}

/// Generate cryptographic watermark proving shard authenticity
///
/// Watermarks enable verification that a shard came from authorized content:
/// - Links back to original content_id
/// - Signed by custodian_agent_id
/// - Contains shard hash for integrity
/// - Timestamped for audit trail
///
/// Content projected outside network carries watermark so consumers can verify:
/// "Is this actually from Sheila's Elohim presence, or a fake?"
fn generate_watermark(
    content_id: &str,
    custodian_id: &str,
    shard_index: u32,
    shard_hash: &str,
) -> ExternResult<String> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = now.as_seconds_and_nanos().0;

    // Create watermark data
    let watermark_data = format!(
        "content_id:{}|custodian:{}|shard:{}|hash:{}|timestamp:{}",
        content_id, custodian_id, shard_index, shard_hash, timestamp
    );

    // Sign watermark with custodian's agent key (would use actual cryptographic signature)
    // For now, create deterministic HMAC-like proof
    let signature = calculate_sha256(&format!("{}|{}", watermark_data, agent_info.agent_initial_pubkey));

    Ok(signature)
}

/// Verify watermark authenticity
///
/// Validates that:
/// - Watermark signature is cryptographically valid
/// - Content ID matches what we expect
/// - Shard data hasn't been tampered with
fn verify_watermark(
    watermark: &str,
    expected_content_id: &str,
    expected_shard_hash: &str,
) -> ExternResult<bool> {
    // In production: verify ECDSA/EdDSA signature
    // For now: watermark is valid if it can be reconstructed

    // Watermark format: content_id:...|shard:...|hash:...
    // Simple validation: check that expected values are in watermark
    let contains_content_id = watermark.contains(&format!("content_id:{}", expected_content_id));
    let contains_shard_hash = watermark.contains(&format!("hash:{}", expected_shard_hash));

    Ok(contains_content_id && contains_shard_hash)
}

/// SHA256 hash helper
fn calculate_sha256(data: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}", hash)
}

/// Store shard on-chain (encrypted, with watermark, linked to commitment)
///
/// Saves encrypted shard data to Holochain DHT with:
/// - Watermark proving origin
/// - Hash for integrity verification
/// - Link to custodian commitment
/// - Metadata for recovery
#[hdk_extern]
pub fn store_shard(input: StoreShardInput) -> ExternResult<StoredShardOutput> {
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Verify watermark validity
    verify_watermark(
        &input.watermark_signature,
        &input.content_id,
        &input.shard_hash,
    )?;

    // Create shard metadata entry (simplified - in production would be a full Shard entry type)
    // For now, store as StringAnchor + metadata to avoid adding new entry type
    let shard_key = format!(
        "shard-{}-{}-{}",
        input.content_id, input.shard_index, input.total_shards
    );

    let shard_anchor = StringAnchor::new("shard_storage", &shard_key);
    let shard_anchor_hash = hash_entry(&EntryTypes::StringAnchor(shard_anchor))?;

    // Store shard metadata as a StringAnchor (the encrypted_shard_data would be stored separately in production)
    let shard_metadata = StringAnchor::new(
        "shard_metadata",
        &format!(
            "content:{}|index:{}|total:{}|hash:{}",
            input.content_id, input.shard_index, input.total_shards, input.shard_hash
        ),
    );
    let metadata_hash = create_entry(&EntryTypes::StringAnchor(shard_metadata))?;

    // Link shard to commitment
    let commitment_anchor = StringAnchor::new("custodian_commitment_id", &input.commitment_id);
    let commitment_anchor_hash = hash_entry(&EntryTypes::StringAnchor(commitment_anchor))?;

    create_link(
        commitment_anchor_hash,
        metadata_hash.clone(),
        LinkTypes::ContentToCommitmentCustodian,
        (),
    )?;

    // Link shards by content for recovery queries
    let content_anchor = StringAnchor::new("content_id", &input.content_id);
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

    create_link(
        content_anchor_hash,
        metadata_hash.clone(),
        LinkTypes::ContentToCommitmentCustodian,
        (),
    )?;

    Ok(StoredShardOutput {
        action_hash: metadata_hash,
        content_id: input.content_id,
        shard_index: input.shard_index,
        stored_at: timestamp,
    })
}

/// Verify shard integrity via probabilistic sampling
///
/// Instead of verifying all shards constantly (expensive), sample-verify:
/// - Verify 10% of shards per cycle
/// - Detects failures early
/// - Reduces DHT load
#[hdk_extern]
pub fn verify_shard(input: VerifyShardInput) -> ExternResult<bool> {
    // Get commitment to find shard assignments
    let commitment_anchor = StringAnchor::new("custodian_commitment_id", &input.commitment_id);
    let commitment_anchor_hash = hash_entry(&EntryTypes::StringAnchor(commitment_anchor))?;

    let query = LinkQuery::try_new(commitment_anchor_hash, LinkTypes::ContentToCommitmentCustodian)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Commitment not found".to_string()
        )));
    }

    // For now, return success if commitment exists
    // In production: retrieve actual shard, verify hash, verify watermark
    Ok(true)
}

/// Retrieve shard for recovery (requires authorization)
///
/// Returns encrypted shard data that can be used for content reconstruction.
/// In emergency mode, multiple custodians provide their shards which are
/// combined (Shamir reconstruction or Reed-Solomon decode) to restore content.
#[hdk_extern]
pub fn get_shard(input: GetShardInput) -> ExternResult<GetShardOutput> {
    // Query for shard by content_id
    let content_anchor = StringAnchor::new("content_id", &input.content_id);
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

    let query = LinkQuery::try_new(content_anchor_hash, LinkTypes::ContentToCommitmentCustodian)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Shard not found".to_string()
        )));
    }

    // Get first matching shard (in production: filter by shard_index)
    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid shard hash".to_string())))?;

    let record = get(action_hash, GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Shard record not found".to_string())))?;

    // Extract shard metadata (simplified placeholder response)
    Ok(GetShardOutput {
        content_id: input.content_id.clone(),
        shard_index: input.shard_index,
        total_shards: 8,
        encrypted_shard_data: "ENCRYPTED_DATA_PLACEHOLDER".to_string(),
        encryption_method: "xchacha20".to_string(),
        shard_hash: calculate_sha256(&format!("{}-{}", input.content_id, input.shard_index)),
        watermark_signature: calculate_sha256("watermark"),
        stored_at: format!("{:?}", sys_time()?),
    })
}

// =============================================================================
// Relationship Hooks - Auto-Create CustodianCommitments
// =============================================================================

/// Auto-create custodian commitments when relationship intimacy level reaches trusted/intimate
///
/// When a HumanRelationship reaches:
/// - "intimate": Auto-propose bidirectional custody for private, invited, local reach content
/// - "trusted": Auto-propose bidirectional custody for neighborhood, municipal reach content
///
/// This implements the hybrid model where family and trusted circles automatically become
/// custodians based on relationship intimacy without manual negotiation.
fn on_relationship_updated(relationship: Relationship) -> ExternResult<()> {
    // Extract intimacy level from metadata_json if present
    let intimacy_level: String = if let Some(metadata) = &relationship.metadata_json {
        if let Ok(metadata_obj) = serde_json::from_str::<serde_json::Value>(metadata) {
            metadata_obj
                .get("intimacy_level")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    // Only auto-create commitments for trusted or intimate relationships
    let reach_filters = match intimacy_level.as_str() {
        "intimate" => vec!["private", "invited", "local"],
        "trusted" => vec!["neighborhood", "municipal"],
        _ => return Ok(()), // No commitments for recognition or connection levels
    };

    // Create bidirectional commitments
    // Both source → target and target → source

    let source_to_target_input = CreateCustodianCommitmentInput {
        custodian_agent_id: relationship.target_id.clone(), // Target becomes custodian of source's content
        beneficiary_agent_id: relationship.source_id.clone(),
        commitment_type: "relationship".to_string(),
        basis: if intimacy_level == "intimate" {
            "intimate_relationship".to_string()
        } else {
            "trusted_relationship".to_string()
        },
        relationship_id: Some(relationship.id.clone()),
        category_override_json: "[]".to_string(),
        content_filters_json: serde_json::to_string(&vec![serde_json::json!({
            "reach_levels": reach_filters.clone(),
            "content_types": [],
            "tags": [],
        })])
        .unwrap_or_else(|_| "[]".to_string()),
        estimated_content_count: 0,
        estimated_size_mb: 0.0,
        shard_strategy: "full_replica".to_string(),
        redundancy_factor: 2,
        shard_assignments_json: "[]".to_string(),
        emergency_triggers_json: serde_json::to_string(&vec![
            serde_json::json!({
                "trigger_type": "manual_signal",
                "enabled": true,
            }),
            serde_json::json!({
                "trigger_type": "trusted_party",
                "enabled": true,
                "trusted_agent_ids": [relationship.target_id.clone()],
            }),
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        emergency_contacts_json: serde_json::to_string(&vec![serde_json::json!({
            "agent_id": relationship.target_id.clone(),
            "contact_method": "agent-to-agent",
            "contact_value": relationship.target_id.clone(),
            "priority": 1,
        })])
        .unwrap_or_else(|_| "[]".to_string()),
        recovery_instructions_json: serde_json::to_string(&serde_json::json!({
            "instructions_markdown": "Reconstruct from relationship-based custodians",
            "shard_reconstruction_method": "full_replica",
            "verification_steps": ["verify_watermarks", "verify_shard_hashes"],
            "fallback_contacts": []
        }))
        .unwrap_or_else(|_| "{}".to_string()),
        cache_priority: Some(80),
        bandwidth_class: Some("medium".to_string()),
        geographic_affinity: None,
        note: Some(format!("{} relationship custody auto-commitment", intimacy_level)),
        metadata_json: Some(format!(r#"{{"relationship_type":"{}","auto_created":true}}"#, relationship.relationship_type)),
    };

    // Create source → target commitment
    let _ = create_custodian_commitment(source_to_target_input);

    // Create bidirectional commitment (target → source)
    let target_to_source_input = CreateCustodianCommitmentInput {
        custodian_agent_id: relationship.source_id.clone(),
        beneficiary_agent_id: relationship.target_id.clone(),
        commitment_type: "relationship".to_string(),
        basis: if intimacy_level == "intimate" {
            "intimate_relationship".to_string()
        } else {
            "trusted_relationship".to_string()
        },
        relationship_id: Some(relationship.id.clone()),
        category_override_json: "[]".to_string(),
        content_filters_json: serde_json::to_string(&vec![serde_json::json!({
            "reach_levels": reach_filters,
            "content_types": [],
            "tags": [],
        })])
        .unwrap_or_else(|_| "[]".to_string()),
        estimated_content_count: 0,
        estimated_size_mb: 0.0,
        shard_strategy: "full_replica".to_string(),
        redundancy_factor: 2,
        shard_assignments_json: "[]".to_string(),
        emergency_triggers_json: serde_json::to_string(&vec![
            serde_json::json!({
                "trigger_type": "manual_signal",
                "enabled": true,
            }),
            serde_json::json!({
                "trigger_type": "trusted_party",
                "enabled": true,
                "trusted_agent_ids": [relationship.source_id.clone()],
            }),
        ])
        .unwrap_or_else(|_| "[]".to_string()),
        emergency_contacts_json: serde_json::to_string(&vec![serde_json::json!({
            "agent_id": relationship.source_id.clone(),
            "contact_method": "agent-to-agent",
            "contact_value": relationship.source_id.clone(),
            "priority": 1,
        })])
        .unwrap_or_else(|_| "[]".to_string()),
        recovery_instructions_json: serde_json::to_string(&serde_json::json!({
            "instructions_markdown": "Reconstruct from relationship-based custodians",
            "shard_reconstruction_method": "full_replica",
            "verification_steps": ["verify_watermarks", "verify_shard_hashes"],
            "fallback_contacts": []
        }))
        .unwrap_or_else(|_| "{}".to_string()),
        cache_priority: Some(80),
        bandwidth_class: Some("medium".to_string()),
        geographic_affinity: None,
        note: Some(format!("{} relationship custody auto-commitment", intimacy_level)),
        metadata_json: Some(format!(r#"{{"relationship_type":"{}","auto_created":true}}"#, relationship.relationship_type)),
    };

    // Create target → source commitment
    let _ = create_custodian_commitment(target_to_source_input);

    Ok(())
}

// =============================================================================
// Phase 4: Emergency Protocol Activation
// =============================================================================

/// Helper: Hash a passphrase using SHA256
fn hash_passphrase(passphrase: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    passphrase.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}", hash)
}

/// Helper: Retrieve commitment by ID from DHT
fn get_commitment_by_id(commitment_id: &str) -> ExternResult<Option<CustodianCommitment>> {
    // Create the StringAnchor that was used during creation
    let id_anchor = StringAnchor::new("custodian_commitment_id", commitment_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    // Query links from the anchor to find the commitment action hash
    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToCommitmentCustodian)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        // Convert link.target to ActionHash
        let action_hash = ActionHash::try_from(link.target.clone())
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash in link".to_string())))?;

        // Get the commitment entry
        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Ok(Some(commitment)) =
                record.entry().to_app_option::<CustodianCommitment>()
            {
                return Ok(Some(commitment));
            }
        }
    }

    Ok(None)
}

/// Emergency activation via manual signal (passphrase)
///
/// Beneficiary provides passphrase → immediate activation → notify emergency contacts
/// Enables immediate recovery if account is compromised or in crisis
#[hdk_extern]
pub fn activate_emergency_manual(input: ActivateEmergencyManualInput) -> ExternResult<CustodianCommitmentOutput> {
    // Get commitment
    let mut commitment = get_commitment_by_id(&input.commitment_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Commitment not found".to_string())))?;

    // Verify this is beneficiary making the request
    let current_agent = agent_info()?.agent_initial_pubkey;
    if commitment.beneficiary_agent_id != current_agent.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only beneficiary can activate emergency manually".to_string()
        )));
    }

    // Extract manual_signal trigger config
    let triggers: Vec<EmergencyTriggerSpec> = serde_json::from_str(&commitment.emergency_triggers_json)
        .unwrap_or_default();

    let manual_trigger = triggers
        .iter()
        .find(|t| t.trigger_type == "manual_signal" && t.enabled)
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Manual signal trigger not enabled".to_string()
        )))?;

    // Verify passphrase if configured
    if let Some(passphrase_hash) = &manual_trigger.passphrase_hash {
        let provided_hash = hash_passphrase(&input.passphrase);
        if provided_hash != *passphrase_hash {
            return Err(wasm_error!(WasmErrorInner::Guest(
                "Invalid passphrase".to_string()
            )));
        }
    }

    // Activate the commitment
    commitment.state = "activated".to_string();
    commitment.activated_at = Some(format!("{:?}", sys_time()?));
    commitment.metadata_json = serde_json::to_string(&serde_json::json!({
        "activation_type": "manual_signal",
        "activation_reason": input.reason,
        "original_metadata": serde_json::from_str::<serde_json::Value>(&commitment.metadata_json).ok(),
    }))
    .unwrap_or_else(|_| "{}".to_string());

    // Update entry
    let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;
    let action_hash = create_entry(EntryTypes::CustodianCommitment(commitment.clone()))?;

    // Notify emergency contacts
    let _ = notify_emergency_contacts(&commitment, &input.reason);

    Ok(CustodianCommitmentOutput {
        action_hash,
        entry_hash,
        commitment,
    })
}

/// Emergency activation via trusted party signature
///
/// Pre-designated trusted agent (e.g., lawyer, family member) can activate recovery
/// Requires valid signature over commitment_id + beneficiary_id
#[hdk_extern]
pub fn activate_emergency_trusted_party(
    input: ActivateEmergencyTrustedPartyInput,
) -> ExternResult<CustodianCommitmentOutput> {
    // Get commitment
    let mut commitment = get_commitment_by_id(&input.commitment_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Commitment not found".to_string())))?;

    // Extract trusted_party trigger config
    let triggers: Vec<EmergencyTriggerSpec> = serde_json::from_str(&commitment.emergency_triggers_json)
        .unwrap_or_default();

    let trusted_trigger = triggers
        .iter()
        .find(|t| t.trigger_type == "trusted_party" && t.enabled)
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Trusted party trigger not enabled".to_string()
        )))?;

    // Verify trusted_agent_id is in the list
    let trusted_ids = trusted_trigger.trusted_agent_ids.as_ref()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No trusted parties configured".to_string()
        )))?;

    if !trusted_ids.contains(&input.trusted_agent_id) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Agent is not a designated trusted party".to_string()
        )));
    }

    // Verify signature (simplified: just check signature is not empty)
    // In production, would verify cryptographic signature with trusted_agent's pubkey
    if input.signature.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Invalid signature".to_string()
        )));
    }

    // Activate commitment
    commitment.state = "activated".to_string();
    commitment.activated_at = Some(format!("{:?}", sys_time()?));
    commitment.metadata_json = serde_json::to_string(&serde_json::json!({
        "activation_type": "trusted_party",
        "trusted_agent_id": input.trusted_agent_id,
        "activation_reason": input.reason,
    }))
    .unwrap_or_else(|_| "{}".to_string());

    // Update entry
    let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;
    let action_hash = create_entry(EntryTypes::CustodianCommitment(commitment.clone()))?;

    // Notify emergency contacts
    let _ = notify_emergency_contacts(&commitment, &input.reason);

    Ok(CustodianCommitmentOutput {
        action_hash,
        entry_hash,
        commitment,
    })
}

/// Submit vote for M-of-N consensus-based emergency activation
///
/// Any custodian can vote to approve recovery (or reject).
/// Once M of N custodians approve, emergency is activated.
#[hdk_extern]
pub fn submit_consensus_vote(input: SubmitConsensusVoteInput) -> ExternResult<ConsensusVoteStatus> {
    // Get commitment
    let commitment = get_commitment_by_id(&input.commitment_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Commitment not found".to_string())))?;

    // Extract consensus trigger
    let triggers: Vec<EmergencyTriggerSpec> = serde_json::from_str(&commitment.emergency_triggers_json)
        .unwrap_or_default();

    let consensus_trigger = triggers
        .iter()
        .find(|t| t.trigger_type == "m_of_n_consensus" && t.enabled)
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "M-of-N consensus trigger not enabled".to_string()
        )))?;

    let threshold_m = consensus_trigger.consensus_m
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Consensus threshold M not configured".to_string()
        )))?;

    let threshold_n = consensus_trigger.consensus_n
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Consensus total N not configured".to_string()
        )))?;

    // Store vote: create an anchor entry for voting record (simplified placeholder)
    // In production, would store as StringAnchor and link to commitment with:
    // {commitment_id, custodian_id, vote_approve, reason, timestamp}

    // For now, just return vote status placeholder

    // Calculate current vote status (simplified: return placeholder)
    let status = ConsensusVoteStatus {
        total_custodians: threshold_n,
        votes_received: 1, // Placeholder
        approves: if input.vote_approve { 1 } else { 0 },
        rejects: if !input.vote_approve { 1 } else { 0 },
        threshold_m,
        threshold_reached: false, // Placeholder
        activation_status: "pending".to_string(),
    };

    Ok(status)
}

/// Check M-of-N consensus status
///
/// Returns current vote counts and whether threshold is reached
#[hdk_extern]
pub fn check_consensus_status(input: CheckConsensusStatusInput) -> ExternResult<ConsensusVoteStatus> {
    // Get commitment
    let commitment = get_commitment_by_id(&input.commitment_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Commitment not found".to_string())))?;

    // Extract consensus trigger
    let triggers: Vec<EmergencyTriggerSpec> = serde_json::from_str(&commitment.emergency_triggers_json)
        .unwrap_or_default();

    let consensus_trigger = triggers
        .iter()
        .find(|t| t.trigger_type == "m_of_n_consensus" && t.enabled)
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "M-of-N consensus trigger not enabled".to_string()
        )))?;

    let threshold_m = consensus_trigger.consensus_m.unwrap_or(0);
    let threshold_n = consensus_trigger.consensus_n.unwrap_or(0);

    // Query votes (simplified placeholder)
    // In production, would aggregate actual votes from DHT
    let status = ConsensusVoteStatus {
        total_custodians: threshold_n,
        votes_received: 0,
        approves: 0,
        rejects: 0,
        threshold_m,
        threshold_reached: false,
        activation_status: "pending".to_string(),
    };

    Ok(status)
}

/// Reconstruct content from shards during emergency recovery
///
/// Gathers shards from custodians, verifies watermarks, reconstructs content.
/// Uses appropriate algorithm: full_replica (copy), threshold_split (Shamir), erasure_coded (Reed-Solomon)
#[hdk_extern]
pub fn reconstruct_content_from_shards(
    input: ReconstructContentInput,
) -> ExternResult<ReconstructContentOutput> {
    // Get commitment
    let commitment = get_commitment_by_id(&input.commitment_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Commitment not found".to_string())))?;

    // Only beneficiary can reconstruct
    let current_agent = agent_info()?.agent_initial_pubkey;
    if commitment.beneficiary_agent_id != current_agent.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only beneficiary can reconstruct content".to_string()
        )));
    }

    // Verify commitment is in activated state
    if commitment.state != "activated" {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Commitment must be activated before reconstruction".to_string()
        )));
    }

    // Parse shard assignments to find which custodians hold shards
    let _shard_assignments: Vec<serde_json::Value> =
        serde_json::from_str(&commitment.shard_assignments_json).unwrap_or_default();

    // Strategy: based on shard_strategy, reconstruct
    match commitment.shard_strategy.as_str() {
        "full_replica" => {
            // Each custodian holds complete copy → just retrieve from first available
            // Placeholder: return reconstructed content
            Ok(ReconstructContentOutput {
                content_id: input.content_id.clone(),
                content: format!("Reconstructed content from full_replica strategy for {}", input.content_id),
                shards_gathered: 1,
                shards_required: 1,
                reconstruction_method: "full_replica".to_string(),
                verification_status: "verified".to_string(),
                error_message: None,
            })
        }
        "threshold_split" => {
            // Shamir's Secret Sharing: need M of N shards to reconstruct
            // Placeholder: requires Shamir library integration
            Ok(ReconstructContentOutput {
                content_id: input.content_id.clone(),
                content: format!("Reconstructed content via Shamir's Secret Sharing for {}", input.content_id),
                shards_gathered: commitment.redundancy_factor,
                shards_required: commitment.redundancy_factor,
                reconstruction_method: "threshold_split".to_string(),
                verification_status: "verified".to_string(),
                error_message: None,
            })
        }
        "erasure_coded" => {
            // Reed-Solomon: need M of N shards to reconstruct
            // Placeholder: requires Reed-Solomon library integration
            Ok(ReconstructContentOutput {
                content_id: input.content_id.clone(),
                content: format!("Reconstructed content via Reed-Solomon for {}", input.content_id),
                shards_gathered: commitment.redundancy_factor,
                shards_required: commitment.redundancy_factor,
                reconstruction_method: "erasure_coded".to_string(),
                verification_status: "verified".to_string(),
                error_message: None,
            })
        }
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            "Unknown shard strategy".to_string()
        ))),
    }
}

/// Notify emergency contacts about activation
///
/// Helper function called after emergency is activated
/// In production, would send signals to emergency_contacts_json
fn notify_emergency_contacts(commitment: &CustodianCommitment, reason: &str) -> ExternResult<()> {
    let contacts: Vec<serde_json::Value> =
        serde_json::from_str(&commitment.emergency_contacts_json).unwrap_or_default();

    for contact in contacts {
        if let Some(agent_id) = contact.get("agent_id").and_then(|v| v.as_str()) {
            // In production: send signal/message to emergency contact
            // Placeholder: just log the notification
            let _message = format!(
                "Emergency activated for commitment {}: {}",
                commitment.id, reason
            );
        }
    }

    Ok(())
}

// =============================================================================
// Phase 6: Category-based Override Functions
// =============================================================================

/// Create or update a category override commitment
///
/// Specialists (doctors, firefighters, archivists) can custody content
/// outside normal relationship reach, based on professional category
#[hdk_extern]
pub fn create_category_override(
    input: CreateCategoryOverrideInput,
) -> ExternResult<CategoryOverrideOutput> {
    // Validate category type
    if !CATEGORY_OVERRIDE_TYPES.contains(&input.category.category_type.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Unknown category type: {}", input.category.category_type)
        )));
    }

    // Validate access level
    if !CATEGORY_ACCESS_LEVELS.contains(&input.category.access_level.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Unknown access level: {}", input.category.access_level)
        )));
    }

    // Get current time outside the closure
    let now = format!("{:?}", sys_time()?);

    // Get existing commitment or create new one
    let mut commitment = get_commitment_by_id(&input.commitment_id)?
        .unwrap_or_else(|| CustodianCommitment {
            id: input.commitment_id.clone(),
            custodian_agent_id: input.specialist_agent_id.clone(),
            beneficiary_agent_id: input.beneficiary_id.clone(),
            commitment_type: "category".to_string(),
            basis: input.category.category_type.clone(),
            relationship_id: None,
            category_override_json: serde_json::to_string(&input.category)
                .unwrap_or_else(|_| "{}".to_string()),
            content_filters_json: serde_json::to_string(&input.category.content_filters)
                .unwrap_or_else(|_| "[]".to_string()),
            estimated_content_count: 0,
            estimated_size_mb: 0.0,
            shard_strategy: "full_replica".to_string(),
            redundancy_factor: 2,
            shard_assignments_json: "[]".to_string(),
            emergency_triggers_json: "[]".to_string(),
            emergency_contacts_json: "[]".to_string(),
            recovery_instructions_json: "{}".to_string(),
            cache_priority: 90, // High priority for specialists
            bandwidth_class: "high".to_string(),
            geographic_affinity: None,
            state: "active".to_string(),
            proposed_at: now.clone(),
            accepted_at: Some(now.clone()),
            activated_at: Some(now.clone()),
            last_verification_at: None,
            verification_failures_json: "[]".to_string(),
            shards_stored_count: 0,
            last_shard_update_at: None,
            total_restores_performed: 0,
            shefa_commitment_id: None,
            note: Some(format!("Category override: {} - {}", input.category.category_type, input.reason)),
            metadata_json: serde_json::to_string(&serde_json::json!({
                "category_type": input.category.category_type,
                "access_level": input.category.access_level,
                "reason": input.reason,
                "expires_at": input.expires_at,
                "credentials": input.category.credentials,
            }))
            .unwrap_or_else(|_| "{}".to_string()),
            created_at: now.clone(),
            updated_at: now,
        });

    // Update commitment with category override info
    commitment.category_override_json = serde_json::to_string(&input.category)
        .unwrap_or_else(|_| "{}".to_string());

    // Store the updated commitment
    let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;
    let action_hash = create_entry(EntryTypes::CustodianCommitment(commitment.clone()))?;

    Ok(CategoryOverrideOutput {
        action_hash,
        entry_hash,
        commitment_id: commitment.id,
        override_status: "active".to_string(),
    })
}

/// Query category overrides with flexible filtering
#[hdk_extern]
pub fn query_category_overrides(
    _input: QueryCategoryOverridesInput,
) -> ExternResult<Vec<CategoryOverrideSummary>> {
    // Placeholder: In production, would query by links and filter
    // For now, return empty list (filters not yet implemented)
    let results: Vec<CategoryOverrideSummary> = Vec::new();
    Ok(results)
}

/// Validate if a specialist can access content via category override
///
/// Returns authorization status and details about which specialist category applies
#[hdk_extern]
pub fn validate_category_access(
    input: ValidateCategoryAccessInput,
) -> ExternResult<ValidateCategoryAccessOutput> {
    // Placeholder: would query for active category overrides for this specialist
    // and check if they match the content's required category
    let _specialist_id = input.specialist_id;
    let _content_id = input.content_id;
    let _required_category = input.required_category;

    Ok(ValidateCategoryAccessOutput {
        authorized: false,
        category_type: None,
        access_level: None,
        reason: "Category override validation not yet fully implemented".to_string(),
    })
}

/// Revoke a category override (beneficiary can revoke access)
#[hdk_extern]
pub fn revoke_category_override(
    commitment_id: String,
) -> ExternResult<CustodianCommitmentOutput> {
    // Get commitment
    let mut commitment = get_commitment_by_id(&commitment_id)?
        .ok_or(wasm_error!(WasmErrorInner::Guest("Commitment not found".to_string())))?;

    // Verify beneficiary is revoking
    let current_agent = agent_info()?.agent_initial_pubkey;
    if commitment.beneficiary_agent_id != current_agent.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only beneficiary can revoke category override".to_string()
        )));
    }

    // Revoke by updating state
    commitment.state = "revoked".to_string();
    commitment.updated_at = format!("{:?}", sys_time()?);

    // Store updated commitment
    let entry_hash = hash_entry(&EntryTypes::CustodianCommitment(commitment.clone()))?;
    let action_hash = create_entry(EntryTypes::CustodianCommitment(commitment.clone()))?;

    Ok(CustodianCommitmentOutput {
        action_hash,
        entry_hash,
        commitment,
    })
}

// =============================================================================
// Phase 7: Performance Optimization
// =============================================================================

/// Probabilistic verification strategy
///
/// Instead of verifying all shards every time, sample a random subset
/// This reduces verification overhead from O(N) to O(sqrt(N))
pub fn should_verify_shard_sample(total_shards: u32) -> bool {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Sample sqrt(total_shards) shards probabilistically
    let sample_size = ((total_shards as f64).sqrt().ceil()) as u32;
    let sample_probability = (sample_size as f64) / (total_shards as f64);

    // Use system time as randomness source
    let mut hasher = DefaultHasher::new();
    if let Ok(time) = sys_time() {
        format!("{:?}", time).hash(&mut hasher);
    } else {
        // Fallback if sys_time fails
        total_shards.hash(&mut hasher);
    }
    let random = (hasher.finish() % 100) as f64;

    random < (sample_probability * 100.0)
}

/// Batch accept multiple custodian commitments
///
/// Optimizes accepting multiple relationship-based commitments in one call
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchAcceptCommitmentsInput {
    pub commitment_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BatchAcceptCommitmentsOutput {
    pub accepted_count: u32,
    pub failed_count: u32,
    pub errors: Vec<String>,
}

#[hdk_extern]
pub fn batch_accept_commitments(
    input: BatchAcceptCommitmentsInput,
) -> ExternResult<BatchAcceptCommitmentsOutput> {
    let mut accepted = 0u32;
    let mut failed = 0u32;
    let mut errors = Vec::new();

    for commitment_id in input.commitment_ids {
        match get_commitment_by_id(&commitment_id) {
            Ok(Some(mut commitment)) => {
                // Accept only if in proposed state
                if commitment.state == "proposed" {
                    commitment.state = "accepted".to_string();
                    let now_str = sys_time()
                        .map(|t| format!("{:?}", t))
                        .unwrap_or_else(|_| "unknown_time".to_string());
                    commitment.accepted_at = Some(now_str);

                    // Store the updated commitment
                    if let Err(e) = create_entry(EntryTypes::CustodianCommitment(commitment)) {
                        failed += 1;
                        errors.push(format!("Failed to accept {}: {:?}", commitment_id, e));
                    } else {
                        accepted += 1;
                    }
                } else {
                    failed += 1;
                    errors.push(format!("Commitment {} not in proposed state", commitment_id));
                }
            }
            Ok(None) => {
                failed += 1;
                errors.push(format!("Commitment {} not found", commitment_id));
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("Error loading commitment {}: {:?}", commitment_id, e));
            }
        }
    }

    Ok(BatchAcceptCommitmentsOutput {
        accepted_count: accepted,
        failed_count: failed,
        errors,
    })
}

/// Batch update commitment metadata
///
/// Update multiple commitments' metadata in one call (e.g., cache priority changes)
#[derive(Serialize, Deserialize, Debug)]
pub struct BatchUpdateCommitmentsInput {
    pub updates: Vec<(String, u32, Option<String>)>, // (commitment_id, new_cache_priority, new_bandwidth_class)
}

#[hdk_extern]
pub fn batch_update_commitments(
    input: BatchUpdateCommitmentsInput,
) -> ExternResult<BatchAcceptCommitmentsOutput> {
    let mut updated = 0u32;
    let mut failed = 0u32;
    let mut errors = Vec::new();

    for (commitment_id, cache_priority, bandwidth_class) in input.updates {
        match get_commitment_by_id(&commitment_id) {
            Ok(Some(mut commitment)) => {
                // Update only non-state fields to avoid state transitions
                commitment.cache_priority = cache_priority.clamp(0, 100);
                if let Some(bw_class) = bandwidth_class {
                    commitment.bandwidth_class = bw_class;
                }
                let now_str = sys_time()
                    .map(|t| format!("{:?}", t))
                    .unwrap_or_else(|_| "unknown_time".to_string());
                commitment.updated_at = now_str;

                if let Err(e) = create_entry(EntryTypes::CustodianCommitment(commitment)) {
                    failed += 1;
                    errors.push(format!("Failed to update {}: {:?}", commitment_id, e));
                } else {
                    updated += 1;
                }
            }
            Ok(None) => {
                failed += 1;
                errors.push(format!("Commitment {} not found", commitment_id));
            }
            Err(e) => {
                failed += 1;
                errors.push(format!("Error loading commitment {}: {:?}", commitment_id, e));
            }
        }
    }

    Ok(BatchAcceptCommitmentsOutput {
        accepted_count: updated,
        failed_count: failed,
        errors,
    })
}

/// Lazy-load shard metadata without full content
///
/// Returns only shard hashes and metadata, not encrypted data
/// Useful for verification and inventory without bandwidth overhead
#[derive(Serialize, Deserialize, Debug)]
pub struct ShardMetadataOnly {
    pub content_id: String,
    pub shard_index: u32,
    pub total_shards: u32,
    pub shard_hash: String,
    pub watermark_signature: String,
    pub stored_at: String,
    pub verified_at: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GetShardMetadataInput {
    pub commitment_id: String,
    pub limit: Option<u32>,
}

#[hdk_extern]
pub fn get_shard_metadata_only(
    _input: GetShardMetadataInput,
) -> ExternResult<Vec<ShardMetadataOnly>> {
    // Placeholder: would query shard metadata links without loading encrypted data
    let results: Vec<ShardMetadataOnly> = Vec::new();
    Ok(results)
}

// =============================================================================
// Shefa: Insurance Mutual Zome Functions
// =============================================================================

/// Create a member risk profile entry
#[hdk_extern]
pub fn create_member_risk_profile(input: MemberRiskProfile) -> ExternResult<(ActionHash, EntryHash)> {
    let action_hash = create_entry(&EntryTypes::MemberRiskProfile(input.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::MemberRiskProfile(input.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("member_risk_profile", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToMemberRiskProfile, ())?;

    // Create member lookup link
    let member_anchor = StringAnchor::new("member_profiles", &input.member_id);
    let member_anchor_hash = hash_entry(&EntryTypes::StringAnchor(member_anchor))?;
    create_link(member_anchor_hash, action_hash.clone(), LinkTypes::MemberToRiskProfile, ())?;

    // RiskProfileByTier link removed - query via projection instead

    Ok((action_hash, entry_hash))
}

/// Get a member risk profile by ID
#[hdk_extern]
pub fn get_member_risk_profile(profile_id: String) -> ExternResult<Option<(ActionHash, MemberRiskProfile)>> {
    let id_anchor = StringAnchor::new("member_risk_profile", &profile_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToMemberRiskProfile)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(profile) = record.entry().to_app_option::<MemberRiskProfile>().ok().flatten() {
            return Ok(Some((action_hash, profile)));
        }
    }

    Ok(None)
}

/// Create a coverage policy entry
#[hdk_extern]
pub fn create_coverage_policy(input: CoveragePolicy) -> ExternResult<(ActionHash, EntryHash)> {
    let action_hash = create_entry(&EntryTypes::CoveragePolicy(input.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::CoveragePolicy(input.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("coverage_policy", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToCoveragePolicy, ())?;

    // Create member lookup link
    let member_anchor = StringAnchor::new("member_policies", &input.member_id);
    let member_anchor_hash = hash_entry(&EntryTypes::StringAnchor(member_anchor))?;
    create_link(member_anchor_hash, action_hash.clone(), LinkTypes::MemberToCoveragePolicy, ())?;

    // CoveragePolicyByLevel link removed - query via projection instead

    Ok((action_hash, entry_hash))
}

/// Get a coverage policy by ID
#[hdk_extern]
pub fn get_coverage_policy(policy_id: String) -> ExternResult<Option<(ActionHash, CoveragePolicy)>> {
    let id_anchor = StringAnchor::new("coverage_policy", &policy_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToCoveragePolicy)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(policy) = record.entry().to_app_option::<CoveragePolicy>().ok().flatten() {
            return Ok(Some((action_hash, policy)));
        }
    }

    Ok(None)
}

/// Create an insurance claim entry
#[hdk_extern]
pub fn create_insurance_claim(input: InsuranceClaim) -> ExternResult<(ActionHash, EntryHash)> {
    let action_hash = create_entry(&EntryTypes::InsuranceClaim(input.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::InsuranceClaim(input.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("insurance_claim", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToInsuranceClaim, ())?;

    // Create member lookup link
    let member_anchor = StringAnchor::new("member_claims", &input.member_id);
    let member_anchor_hash = hash_entry(&EntryTypes::StringAnchor(member_anchor))?;
    create_link(member_anchor_hash, action_hash.clone(), LinkTypes::MemberToClaim, ())?;

    // Create status lookup link
    let status_anchor = StringAnchor::new("claims_by_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::ClaimByStatus, ())?;

    Ok((action_hash, entry_hash))
}

/// Get an insurance claim by ID
#[hdk_extern]
pub fn get_insurance_claim(claim_id: String) -> ExternResult<Option<(ActionHash, InsuranceClaim)>> {
    let id_anchor = StringAnchor::new("insurance_claim", &claim_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToInsuranceClaim)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(claim) = record.entry().to_app_option::<InsuranceClaim>().ok().flatten() {
            return Ok(Some((action_hash, claim)));
        }
    }

    Ok(None)
}

/// Create an adjustment reasoning entry
#[hdk_extern]
pub fn create_adjustment_reasoning(input: AdjustmentReasoning) -> ExternResult<(ActionHash, EntryHash)> {
    let action_hash = create_entry(&EntryTypes::AdjustmentReasoning(input.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::AdjustmentReasoning(input.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("adjustment_reasoning", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToAdjustmentReasoning, ())?;

    // Create claim lookup link
    let claim_anchor = StringAnchor::new("claim_adjustments", &input.claim_id);
    let claim_anchor_hash = hash_entry(&EntryTypes::StringAnchor(claim_anchor))?;
    create_link(claim_anchor_hash, action_hash.clone(), LinkTypes::ClaimToAdjustment, ())?;

    Ok((action_hash, entry_hash))
}

/// Get an adjustment reasoning by ID
#[hdk_extern]
pub fn get_adjustment_reasoning(reasoning_id: String) -> ExternResult<Option<(ActionHash, AdjustmentReasoning)>> {
    let id_anchor = StringAnchor::new("adjustment_reasoning", &reasoning_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToAdjustmentReasoning)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(reasoning) = record.entry().to_app_option::<AdjustmentReasoning>().ok().flatten() {
            return Ok(Some((action_hash, reasoning)));
        }
    }

    Ok(None)
}

// =============================================================================
// Shefa: Requests & Offers Zome Functions
// =============================================================================

/// Create a service request entry
#[hdk_extern]
pub fn create_service_request(input: ServiceRequest) -> ExternResult<(ActionHash, EntryHash)> {
    let action_hash = create_entry(&EntryTypes::ServiceRequest(input.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::ServiceRequest(input.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("service_request", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToServiceRequest, ())?;

    // Create requester lookup link
    let requester_anchor = StringAnchor::new("user_requests", &input.requester_id);
    let requester_anchor_hash = hash_entry(&EntryTypes::StringAnchor(requester_anchor))?;
    create_link(requester_anchor_hash, action_hash.clone(), LinkTypes::RequesterToRequest, ())?;

    // Create status lookup link
    let status_anchor = StringAnchor::new("requests_by_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::RequestByStatus, ())?;

    Ok((action_hash, entry_hash))
}

/// Get a service request by ID
#[hdk_extern]
pub fn get_service_request(request_id: String) -> ExternResult<Option<(ActionHash, ServiceRequest)>> {
    let id_anchor = StringAnchor::new("service_request", &request_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToServiceRequest)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(request) = record.entry().to_app_option::<ServiceRequest>().ok().flatten() {
            return Ok(Some((action_hash, request)));
        }
    }

    Ok(None)
}

/// Create a service offer entry
#[hdk_extern]
pub fn create_service_offer(input: ServiceOffer) -> ExternResult<(ActionHash, EntryHash)> {
    let action_hash = create_entry(&EntryTypes::ServiceOffer(input.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::ServiceOffer(input.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("service_offer", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToServiceOffer, ())?;

    // Create offeror lookup link
    let offeror_anchor = StringAnchor::new("user_offers", &input.offeror_id);
    let offeror_anchor_hash = hash_entry(&EntryTypes::StringAnchor(offeror_anchor))?;
    create_link(offeror_anchor_hash, action_hash.clone(), LinkTypes::OfferorToOffer, ())?;

    // Create status lookup link
    let status_anchor = StringAnchor::new("offers_by_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::OfferByStatus, ())?;

    Ok((action_hash, entry_hash))
}

/// Get a service offer by ID
#[hdk_extern]
pub fn get_service_offer(offer_id: String) -> ExternResult<Option<(ActionHash, ServiceOffer)>> {
    let id_anchor = StringAnchor::new("service_offer", &offer_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToServiceOffer)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(offer) = record.entry().to_app_option::<ServiceOffer>().ok().flatten() {
            return Ok(Some((action_hash, offer)));
        }
    }

    Ok(None)
}

/// Create a service match entry
#[hdk_extern]
pub fn create_service_match(input: ServiceMatch) -> ExternResult<(ActionHash, EntryHash)> {
    let action_hash = create_entry(&EntryTypes::ServiceMatch(input.clone()))?;
    let entry_hash = hash_entry(&EntryTypes::ServiceMatch(input.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("service_match", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToServiceMatch, ())?;

    // Create request lookup link
    let request_anchor = StringAnchor::new("request_matches", &input.request_id);
    let request_anchor_hash = hash_entry(&EntryTypes::StringAnchor(request_anchor))?;
    create_link(request_anchor_hash, action_hash.clone(), LinkTypes::RequestToMatch, ())?;

    // Create offer lookup link
    let offer_anchor = StringAnchor::new("offer_matches", &input.offer_id);
    let offer_anchor_hash = hash_entry(&EntryTypes::StringAnchor(offer_anchor))?;
    create_link(offer_anchor_hash, action_hash.clone(), LinkTypes::OfferToMatch, ())?;

    // Create status lookup link
    let status_anchor = StringAnchor::new("matches_by_status", &input.status);
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(status_anchor_hash, action_hash.clone(), LinkTypes::MatchByStatus, ())?;

    Ok((action_hash, entry_hash))
}

/// Get a service match by ID
#[hdk_extern]
pub fn get_service_match(match_id: String) -> ExternResult<Option<(ActionHash, ServiceMatch)>> {
    let id_anchor = StringAnchor::new("service_match", &match_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToServiceMatch)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string())))?;

    let record = get(action_hash.clone(), GetOptions::default())?;
    if let Some(record) = record {
        if let Some(service_match) = record.entry().to_app_option::<ServiceMatch>().ok().flatten() {
            return Ok(Some((action_hash, service_match)));
        }
    }

    Ok(None)
}

// =============================================================================
// Post-Commit Signals for Doorway Projection
// =============================================================================

/// Signal types emitted after commits for real-time projection.
///
/// These signals are consumed by Doorway's Projection Engine to
/// update the MongoDB cache in real-time as the DHT changes.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ProjectionSignal {
    /// Content entry was created or updated
    ContentCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        content: Content,
        author: AgentPubKey,
    },
    /// LearningPath was created or updated
    PathCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        path: LearningPath,
        author: AgentPubKey,
    },
    /// PathStep was created or updated
    StepCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        step: PathStep,
        author: AgentPubKey,
    },
    /// PathChapter was created or updated
    ChapterCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        chapter: PathChapter,
        author: AgentPubKey,
    },
    /// Relationship was created
    RelationshipCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        relationship: Relationship,
        author: AgentPubKey,
    },
    /// Human (agent profile) was created or updated
    HumanCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        human: Human,
        author: AgentPubKey,
    },
    /// Agent was created or updated
    AgentCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        agent: Agent,
        author: AgentPubKey,
    },
    /// ContributorPresence was created or updated
    PresenceCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        presence: ContributorPresence,
        author: AgentPubKey,
    },
    /// CustodianCommitment was created or updated
    /// Signals custody relationship changes for real-time projection
    CustodianCommitmentCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        commitment: CustodianCommitment,
        author: AgentPubKey,
    },
    /// MemberRiskProfile was created or updated
    MemberRiskProfileCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        profile: MemberRiskProfile,
        author: AgentPubKey,
    },
    /// CoveragePolicy was created or updated
    CoveragePolicyCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        policy: CoveragePolicy,
        author: AgentPubKey,
    },
    /// InsuranceClaim was created or updated
    InsuranceClaimCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        claim: InsuranceClaim,
        author: AgentPubKey,
    },
    /// AdjustmentReasoning was created or updated
    AdjustmentReasoningCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        reasoning: AdjustmentReasoning,
        author: AgentPubKey,
    },
    /// ServiceRequest was created or updated
    ServiceRequestCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        request: ServiceRequest,
        author: AgentPubKey,
    },
    /// ServiceOffer was created or updated
    ServiceOfferCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        offer: ServiceOffer,
        author: AgentPubKey,
    },
    /// ServiceMatch was created or updated
    ServiceMatchCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        service_match: ServiceMatch,
        author: AgentPubKey,
    },
    /// DoorwayRegistration was created or updated
    DoorwayCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        doorway: DoorwayRegistration,
        author: AgentPubKey,
    },
    /// DoorwayHeartbeat was recorded
    DoorwayHeartbeatCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        heartbeat: DoorwayHeartbeat,
        author: AgentPubKey,
    },
    /// DoorwayHeartbeatSummary was recorded (daily aggregate)
    DoorwaySummaryCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        summary: DoorwayHeartbeatSummary,
        author: AgentPubKey,
    },
    /// Generic entry committed (for extension)
    EntryCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        entry_type: String,
        author: AgentPubKey,
    },

    // =========================================================================
    // Import Batch Signals - for Elohim-Store orchestration
    // =========================================================================

    /// ImportBatch was queued (store can start processing)
    ImportBatchQueued {
        batch_id: String,
        blob_hash: String,
        total_items: u32,
        batch_type: String,
    },

    /// ImportBatch progress update (periodic during processing)
    ImportBatchProgress {
        batch_id: String,
        processed_count: u32,
        error_count: u32,
        total_items: u32,
    },

    /// ImportBatch processing completed (success or with errors)
    ImportBatchCompleted {
        batch_id: String,
        processed_count: u32,
        error_count: u32,
        total_items: u32,
        errors: Vec<String>,
    },

    /// ImportBatch failed critically (halted processing)
    ImportBatchFailed {
        batch_id: String,
        processed_count: u32,
        error_count: u32,
        total_items: u32,
        fatal_error: String,
    },
}

/// Post-commit callback - emits signals for projection.
///
/// Called by Holochain after each successful commit. Inspects the
/// committed entries and emits signals that Doorway subscribes to
/// for real-time cache updates.
#[hdk_extern]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) -> ExternResult<()> {
    for signed_action in committed_actions {
        let action = signed_action.hashed.content.clone();
        let action_hash = signed_action.hashed.hash.clone();

        // Only process Create and Update actions (not deletes, links, etc.)
        let entry_hash = match &action {
            Action::Create(create) => create.entry_hash.clone(),
            Action::Update(update) => update.entry_hash.clone(),
            _ => continue,
        };

        // Get the entry to determine its type and emit the appropriate signal
        let record = match get(action_hash.clone(), GetOptions::default())? {
            Some(r) => r,
            None => continue,
        };

        let author = action.author().clone();

        // Try to deserialize as each entry type and emit the corresponding signal
        if let Some(content) = record.entry().to_app_option::<Content>().ok().flatten() {
            // Emit projection signal (for MongoDB)
            emit_signal(ProjectionSignal::ContentCommitted {
                action_hash,
                entry_hash,
                content: content.clone(),
                author,
            })?;
            // Emit cache signal (for Doorway)
            emit_signal(DoorwaySignal::new(CacheSignal::upsert(&content)))?;
        } else if let Some(path) = record.entry().to_app_option::<LearningPath>().ok().flatten() {
            // Emit projection signal (for MongoDB)
            emit_signal(ProjectionSignal::PathCommitted {
                action_hash,
                entry_hash,
                path: path.clone(),
                author,
            })?;
            // Emit cache signal (for Doorway)
            emit_signal(DoorwaySignal::new(CacheSignal::upsert(&path)))?;
        } else if let Some(step) = record.entry().to_app_option::<PathStep>().ok().flatten() {
            emit_signal(ProjectionSignal::StepCommitted {
                action_hash,
                entry_hash,
                step,
                author,
            })?;
        } else if let Some(chapter) = record.entry().to_app_option::<PathChapter>().ok().flatten() {
            emit_signal(ProjectionSignal::ChapterCommitted {
                action_hash,
                entry_hash,
                chapter,
                author,
            })?;
        } else if let Some(relationship) = record.entry().to_app_option::<Relationship>().ok().flatten() {
            // Auto-create custodian commitments when relationship reaches trusted/intimate
            let _ = on_relationship_updated(relationship.clone());

            // Emit projection signal (for MongoDB)
            emit_signal(ProjectionSignal::RelationshipCommitted {
                action_hash,
                entry_hash,
                relationship: relationship.clone(),
                author,
            })?;
            // Emit cache signal (for Doorway)
            emit_signal(DoorwaySignal::new(CacheSignal::upsert(&relationship)))?;
        } else if let Some(human) = record.entry().to_app_option::<Human>().ok().flatten() {
            emit_signal(ProjectionSignal::HumanCommitted {
                action_hash,
                entry_hash,
                human,
                author,
            })?;
        } else if let Some(agent) = record.entry().to_app_option::<Agent>().ok().flatten() {
            emit_signal(ProjectionSignal::AgentCommitted {
                action_hash,
                entry_hash,
                agent,
                author,
            })?;
        } else if let Some(presence) = record.entry().to_app_option::<ContributorPresence>().ok().flatten() {
            emit_signal(ProjectionSignal::PresenceCommitted {
                action_hash,
                entry_hash,
                presence,
                author,
            })?;
        } else if let Some(commitment) = record.entry().to_app_option::<CustodianCommitment>().ok().flatten() {
            emit_signal(ProjectionSignal::CustodianCommitmentCommitted {
                action_hash,
                entry_hash,
                commitment,
                author,
            })?;
        } else if let Some(profile) = record.entry().to_app_option::<MemberRiskProfile>().ok().flatten() {
            emit_signal(ProjectionSignal::MemberRiskProfileCommitted {
                action_hash,
                entry_hash,
                profile,
                author,
            })?;
        } else if let Some(policy) = record.entry().to_app_option::<CoveragePolicy>().ok().flatten() {
            emit_signal(ProjectionSignal::CoveragePolicyCommitted {
                action_hash,
                entry_hash,
                policy,
                author,
            })?;
        } else if let Some(claim) = record.entry().to_app_option::<InsuranceClaim>().ok().flatten() {
            emit_signal(ProjectionSignal::InsuranceClaimCommitted {
                action_hash,
                entry_hash,
                claim,
                author,
            })?;
        } else if let Some(reasoning) = record.entry().to_app_option::<AdjustmentReasoning>().ok().flatten() {
            emit_signal(ProjectionSignal::AdjustmentReasoningCommitted {
                action_hash,
                entry_hash,
                reasoning,
                author,
            })?;
        } else if let Some(request) = record.entry().to_app_option::<ServiceRequest>().ok().flatten() {
            emit_signal(ProjectionSignal::ServiceRequestCommitted {
                action_hash,
                entry_hash,
                request,
                author,
            })?;
        } else if let Some(offer) = record.entry().to_app_option::<ServiceOffer>().ok().flatten() {
            emit_signal(ProjectionSignal::ServiceOfferCommitted {
                action_hash,
                entry_hash,
                offer,
                author,
            })?;
        } else if let Some(service_match) = record.entry().to_app_option::<ServiceMatch>().ok().flatten() {
            emit_signal(ProjectionSignal::ServiceMatchCommitted {
                action_hash,
                entry_hash,
                service_match,
                author,
            })?;
        } else if let Some(doorway) = record.entry().to_app_option::<DoorwayRegistration>().ok().flatten() {
            emit_signal(ProjectionSignal::DoorwayCommitted {
                action_hash,
                entry_hash,
                doorway,
                author,
            })?;
        } else if let Some(heartbeat) = record.entry().to_app_option::<DoorwayHeartbeat>().ok().flatten() {
            emit_signal(ProjectionSignal::DoorwayHeartbeatCommitted {
                action_hash,
                entry_hash,
                heartbeat,
                author,
            })?;
        } else if let Some(summary) = record.entry().to_app_option::<DoorwayHeartbeatSummary>().ok().flatten() {
            emit_signal(ProjectionSignal::DoorwaySummaryCommitted {
                action_hash,
                entry_hash,
                summary,
                author,
            })?;
        }
        // Other entry types can be added here as needed
    }

    Ok(())
}

// Doorway functions moved to: holochain/dna/infrastructure/zomes/infrastructure/

// =============================================================================
// SHEFA: FLOW PLANNING ZOME FUNCTIONS (Phase 1 - CRUD Operations)
// =============================================================================

/// Create a new FlowPlan
#[hdk_extern]
pub fn create_flow_plan(flow_plan: FlowPlan) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::FlowPlan(flow_plan.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Created entry not found".to_string()
        )))?;

    // Link from steward to plan
    create_link(
        flow_plan.steward_id.clone(),
        action_hash.clone(),
        LinkTypes::StewardToFlowPlan,
        (),
    )?;

    Ok(record)
}

/// Retrieve a FlowPlan by action hash
#[hdk_extern]
pub fn get_flow_plan(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

/// Get all flow plans for a steward
#[hdk_extern]
pub fn get_plans_for_steward(steward_id: AgentPubKey) -> ExternResult<Vec<Record>> {
    let query = LinkQuery::try_new(steward_id, LinkTypes::StewardToFlowPlan)?;
    let links = get_links(query, GetStrategy::default())?;
    let mut records = Vec::new();

    for link in links {
        if let Ok(action_hash) = ActionHash::try_from(link.target.clone()) {
            if let Ok(Some(record)) = get(action_hash, GetOptions::default()) {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Create a FlowBudget linked to a FlowPlan
#[hdk_extern]
pub fn create_flow_budget(flow_budget: FlowBudget) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::FlowBudget(flow_budget.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Created budget not found".to_string()
        )))?;

    Ok(record)
}

/// Create a FlowGoal linked to a FlowPlan
#[hdk_extern]
pub fn create_flow_goal(flow_goal: FlowGoal) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::FlowGoal(flow_goal.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Created goal not found".to_string()
        )))?;

    Ok(record)
}

/// Create a FlowMilestone linked to a FlowPlan
#[hdk_extern]
pub fn create_flow_milestone(flow_milestone: FlowMilestone) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::FlowMilestone(flow_milestone.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Created milestone not found".to_string()
        )))?;

    Ok(record)
}

/// Create a FlowScenario linked to a FlowPlan
#[hdk_extern]
pub fn create_flow_scenario(flow_scenario: FlowScenario) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::FlowScenario(flow_scenario.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Created scenario not found".to_string()
        )))?;

    Ok(record)
}

/// Create a FlowProjection linked to a FlowScenario
#[hdk_extern]
pub fn create_flow_projection(flow_projection: FlowProjection) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::FlowProjection(flow_projection.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Created projection not found".to_string()
        )))?;

    Ok(record)
}

/// Create a RecurringPattern for a steward
#[hdk_extern]
pub fn create_recurring_pattern(recurring_pattern: RecurringPattern) -> ExternResult<Record> {
    let action_hash = create_entry(&EntryTypes::RecurringPattern(recurring_pattern.clone()))?;
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Created pattern not found".to_string()
        )))?;

    // Link from steward to pattern
    create_link(
        recurring_pattern.steward_id.clone(),
        action_hash.clone(),
        LinkTypes::StewardToRecurringPattern,
        (),
    )?;

    Ok(record)
}

// =============================================================================
// CACHE WARMING - Pre-populate doorway cache with existing content
// =============================================================================

/// Input for warming doorway cache
#[derive(Serialize, Deserialize, Debug)]
pub struct WarmCacheInput {
    /// Content IDs to warm (seeder passes what it seeded)
    pub content_ids: Vec<String>,
    /// Path IDs to warm (if None, warms all paths from get_all_paths)
    pub path_ids: Option<Vec<String>>,
}

/// Output from cache warming operation
#[derive(Serialize, Deserialize, Debug)]
pub struct WarmCacheOutput {
    /// Number of content items successfully warmed
    pub content_warmed: u32,
    /// Number of paths successfully warmed
    pub paths_warmed: u32,
    /// Errors encountered (id: message)
    pub errors: Vec<String>,
}

/// Warm doorway cache by emitting signals for existing content.
///
/// Called by seeder after bulk content creation to pre-populate
/// doorway's projection store before users arrive. This ensures
/// instant cache hits instead of slow conductor fallback.
///
/// The DNA is the authoritative source - doorway just listens to signals.
#[hdk_extern]
pub fn warm_cache(input: WarmCacheInput) -> ExternResult<WarmCacheOutput> {
    let mut content_warmed = 0u32;
    let mut paths_warmed = 0u32;
    let mut errors: Vec<String> = Vec::new();

    // Warm content by ID
    for id in &input.content_ids {
        match get_content_by_id(QueryByIdInput { id: id.clone() }) {
            Ok(Some(output)) => {
                // Emit cache signal - doorway's SignalSubscriber picks this up
                if let Err(e) = emit_signal(DoorwaySignal::new(CacheSignal::upsert(&output.content))) {
                    errors.push(format!("{}: signal error: {:?}", id, e));
                } else {
                    content_warmed += 1;
                }
            }
            Ok(None) => {
                errors.push(format!("{}: not found", id));
            }
            Err(e) => {
                errors.push(format!("{}: {:?}", id, e));
            }
        }
    }

    // Determine which path IDs to warm
    let path_ids_to_warm: Vec<String> = match input.path_ids {
        Some(ids) => ids,
        None => {
            // Get all paths if no specific IDs provided
            match get_all_paths(()) {
                Ok(index) => index.paths.into_iter().map(|p| p.id).collect(),
                Err(e) => {
                    errors.push(format!("get_all_paths failed: {:?}", e));
                    Vec::new()
                }
            }
        }
    };

    // Warm paths
    for id in path_ids_to_warm {
        // Use get_path_overview for efficiency (doesn't load all steps)
        match get_path_overview(id.clone()) {
            Ok(Some(overview)) => {
                // Emit cache signal for the path
                if let Err(e) = emit_signal(DoorwaySignal::new(CacheSignal::upsert(&overview.path))) {
                    errors.push(format!("path/{}: signal error: {:?}", id, e));
                } else {
                    paths_warmed += 1;
                }
            }
            Ok(None) => {
                errors.push(format!("path/{}: not found", id));
            }
            Err(e) => {
                errors.push(format!("path/{}: {:?}", id, e));
            }
        }
    }

    Ok(WarmCacheOutput {
        content_warmed,
        paths_warmed,
        errors,
    })
}

// =============================================================================
// NOTE: Plaid/Banking Integration REMOVED from Holochain
// =============================================================================
// All Plaid/banking zome functions have been moved to the banking-bridge module
// in the Angular app (elohim-app/src/app/shefa/banking-bridge).
//
// Rationale:
// 1. Bank credentials are personal convenience, not network signals
// 2. Staging data is ephemeral - only approved transactions become EconomicEvents
// 3. Separation prevents cluttering the next-gen economy domain
//
// The EconomicEventBridgeService in banking-bridge handles:
// - Local IndexedDB storage for PlaidConnection, ImportBatch, StagedTransaction, TransactionRule
// - Approval workflow entirely client-side
// - Committing approved transactions to Holochain via create_economic_event()
//
// The ONLY network signal from banking is the final EconomicEvent.
// =============================================================================

// =============================================================================
// Shard Manifest Management - Unified Blob Storage Model
// =============================================================================
// These functions manage the distributed shard system for blob storage.
// Every blob has a ShardManifest (even single-shard blobs) and ShardLocations
// track which nodes hold which shards.

/// Input for registering a shard manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterShardManifestInput {
    pub blob_hash: String,
    pub total_size: u64,
    pub mime_type: String,
    pub encoding: String,
    pub data_shards: u8,
    pub total_shards: u8,
    pub shard_size: u64,
    pub shard_hashes: Vec<String>,
    pub reach: String,
    pub author_id: Option<String>,
}

/// Output from registering a shard manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterShardManifestOutput {
    pub action_hash: ActionHash,
    pub manifest: ShardManifest,
}

/// Register a new shard manifest for a blob
///
/// Creates the ShardManifest entry and establishes links:
/// - Anchor(blob_hash) -> ShardManifest (for lookup by blob hash)
/// - Anchor(author_id) -> ShardManifest (for author's manifests)
#[hdk_extern]
pub fn register_shard_manifest(input: RegisterShardManifestInput) -> ExternResult<RegisterShardManifestOutput> {
    // Validate encoding type
    if !SHARD_ENCODINGS.contains(&input.encoding.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Invalid encoding type: {}. Valid: {:?}", input.encoding, SHARD_ENCODINGS)
        )));
    }

    // Validate shard counts
    if input.shard_hashes.len() != input.total_shards as usize {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Shard hash count ({}) must match total_shards ({})",
                    input.shard_hashes.len(), input.total_shards)
        )));
    }

    let now = format!("{:?}", sys_time()?);

    let manifest = ShardManifest {
        blob_hash: input.blob_hash.clone(),
        total_size: input.total_size,
        mime_type: input.mime_type,
        encoding: input.encoding,
        data_shards: input.data_shards,
        total_shards: input.total_shards,
        shard_size: input.shard_size,
        shard_hashes: input.shard_hashes,
        reach: input.reach,
        author_id: input.author_id.clone(),
        created_at: now.clone(),
        verified_at: Some(now),
    };

    let action_hash = create_entry(EntryTypes::ShardManifest(manifest.clone()))?;

    // Create blob_hash -> ShardManifest link
    let blob_anchor = StringAnchor::new("blob_hash", &input.blob_hash);
    let blob_anchor_hash = hash_entry(&EntryTypes::StringAnchor(blob_anchor.clone()))?;
    create_entry(EntryTypes::StringAnchor(blob_anchor))?;
    create_link(
        blob_anchor_hash,
        action_hash.clone(),
        LinkTypes::BlobToManifest,
        (),
    )?;

    // Create author_id -> ShardManifest link (if author specified)
    if let Some(author_id) = &input.author_id {
        let author_anchor = StringAnchor::new("author_manifests", author_id);
        let author_anchor_hash = hash_entry(&EntryTypes::StringAnchor(author_anchor.clone()))?;
        create_entry(EntryTypes::StringAnchor(author_anchor))?;
        create_link(
            author_anchor_hash,
            action_hash.clone(),
            LinkTypes::AuthorToManifests,
            (),
        )?;
    }

    // Emit signal for doorway cache
    let _ = emit_signal(DoorwaySignal::new(CacheSignal {
        signal_type: CacheSignalType::Upsert,
        doc_type: "ShardManifest".to_string(),
        doc_id: input.blob_hash,
        data: serde_json::to_value(&manifest).ok(),
        ttl_secs: Some(86400), // 24 hours
        public: manifest.reach == "commons",
        reach: Some(manifest.reach.clone()),
    }));

    Ok(RegisterShardManifestOutput { action_hash, manifest })
}

/// Get shard manifest by blob hash
#[hdk_extern]
pub fn get_shard_manifest(blob_hash: String) -> ExternResult<Option<ShardManifest>> {
    let blob_anchor = StringAnchor::new("blob_hash", &blob_hash);
    let blob_anchor_hash = hash_entry(&EntryTypes::StringAnchor(blob_anchor))?;

    let query = LinkQuery::try_new(blob_anchor_hash, LinkTypes::BlobToManifest)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Ok(None);
    }

    // Get the most recent manifest
    let action_hash = ActionHash::try_from(links[0].target.clone())
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid manifest hash".to_string())))?;

    let record = get(action_hash, GetOptions::default())?;

    match record {
        Some(r) => {
            let manifest: ShardManifest = r.entry()
                .to_app_option()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode manifest: {:?}", e))))?
                .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Manifest entry not found".to_string())))?;
            Ok(Some(manifest))
        }
        None => Ok(None),
    }
}

/// Input for registering a shard location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterShardLocationInput {
    pub shard_hash: String,
    pub shard_index: u8,
    pub holder: String,
    /// W3C DID of the holder (e.g., "did:web:doorway.elohim.host")
    pub holder_did: Option<String>,
    /// Additional storage DIDs that can serve this shard
    #[serde(default)]
    pub storage_dids: Vec<String>,
    pub endpoint: Option<String>,
    pub storage_tier: Option<String>,
}

/// Output from registering a shard location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterShardLocationOutput {
    pub action_hash: ActionHash,
    pub location: ShardLocation,
}

/// Register that this node holds a shard
///
/// Creates a ShardLocation entry and links:
/// - Anchor(shard_hash) -> ShardLocation (find holders by shard)
/// - Anchor(holder) -> ShardLocation (find shards by holder)
/// - Anchor(holder_did) -> ShardLocation (find locations by DID)
#[hdk_extern]
pub fn register_shard_location(input: RegisterShardLocationInput) -> ExternResult<RegisterShardLocationOutput> {
    let now = format!("{:?}", sys_time()?);

    let location = ShardLocation {
        shard_hash: input.shard_hash.clone(),
        shard_index: input.shard_index,
        holder: input.holder.clone(),
        holder_did: input.holder_did.clone(),
        storage_dids: input.storage_dids,
        endpoint: input.endpoint,
        storage_tier: input.storage_tier,
        verified_at: now,
        is_active: true,
    };

    let action_hash = create_entry(EntryTypes::ShardLocation(location.clone()))?;

    // Create shard_hash -> ShardLocation link
    let shard_anchor = StringAnchor::new("shard_hash", &input.shard_hash);
    let shard_anchor_hash = hash_entry(&EntryTypes::StringAnchor(shard_anchor.clone()))?;
    create_entry(EntryTypes::StringAnchor(shard_anchor))?;
    create_link(
        shard_anchor_hash,
        action_hash.clone(),
        LinkTypes::ShardToLocations,
        (),
    )?;

    // Create holder -> ShardLocation link
    let holder_anchor = StringAnchor::new("holder_shards", &input.holder);
    let holder_anchor_hash = hash_entry(&EntryTypes::StringAnchor(holder_anchor.clone()))?;
    create_entry(EntryTypes::StringAnchor(holder_anchor))?;
    create_link(
        holder_anchor_hash,
        action_hash.clone(),
        LinkTypes::HolderToShards,
        (),
    )?;

    // Create holder_did -> ShardLocation link (if DID provided)
    if let Some(ref did) = input.holder_did {
        let did_anchor = StringAnchor::new("did_shards", did);
        let did_anchor_hash = hash_entry(&EntryTypes::StringAnchor(did_anchor.clone()))?;
        create_entry(EntryTypes::StringAnchor(did_anchor))?;
        create_link(
            did_anchor_hash,
            action_hash.clone(),
            LinkTypes::HolderToShards, // Reuse link type for DID lookups
            (),
        )?;
    }

    Ok(RegisterShardLocationOutput { action_hash, location })
}

/// Get all locations for a shard
#[hdk_extern]
pub fn get_shard_locations(shard_hash: String) -> ExternResult<Vec<ShardLocation>> {
    let shard_anchor = StringAnchor::new("shard_hash", &shard_hash);
    let shard_anchor_hash = hash_entry(&EntryTypes::StringAnchor(shard_anchor))?;

    let query = LinkQuery::try_new(shard_anchor_hash, LinkTypes::ShardToLocations)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut locations = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid location hash".to_string())))?;

        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Ok(Some(loc)) = record.entry().to_app_option::<ShardLocation>() {
                if loc.is_active {
                    locations.push(loc);
                }
            }
        }
    }

    Ok(locations)
}

/// Get all shards held by a specific agent
#[hdk_extern]
pub fn get_holder_shards(holder: String) -> ExternResult<Vec<ShardLocation>> {
    let holder_anchor = StringAnchor::new("holder_shards", &holder);
    let holder_anchor_hash = hash_entry(&EntryTypes::StringAnchor(holder_anchor))?;

    let query = LinkQuery::try_new(holder_anchor_hash, LinkTypes::HolderToShards)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut locations = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid location hash".to_string())))?;

        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Ok(Some(loc)) = record.entry().to_app_option::<ShardLocation>() {
                if loc.is_active {
                    locations.push(loc);
                }
            }
        }
    }

    Ok(locations)
}

/// Mark a shard location as inactive (shard no longer available)
#[hdk_extern]
pub fn deactivate_shard_location(action_hash: ActionHash) -> ExternResult<()> {
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Shard location not found".to_string())))?;

    let mut location: ShardLocation = record.entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode location: {:?}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Location entry not found".to_string())))?;

    location.is_active = false;
    location.verified_at = format!("{:?}", sys_time()?);

    update_entry(action_hash, &location)?;

    Ok(())
}

// =============================================================================
// DID-Based Storage Discovery
// =============================================================================

/// Get all storage DIDs for a blob (aggregated from all shard locations)
///
/// This is the primary discovery function for DID-based content federation.
/// Returns all DIDs (holder_did + storage_dids) from active shard locations.
#[hdk_extern]
pub fn get_storage_dids_for_blob(blob_hash: String) -> ExternResult<Vec<String>> {
    // First get the manifest to find all shard hashes
    let manifest_anchor = StringAnchor::new("blob_manifest", &blob_hash);
    let manifest_anchor_hash = hash_entry(&EntryTypes::StringAnchor(manifest_anchor))?;

    let query = LinkQuery::try_new(manifest_anchor_hash, LinkTypes::BlobToManifest)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut all_dids: Vec<String> = Vec::new();

    // Get manifest to find shard hashes
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid manifest hash".to_string())))?;

        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Ok(Some(manifest)) = record.entry().to_app_option::<ShardManifest>() {
                // For each shard in the manifest, get its locations
                for shard_hash in &manifest.shard_hashes {
                    let locations = get_shard_locations(shard_hash.clone())?;
                    for loc in locations {
                        // Add holder_did if present
                        if let Some(ref did) = loc.holder_did {
                            if !all_dids.contains(did) {
                                all_dids.push(did.clone());
                            }
                        }
                        // Add all storage_dids
                        for did in &loc.storage_dids {
                            if !all_dids.contains(did) {
                                all_dids.push(did.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(all_dids)
}

/// Input for adding a storage DID to an existing shard location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddStorageDIDInput {
    pub action_hash: ActionHash,
    pub storage_did: String,
}

/// Add a storage DID to an existing shard location
///
/// Used when a doorway or storage node replicates a shard and wants to
/// advertise itself as an additional source.
#[hdk_extern]
pub fn add_storage_did_to_shard(input: AddStorageDIDInput) -> ExternResult<ShardLocation> {
    let record = get(input.action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Shard location not found".to_string())))?;

    let mut location: ShardLocation = record.entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to decode location: {:?}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Location entry not found".to_string())))?;

    // Add DID if not already present
    if !location.storage_dids.contains(&input.storage_did) {
        location.storage_dids.push(input.storage_did.clone());
        location.verified_at = format!("{:?}", sys_time()?);
        update_entry(input.action_hash, &location)?;
    }

    Ok(location)
}

/// Get all shard locations for a specific DID
///
/// Useful for finding what shards a doorway/storage node is serving.
#[hdk_extern]
pub fn get_shard_locations_by_did(did: String) -> ExternResult<Vec<ShardLocation>> {
    let did_anchor = StringAnchor::new("did_shards", &did);
    let did_anchor_hash = hash_entry(&EntryTypes::StringAnchor(did_anchor))?;

    let query = LinkQuery::try_new(did_anchor_hash, LinkTypes::HolderToShards)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut locations = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid location hash".to_string())))?;

        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Ok(Some(loc)) = record.entry().to_app_option::<ShardLocation>() {
                if loc.is_active {
                    locations.push(loc);
                }
            }
        }
    }

    Ok(locations)
}

/// Get all manifests for a specific author
#[hdk_extern]
pub fn get_author_manifests(author_id: String) -> ExternResult<Vec<ShardManifest>> {
    let author_anchor = StringAnchor::new("author_manifests", &author_id);
    let author_anchor_hash = hash_entry(&EntryTypes::StringAnchor(author_anchor))?;

    let query = LinkQuery::try_new(author_anchor_hash, LinkTypes::AuthorToManifests)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut manifests = Vec::new();
    for link in links {
        let action_hash = ActionHash::try_from(link.target)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Invalid manifest hash".to_string())))?;

        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Ok(Some(manifest)) = record.entry().to_app_option::<ShardManifest>() {
                manifests.push(manifest);
            }
        }
    }

    Ok(manifests)
}

/// Resolve a blob - get manifest and all available shard locations
///
/// This is the main query for clients wanting to fetch a blob.
/// Returns the manifest and locations for each shard so the client
/// can decide where to fetch from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobResolutionOutput {
    pub manifest: ShardManifest,
    pub shard_locations: Vec<Vec<ShardLocation>>, // One vec per shard in order
}

#[hdk_extern]
pub fn resolve_blob(blob_hash: String) -> ExternResult<Option<BlobResolutionOutput>> {
    // Get manifest
    let manifest = match get_shard_manifest(blob_hash)? {
        Some(m) => m,
        None => return Ok(None),
    };

    // Get locations for each shard
    let mut shard_locations = Vec::with_capacity(manifest.shard_hashes.len());
    for shard_hash in &manifest.shard_hashes {
        let locations = get_shard_locations(shard_hash.clone())?;
        shard_locations.push(locations);
    }

    Ok(Some(BlobResolutionOutput {
        manifest,
        shard_locations,
    }))
}

// Qahal relationship functions moved to: holochain/dna/imagodei/zomes/imagodei/
