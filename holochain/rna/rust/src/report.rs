//! Migration reporting and verification structures
//!
//! # RNA Metaphor: Transcription Reports
//!
//! Just as biologists track gene expression levels and transcription success,
//! these structures track what was successfully "transcribed" from one DNA to another.
//!
//! # Usage
//!
//! ```rust,ignore
//! use hc_rna::{MigrationReport, MigrationVerification};
//!
//! let mut report = MigrationReport::new("v1".to_string(), "v2".to_string());
//!
//! // Track successes and failures
//! report.record_success("Content");
//! report.record_failure("Path", Some("path-123".to_string()), "Invalid reference".to_string());
//!
//! // Complete the report
//! report.complete();
//!
//! println!("Migration success: {}", report.is_success());
//! ```

use hdk::prelude::*;
use std::collections::HashMap;

/// Migration report tracking success/failure of migrated items
///
/// This is the "transcription report" - documenting what was successfully
/// transcribed from the source DNA to the target DNA.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationReport {
    /// Schema version of the source DNA
    pub source_version: String,
    /// Schema version of the target DNA
    pub target_version: String,
    /// When migration started (ISO 8601 or Holochain timestamp)
    pub started_at: String,
    /// When migration completed (None if still running)
    pub completed_at: Option<String>,
    /// Per-entry-type migration counts
    pub entry_counts: HashMap<String, MigrationCounts>,
    /// List of errors encountered
    pub errors: Vec<MigrationError>,
    /// Post-migration verification results
    pub verification: MigrationVerification,
}

/// Counts for a single entry type
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MigrationCounts {
    /// Number of entries exported from source
    pub exported: u32,
    /// Number of entries transformed
    pub transformed: u32,
    /// Number of entries successfully imported
    pub imported: u32,
    /// Number of entries skipped (already exist)
    pub skipped: u32,
    /// Number of entries that failed to import
    pub failed: u32,
}

/// A single migration error
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MigrationError {
    /// The type of entry that failed
    pub entry_type: String,
    /// The ID of the specific entry (if available)
    pub entry_id: Option<String>,
    /// Which phase the error occurred in
    pub phase: MigrationPhase,
    /// Human-readable error message
    pub message: String,
}

/// Phase of migration where an error occurred
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum MigrationPhase {
    /// Error during export from source DNA
    Export,
    /// Error during schema transformation
    Transform,
    /// Error during import to target DNA
    Import,
    /// Error during verification
    Verify,
}

/// Verification results after migration
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct MigrationVerification {
    /// Overall verification passed
    pub passed: bool,
    /// Per-entry-type count checks
    pub count_checks: HashMap<String, CountCheck>,
    /// All references resolve correctly
    pub reference_integrity: bool,
    /// Additional notes or warnings
    pub notes: Vec<String>,
}

/// Result of checking expected vs actual counts
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CountCheck {
    /// Expected count (from source)
    pub expected: u32,
    /// Actual count (in target)
    pub actual: u32,
    /// Whether this check passed
    pub passed: bool,
}

impl MigrationReport {
    /// Create a new migration report
    ///
    /// # Arguments
    ///
    /// * `source_version` - Schema version of source DNA (e.g., "v1")
    /// * `target_version` - Schema version of target DNA (e.g., "v2")
    pub fn new(source_version: String, target_version: String) -> Self {
        let now = sys_time()
            .ok()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| chrono_now());

