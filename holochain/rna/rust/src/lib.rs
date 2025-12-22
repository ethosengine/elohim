//! # hc-rna - Holochain RNA Migration Toolkit
//!
//! RNA (Ribonucleic Acid) in biology reads DNA and coordinates protein synthesis.
//! In Holochain, RNA transcribes data between DNA versions during migrations.
//!
//! ## The Biological Metaphor
//!
//! | Biology | Holochain Analog |
//! |---------|------------------|
//! | **DNA** | Integrity zome - immutable validation rules |
//! | **RNA** | This module - transcribes data between DNA versions |
//! | **Codon** | Transform function - maps old field patterns to new |
//! | **Ribosome** | Import function - synthesizes new entries |
//! | **mRNA** | Export data - carries information from source DNA |
//! | **tRNA** | Bridge call - transfers data between cells |
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use hc_rna::{bridge_call, MigrationReport, Transformer};
//!
//! // 1. Export from source DNA via bridge call
//! let data: Vec<OldEntry> = bridge_call("my-dna-v1", "coordinator", "export_all", ())?;
//!
//! // 2. Transform to new schema
//! let transformed: Vec<NewEntry> = data.into_iter()
//!     .map(|e| MyTransformer.transform(e))
//!     .collect();
//!
//! // 3. Import into current DNA
//! let mut report = MigrationReport::new("v1", "v2");
//! for entry in transformed {
//!     match create_entry(&entry) {
//!         Ok(_) => report.record_success("MyEntry"),
//!         Err(e) => report.record_failure("MyEntry", None, e.to_string()),
//!     }
//! }
//! ```
//!
//! ## Module Organization
//!
//! - [`bridge`] - Cross-DNA communication helpers (tRNA)
//! - [`report`] - Migration tracking and verification
//! - [`config`] - Migration configuration options
//! - [`traits`] - Extension points for custom logic (external orchestration)
//! - [`healing`] - Core self-healing types: ValidationStatus, HealingSignal, HealingReport
//! - [`self_healing`] - SelfHealingEntry trait: implement for any entry type
//! - [`healing_orchestrator`] - Background healing orchestrator: manages healing workflow
//!
//! ## Two Patterns Supported
//!
//! ### 1. External Orchestration (Original RNA)
//!
//! Use [`Exporter`], [`Transformer`], [`Importer`] traits with external tooling.
//! Good for one-time migrations or controlled deployments.
//!
//! ### 2. Self-Healing DNA (New)
//!
//! Implement [`SelfHealingEntry`] for your entry types, use [`HealingOrchestrator`]
//! in init and read paths. The DNA heals itself continuously without external coordination.
//! Perfect for rapid schema iteration and operational resilience.

pub mod bridge;
pub mod config;
pub mod report;
pub mod traits;
pub mod healing;
pub mod self_healing;
pub mod healing_orchestrator;
pub mod entry_type_provider;
pub mod healing_strategy;
pub mod flexible_orchestrator;
pub mod analyzer;
pub mod generator;

// Re-export commonly used items
pub use bridge::bridge_call;
pub use config::{MigrationInput, OrchestratorConfig};
pub use report::{
    CountCheck, MigrationCounts, MigrationError, MigrationPhase, MigrationReport,
    MigrationVerification,
};
pub use traits::{Exporter, Importer, TransformContext, Transformer as MigrationTransformer, Verifier};

// Re-export self-healing types
pub use healing::{
    ValidationStatus, ValidationRule, HealingSignal, HealingMetadata, HealingResult,
    HealingReport, HealingCounts, AcceptAllValidator,
};
pub use self_healing::{SelfHealingEntry, HealedEntry, ValidationResult, BatchValidator};
pub use healing_orchestrator::{HealingOrchestrator, emit_healing_signal};

// Re-export flexible architecture types
pub use entry_type_provider::{
    HealableEntry, Validator, Transformer, ReferenceResolver,
    DegradationHandler, DegradationDecision, EntryTypeProvider, EntryTypeRegistry,
};
pub use healing_strategy::{
    HealingResult as HealingStrategyResult, HealingStrategy, HealingContext,
    BridgeFirstStrategy, SelfRepairFirstStrategy, LocalRepairOnlyStrategy, NoHealingStrategy,
};
pub use flexible_orchestrator::{OrchestratorConfig as FlexibleOrchestratorConfig, FlexibleOrchestrator, HealingOutcome};

// Re-export schema analysis and generation
pub use analyzer::{DNAAnalyzer, EntryTypeSchema, FieldType, Field};
pub use generator::ProviderGenerator;
