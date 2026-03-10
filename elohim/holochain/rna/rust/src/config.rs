//! Migration configuration types
//!
//! # RNA Metaphor: Promoter Sequences
//!
//! In biology, promoter sequences are DNA regions that initiate transcription.
//! They determine when and how genes are expressed.
//!
//! These configuration types serve a similar purpose - they control when and
//! how migration "transcription" occurs.

use hdk::prelude::*;

/// Input for triggering a migration from within a zome
///
/// # Example
///
/// ```rust,ignore
/// #[hdk_extern]
/// pub fn migrate(input: MigrationInput) -> ExternResult<MigrationReport> {
///     if input.dry_run {
///         // Preview only
///     }
///     // ... migration logic
/// }
/// ```
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationInput {
    /// The role name of the source DNA version (e.g., "my-dna-v1")
    pub source_role_name: String,

    /// Entry types to migrate (empty = all types)
    #[serde(default)]
    pub entry_types: Vec<String>,

    /// Dry run mode - validate and report but don't create entries
    #[serde(default)]
    pub dry_run: bool,

    /// Continue on errors (true) or fail fast on first error (false)
    #[serde(default = "default_continue_on_error")]
    pub continue_on_error: bool,

    /// Maximum entries to migrate per type (for testing, None = unlimited)
    #[serde(default)]
    pub limit: Option<u32>,

    /// Skip verification after import
    #[serde(default)]
    pub skip_verification: bool,
}

fn default_continue_on_error() -> bool {
    true
}

impl Default for MigrationInput {
    fn default() -> Self {
        Self {
            source_role_name: "previous".to_string(),
            entry_types: Vec::new(),
            dry_run: false,
            continue_on_error: true,
            limit: None,
            skip_verification: false,
        }
    }
}

impl MigrationInput {
    /// Create input for migrating all entry types from a role
    pub fn all_from(role_name: &str) -> Self {
        Self {
            source_role_name: role_name.to_string(),
            ..Default::default()
        }
    }

    /// Create a dry-run input for previewing migration
    pub fn dry_run_from(role_name: &str) -> Self {
        Self {
            source_role_name: role_name.to_string(),
            dry_run: true,
            ..Default::default()
        }
    }

    /// Create input for migrating specific entry types
    pub fn types_from(role_name: &str, types: Vec<String>) -> Self {
        Self {
            source_role_name: role_name.to_string(),
            entry_types: types,
            ..Default::default()
        }
    }

    /// Check if a specific entry type should be migrated
    pub fn should_migrate(&self, entry_type: &str) -> bool {
        self.entry_types.is_empty() || self.entry_types.contains(&entry_type.to_string())
    }
}

/// Configuration for the TypeScript migration orchestrator
///
/// This mirrors the TypeScript `RNAConfig` interface for serialization
/// between Rust and TypeScript.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrchestratorConfig {
    /// Source DNA role name in happ.yaml
    pub source_role: String,
    /// Target DNA role name in happ.yaml
    pub target_role: String,
    /// Zome name in source DNA
    pub source_zome: String,
    /// Zome name in target DNA
    pub target_zome: String,
    /// Export function name (default: "export_for_migration")
    pub export_fn: String,
    /// Import function name (default: "import_migrated")
    pub import_fn: String,
    /// Verify function name (default: "verify_migration")
    pub verify_fn: String,
    /// Schema version function name (default: "export_schema_version")
    pub version_fn: String,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            source_role: "previous".to_string(),
            target_role: "current".to_string(),
            source_zome: "coordinator".to_string(),
            target_zome: "coordinator".to_string(),
            export_fn: "export_for_migration".to_string(),
            import_fn: "import_migrated".to_string(),
            verify_fn: "verify_migration".to_string(),
            version_fn: "export_schema_version".to_string(),
        }
    }
}

impl OrchestratorConfig {
    /// Create config for a simple same-zome migration
    pub fn simple(source_role: &str, target_role: &str, zome: &str) -> Self {
        Self {
            source_role: source_role.to_string(),
            target_role: target_role.to_string(),
            source_zome: zome.to_string(),
            target_zome: zome.to_string(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_input_should_migrate() {
        let input = MigrationInput::default();
        assert!(input.should_migrate("Content")); // Empty = all

        let input = MigrationInput::types_from("v1", vec!["Content".to_string()]);
        assert!(input.should_migrate("Content"));
        assert!(!input.should_migrate("Path"));
    }
}