        Self {
            source_version,
            target_version,
            started_at: now,
            completed_at: None,
            entry_counts: HashMap::new(),
            errors: Vec::new(),
            verification: MigrationVerification::default(),
        }
    }

    /// Record a successful import
    pub fn record_success(&mut self, entry_type: &str) {
        let counts = self.entry_counts.entry(entry_type.to_string()).or_default();
        counts.imported += 1;
    }

    /// Record a skipped entry (already exists)
    pub fn record_skip(&mut self, entry_type: &str) {
        let counts = self.entry_counts.entry(entry_type.to_string()).or_default();
        counts.skipped += 1;
    }

    /// Record a failed import
    pub fn record_failure(&mut self, entry_type: &str, entry_id: Option<String>, message: String) {
        let counts = self.entry_counts.entry(entry_type.to_string()).or_default();
        counts.failed += 1;
        self.errors.push(MigrationError {
            entry_type: entry_type.to_string(),
            entry_id,
            phase: MigrationPhase::Import,
            message,
        });
    }

    /// Record an export count
    pub fn record_exported(&mut self, entry_type: &str, count: u32) {
        let counts = self.entry_counts.entry(entry_type.to_string()).or_default();
        counts.exported = count;
    }

    /// Record a transform count
    pub fn record_transformed(&mut self, entry_type: &str, count: u32) {
        let counts = self.entry_counts.entry(entry_type.to_string()).or_default();
        counts.transformed = count;
    }

    /// Add an error for any phase
    pub fn add_error(&mut self, entry_type: &str, phase: MigrationPhase, message: String) {
        self.errors.push(MigrationError {
            entry_type: entry_type.to_string(),
            entry_id: None,
            phase,
            message,
        });
    }

    /// Mark the migration as complete
    pub fn complete(&mut self) {
        let now = sys_time()
            .ok()
            .map(|t| format!("{:?}", t))
            .unwrap_or_else(|| chrono_now());
        self.completed_at = Some(now);
    }

    /// Check if migration was successful (no failures, verification passed)
    pub fn is_success(&self) -> bool {
        let no_failures = self
            .entry_counts
            .values()
            .all(|c| c.failed == 0);
        no_failures && self.verification.passed
    }

    /// Get total count of successfully imported entries
    pub fn total_imported(&self) -> u32 {
        self.entry_counts.values().map(|c| c.imported).sum()
    }

    /// Get total count of failed entries
    pub fn total_failed(&self) -> u32 {
        self.entry_counts.values().map(|c| c.failed).sum()
    }

    /// Get total count of skipped entries
    pub fn total_skipped(&self) -> u32 {
        self.entry_counts.values().map(|c| c.skipped).sum()
    }
}

impl MigrationVerification {
    /// Create a new verification with all checks passing
    pub fn passed() -> Self {
        Self {
            passed: true,
            count_checks: HashMap::new(),
            reference_integrity: true,
            notes: Vec::new(),
        }
    }

    /// Add a count check result
    pub fn add_count_check(&mut self, entry_type: &str, expected: u32, actual: u32) {
        let passed = actual >= expected;
        self.count_checks.insert(
            entry_type.to_string(),
            CountCheck {
                expected,
                actual,
                passed,
            },
        );
        // Update overall passed status
        self.passed = self.passed && passed;
    }

    /// Add a note
    pub fn add_note(&mut self, note: String) {
        self.notes.push(note);
    }

    /// Mark reference integrity check result
    pub fn set_reference_integrity(&mut self, passed: bool) {
        self.reference_integrity = passed;
        self.passed = self.passed && passed;
    }
}

/// Fallback timestamp when sys_time() not available (outside wasm)
fn chrono_now() -> String {
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_report_basic() {
        let mut report = MigrationReport::new("v1".to_string(), "v2".to_string());

        report.record_success("Content");
        report.record_success("Content");
        report.record_skip("Content");
        report.record_failure("Path", Some("path-1".to_string()), "Bad ref".to_string());

        assert_eq!(report.entry_counts.get("Content").unwrap().imported, 2);
        assert_eq!(report.entry_counts.get("Content").unwrap().skipped, 1);
        assert_eq!(report.entry_counts.get("Path").unwrap().failed, 1);
        assert_eq!(report.errors.len(), 1);
    }

    #[test]
    fn test_verification() {
        let mut verification = MigrationVerification::passed();

        verification.add_count_check("Content", 10, 10);
        assert!(verification.passed);

        verification.add_count_check("Path", 5, 3);
        assert!(!verification.passed);
    }
}
