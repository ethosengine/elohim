// =============================================================================
// Migration Module Template
// =============================================================================
// This module provides functions for migrating data from a previous DNA version.
// When creating a new DNA version (e.g., v2), copy this template and customize
// the transform functions for your schema changes.
//
// Usage:
// 1. Bundle both v1 and v2 DNAs in happ.yaml
// 2. Install the hApp with both DNAs
// 3. Call migrate_from_previous_version("lamad-v1") from v2
// 4. The migration zome will bridge-call v1's export functions
// 5. Transform and import data into v2
//
// Prerequisites:
// - Previous DNA must have export functions (export_all_content, etc.)
// - Both DNAs must be installed in the same conductor
// - Caller must have capability to call the previous DNA

use hdk::prelude::*;
use crate::{Content, LearningPath, PathStep, ContentMastery, AgentProgress, create_content, CreateContentInput};

/// Migration report tracking success/failure of migrated items
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationReport {
    pub source_version: String,
    pub target_version: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub content_migrated: u32,
    pub content_failed: u32,
    pub paths_migrated: u32,
    pub paths_failed: u32,
    pub mastery_migrated: u32,
    pub mastery_failed: u32,
    pub progress_migrated: u32,
    pub progress_failed: u32,
    pub errors: Vec<String>,
    pub verification: MigrationVerification,
}

/// Verification results after migration
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MigrationVerification {
    pub passed: bool,
    pub content_count_match: bool,
    pub path_count_match: bool,
    pub reference_integrity: bool,
    pub notes: Vec<String>,
}

impl MigrationReport {
    pub fn new(source_version: String, target_version: String) -> Self {
        let now = sys_time().ok().map(|t| format!("{:?}", t)).unwrap_or_else(|| "unknown".to_string());
        Self {
            source_version,
            target_version,
            started_at: now,
            completed_at: None,
            content_migrated: 0,
            content_failed: 0,
            paths_migrated: 0,
            paths_failed: 0,
            mastery_migrated: 0,
            mastery_failed: 0,
            progress_migrated: 0,
            progress_failed: 0,
            errors: Vec::new(),
            verification: MigrationVerification::default(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
    }

    pub fn complete(&mut self) {
        let now = sys_time().ok().map(|t| format!("{:?}", t)).unwrap_or_else(|| "unknown".to_string());
        self.completed_at = Some(now);
    }
}

/// Input for migration - specifies the source DNA role name
#[derive(Serialize, Deserialize, Debug)]
pub struct MigrationInput {
    /// The role name of the previous DNA version (e.g., "lamad-v1")
    pub source_role_name: String,
    /// Optional: only migrate specific data types
    pub migrate_content: bool,
    pub migrate_paths: bool,
    pub migrate_mastery: bool,
    pub migrate_progress: bool,
    /// Dry run mode - validate but don't create entries
    pub dry_run: bool,
}

impl Default for MigrationInput {
    fn default() -> Self {
        Self {
            source_role_name: "lamad-previous".to_string(),
            migrate_content: true,
            migrate_paths: true,
            migrate_mastery: true,
            migrate_progress: true,
            dry_run: false,
        }
    }
}

// =============================================================================
// Bridge Call Helpers
// =============================================================================

/// Call a function on another DNA role via bridge
///
/// # Arguments
/// * `role_name` - The role name from happ.yaml (e.g., "lamad-v1")
/// * `zome_name` - The zome to call (e.g., "content_store")
/// * `fn_name` - The function to call (e.g., "export_all_content")
/// * `payload` - The input payload (use () for no input)
#[allow(dead_code)]
fn bridge_call<I, O>(
    role_name: &str,
    zome_name: &str,
    fn_name: &str,
    payload: I,
) -> ExternResult<O>
where
    I: Serialize + std::fmt::Debug,
    O: serde::de::DeserializeOwned + std::fmt::Debug,
{
    // Use CallTargetCell::OtherRole for cross-DNA calls within the same hApp
    // The role_name maps to the DNA role in happ.yaml
    let response = call(
        CallTargetCell::OtherRole(role_name.to_string()),
        ZomeName::from(zome_name),
        FunctionName::from(fn_name),
        None, // CapSecret - None for unrestricted calls
        payload,
    )?;

    match response {
        ZomeCallResponse::Ok(result) => {
            let output: O = result.decode()
                .map_err(|e| wasm_error!(WasmErrorInner::Guest(
                    format!("Failed to decode bridge call response: {:?}", e)
                )))?;
            Ok(output)
        }
        ZomeCallResponse::Unauthorized(_, _, _, _) => {
            Err(wasm_error!(WasmErrorInner::Guest(
                "Unauthorized to call target DNA".to_string()
            )))
        }
        ZomeCallResponse::NetworkError(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(
                format!("Network error in bridge call: {}", err)
            )))
        }
        ZomeCallResponse::CountersigningSession(err) => {
            Err(wasm_error!(WasmErrorInner::Guest(
                format!("Countersigning error in bridge call: {}", err)
            )))
        }
        ZomeCallResponse::AuthenticationFailed(_, _) => {
            Err(wasm_error!(WasmErrorInner::Guest(
                "Authentication failed for bridge call".to_string()
            )))
        }
    }
}

