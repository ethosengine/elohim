//! Traits defining the migration contract
//!
//! # RNA Metaphor
//!
//! These traits map to biological processes in gene expression:
//!
//! - **Exporter** = "Gene Expression" - reading genetic information from DNA
//! - **Transformer** = "Codon" - mapping one pattern to another
//! - **Importer** = "Ribosome" - synthesizing proteins (entries) from mRNA
//! - **Verifier** = "Quality Control" - checking protein folding
//!
//! # Usage
//!
//! ```rust,ignore
//! use hc_rna::{Transformer, Importer};
//!
//! // Implement Transformer for schema changes
//! struct ContentTransformer;
//! impl Transformer<ContentV1, ContentV2> for ContentTransformer {
//!     fn transform(&self, old: ContentV1) -> ContentV2 {
//!         ContentV2 {
//!             id: old.id,
//!             title: old.title,
//!             // New field with default
//!             created_at: old.metadata.get("created_at")
//!                 .cloned()
//!                 .unwrap_or_default(),
//!             ..Default::default()
//!         }
//!     }
//! }
//! ```

use hdk::prelude::*;
use std::collections::HashMap;

use crate::report::MigrationReport;

/// Trait for exporting entries from source DNA
///
/// # RNA Metaphor: Gene Expression
///
/// Just as gene expression reads genetic information from DNA,
/// the Exporter reads entry data from the source DNA for migration.
///
/// # Implementation
///
/// Implementors should:
/// 1. Query their source chain for all relevant entries
/// 2. Package them in a serializable format
/// 3. Include schema version information
///
/// # Example
///
/// ```rust,ignore
/// struct ContentExporter;
///
/// impl Exporter for ContentExporter {
///     type ExportData = Vec<Content>;
///
///     fn export_all(&self) -> ExternResult<Self::ExportData> {
///         let filter = ChainQueryFilter::new()
///             .entry_type(EntryTypes::Content.try_into()?);
///         let records = query(filter)?;
///         // ... extract and return content
///     }
///
///     fn schema_version(&self) -> &str {
///         "v1"
///     }
/// }
/// ```
pub trait Exporter {
    /// The type that will be exported (usually a Vec of entry types)
    type ExportData: Serialize + serde::de::DeserializeOwned;

    /// Export all data that needs migration
    fn export_all(&self) -> ExternResult<Self::ExportData>;

    /// Get the schema version of this exporter
    fn schema_version(&self) -> &str;

    /// Optional: Export with a limit (for testing)
    fn export_limited(&self, _limit: u32) -> ExternResult<Self::ExportData> {
        self.export_all()
    }
}

/// Trait for transforming entries between schema versions
///
/// # RNA Metaphor: Codon
///
/// In biology, a codon is a three-nucleotide sequence that maps to
/// a specific amino acid. Transform functions similarly map old
/// field patterns to new field patterns.
///
/// # Implementation
///
/// Implementors should handle:
/// - **Field additions**: Set defaults for new required fields
/// - **Field removals**: Simply don't copy removed fields
/// - **Field renames**: Map old name to new name
/// - **Type changes**: Convert data types as needed
///
/// # Example
///
/// ```rust,ignore
/// struct MyTransformer;
///
/// impl Transformer<OldEntry, NewEntry> for MyTransformer {
///     fn transform(&self, old: OldEntry) -> NewEntry {
///         NewEntry {
///             // Direct mapping
///             id: old.id,
///             // Renamed field
///             display_name: old.name,
///             // New field with default
///             version: 1,
///             // Field from metadata
///             tags: serde_json::from_str(&old.metadata_json)
///                 .ok()
///                 .and_then(|v: serde_json::Value| v.get("tags").cloned())
///                 .map(|v| serde_json::from_value(v).unwrap_or_default())
///                 .unwrap_or_default(),
///         }
///     }
/// }
/// ```
pub trait Transformer<Source, Target> {
    /// Transform a single entry from source schema to target schema
    fn transform(&self, source: Source) -> Target;

    /// Transform with additional context (for ID remapping, etc.)
    ///
    /// Override this for complex transformations that need:
    /// - Mapping old IDs to new IDs
    /// - Looking up related entries
    /// - Conditional logic based on migration state
    fn transform_with_context(&self, source: Source, _context: &TransformContext) -> Target {
        self.transform(source)
    }

    /// Transform a batch of entries
    fn transform_batch(&self, sources: Vec<Source>) -> Vec<Target> {
        sources.into_iter().map(|s| self.transform(s)).collect()
    }

