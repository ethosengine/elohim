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
//! - [`traits`] - Extension points for custom logic

pub mod bridge;
pub mod config;
pub mod report;
pub mod traits;

// Re-export commonly used items
pub use bridge::bridge_call;
pub use config::{MigrationInput, OrchestratorConfig};
pub use report::{
    CountCheck, MigrationCounts, MigrationError, MigrationPhase, MigrationReport,
    MigrationVerification,
};
pub use traits::{Exporter, Importer, TransformContext, Transformer, Verifier};