// =============================================================================
// Transform Functions
// =============================================================================
// These functions handle schema evolution between versions.
// Customize these for your specific v1 -> v2 changes.

/// Transform content from v1 schema to current schema
///
/// Customize this function for your schema changes:
/// - Add new required fields with defaults
/// - Remove deprecated fields
/// - Rename fields
/// - Transform data formats
fn transform_content_v1_to_current(v1_content: Content) -> Content {
    // Example transformations (customize for your needs):

    // If v2 adds a new required field, set a default:
    // content.new_field = v1_content.metadata_json
    //     .and_then(|m| serde_json::from_str::<Value>(&m).ok())
    //     .and_then(|v| v.get("new_field").cloned())
    //     .unwrap_or_default();

    // For now, v1 and current are identical, so pass through:
    v1_content
}

/// Transform learning path from v1 schema to current schema
fn transform_path_v1_to_current(v1_path: LearningPath) -> LearningPath {
    // Customize for path schema changes
    v1_path
}

/// Transform path step from v1 schema to current schema
fn transform_step_v1_to_current(v1_step: PathStep) -> PathStep {
    // Customize for step schema changes
    v1_step
}

/// Transform mastery record from v1 schema to current schema
fn transform_mastery_v1_to_current(v1_mastery: ContentMastery) -> ContentMastery {
    // Customize for mastery schema changes
    // E.g., if mastery levels changed, remap them here
    v1_mastery
}

/// Transform progress record from v1 schema to current schema
fn transform_progress_v1_to_current(v1_progress: AgentProgress) -> AgentProgress {
    // Customize for progress schema changes
    v1_progress
}

// =============================================================================
// Migration Execution
// =============================================================================

/// Main migration function - call this from v2 to migrate from v1
///
/// This is a placeholder implementation. The actual bridge_call mechanism
/// requires the conductor to resolve cell IDs from role names, which happens
/// at runtime. See the TypeScript migrate.ts for the recommended approach
/// that uses the admin API to coordinate the migration.
#[hdk_extern]
pub fn migrate_from_previous_version(input: MigrationInput) -> ExternResult<MigrationReport> {
    let current_version = crate::SCHEMA_VERSION.to_string();
    let mut report = MigrationReport::new(input.source_role_name.clone(), current_version);

    // Note: Direct bridge calls from zomes require cell_id resolution
    // which is complex. The recommended approach is to:
    // 1. Use TypeScript/migrate.ts to orchestrate
    // 2. Call export functions on v1 via app websocket
    // 3. Call import functions on v2 via app websocket
    // 4. The zome provides the transform logic

    report.add_error(
        "Direct zome-to-zome bridge migration not yet implemented. \
         Use migrate.ts CLI tool instead.".to_string()
    );

    report.complete();
    Ok(report)
}

/// Import pre-exported content into current DNA
/// Called by migrate.ts after exporting from v1
#[hdk_extern]
pub fn import_migrated_content(content_list: Vec<Content>) -> ExternResult<MigrationReport> {
    let mut report = MigrationReport::new("external".to_string(), crate::SCHEMA_VERSION.to_string());

    for content in content_list {
        let transformed = transform_content_v1_to_current(content.clone());

        let input = CreateContentInput {
            id: transformed.id.clone(),
            content_type: transformed.content_type,
            title: transformed.title,
            description: transformed.description,
            content: transformed.content,
            content_format: transformed.content_format,
            tags: transformed.tags,
            source_path: transformed.source_path,
            related_node_ids: transformed.related_node_ids,
            reach: transformed.reach,
            metadata_json: transformed.metadata_json,
        };

        match create_content(input) {
            Ok(_) => report.content_migrated += 1,
            Err(e) => {
                report.content_failed += 1;
                report.add_error(format!("Failed to import content '{}': {:?}", transformed.id, e));
            }
        }
    }

    report.complete();
    Ok(report)
}

/// Verify migration completeness
/// Compare counts and check reference integrity
#[hdk_extern]
pub fn verify_migration(expected_counts: MigrationCounts) -> ExternResult<MigrationVerification> {
    let mut verification = MigrationVerification::default();

    // Get current counts
    let content_stats = crate::get_content_stats(())?;
    let path_index = crate::get_all_paths(())?;

    // Compare counts
    verification.content_count_match = content_stats.total_count >= expected_counts.content_count;
    verification.path_count_match = path_index.total_count >= expected_counts.path_count;

    if !verification.content_count_match {
        verification.notes.push(format!(
            "Content count mismatch: expected {}, got {}",
            expected_counts.content_count,
            content_stats.total_count
        ));
    }

    if !verification.path_count_match {
        verification.notes.push(format!(
            "Path count mismatch: expected {}, got {}",
            expected_counts.path_count,
            path_index.total_count
        ));
    }

    // TODO: Check reference integrity (all path steps reference valid content)
    verification.reference_integrity = true; // Placeholder

    verification.passed = verification.content_count_match
        && verification.path_count_match
        && verification.reference_integrity;

    Ok(verification)
}

/// Expected counts for verification
#[derive(Serialize, Deserialize, Debug)]
pub struct MigrationCounts {
    pub content_count: u32,
    pub path_count: u32,
    pub mastery_count: u32,
    pub progress_count: u32,
}