    /// Transform a batch with context
    fn transform_batch_with_context(
        &self,
        sources: Vec<Source>,
        context: &TransformContext,
    ) -> Vec<Target> {
        sources
            .into_iter()
            .map(|s| self.transform_with_context(s, context))
            .collect()
    }
}

/// Context available during transformation
///
/// Use this for complex transformations that need to:
/// - Remap IDs from old to new
/// - Access configuration
/// - Track transformation state
#[derive(Default, Debug, Clone)]
pub struct TransformContext {
    /// Map of old IDs to new IDs (for reference updates)
    pub id_map: HashMap<String, String>,
    /// Additional metadata for transformation logic
    pub metadata: HashMap<String, String>,
}

impl TransformContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an ID mapping
    pub fn map_id(&mut self, old_id: String, new_id: String) {
        self.id_map.insert(old_id, new_id);
    }

    /// Get the new ID for an old ID (or return old ID if no mapping)
    pub fn resolve_id(&self, old_id: &str) -> String {
        self.id_map
            .get(old_id)
            .cloned()
            .unwrap_or_else(|| old_id.to_string())
    }

    /// Set metadata
    pub fn set_metadata(&mut self, key: &str, value: String) {
        self.metadata.insert(key.to_string(), value);
    }

    /// Get metadata
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Trait for importing entries into target DNA
///
/// # RNA Metaphor: Ribosome
///
/// The ribosome is the molecular machine that reads mRNA and synthesizes
/// proteins. Similarly, Importers read transformed data and create new entries.
///
/// # Implementation
///
/// Implementors should:
/// 1. Check if entry already exists (idempotency)
/// 2. Create the entry
/// 3. Create necessary links/indices
///
/// # Example
///
/// ```rust,ignore
/// struct ContentImporter;
///
/// impl Importer<Content> for ContentImporter {
///     fn import_one(&self, entry: Content) -> ExternResult<bool> {
///         // Check if exists
///         if get_content_by_id(&entry.id)?.is_some() {
///             return Ok(false); // Skip, already exists
///         }
///
///         // Create entry
///         create_entry(&entry)?;
///
///         // Create index links
///         create_link(anchor_hash, entry_hash, LinkTypes::IdToContent, ())?;
///
///         Ok(true)
///     }
///
///     fn entry_type_name(&self) -> &str {
///         "Content"
///     }
/// }
/// ```
pub trait Importer<T> {
    /// Import a single entry, handling duplicates gracefully
    ///
    /// # Returns
    /// - `Ok(true)` if entry was created
    /// - `Ok(false)` if entry was skipped (already exists)
    /// - `Err` if import failed
    fn import_one(&self, entry: T) -> ExternResult<bool>;

    /// The name of this entry type for reporting
    fn entry_type_name(&self) -> &str;

    /// Import multiple entries with reporting
    ///
    /// Default implementation calls `import_one` for each entry and
    /// records results in the report.
    fn import_batch(&self, entries: Vec<T>, report: &mut MigrationReport) {
        for entry in entries {
            match self.import_one(entry) {
                Ok(true) => report.record_success(self.entry_type_name()),
                Ok(false) => report.record_skip(self.entry_type_name()),
                Err(e) => report.record_failure(
                    self.entry_type_name(),
                    None,
                    format!("{:?}", e),
                ),
            }
        }
    }
}

/// Trait for verifying migration completeness
///
/// # RNA Metaphor: Quality Control
///
/// Cells have quality control mechanisms to ensure proteins are
/// correctly folded. Verifiers check that migration was complete.
pub trait Verifier {
    /// Type representing counts for verification
    type Counts;

    /// Get current counts from target DNA
    fn get_current_counts(&self) -> ExternResult<Self::Counts>;

    /// Verify counts match expected values
    fn verify_counts(&self, expected: &Self::Counts, actual: &Self::Counts) -> bool;

    /// Check reference integrity (all links point to valid entries)
    fn verify_references(&self) -> ExternResult<bool>;
}

/// Identity transformer - passes entries through unchanged
///
/// Use this when source and target schemas are identical.
pub struct IdentityTransformer;

impl<T: Clone> Transformer<T, T> for IdentityTransformer {
    fn transform(&self, source: T) -> T {
        source
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transform_context() {
        let mut ctx = TransformContext::new();
        ctx.map_id("old-1".to_string(), "new-1".to_string());

        assert_eq!(ctx.resolve_id("old-1"), "new-1");
        assert_eq!(ctx.resolve_id("unknown"), "unknown");
    }

    #[test]
    fn test_identity_transformer() {
        let transformer = IdentityTransformer;
        let input = "test".to_string();
        let output = transformer.transform(input.clone());
        assert_eq!(input, output);
    }
}
