//! `eprfs-agent`: renders elohim-agent capability EPRs into eprfs projection
//! manifests for runtime-specific surfaces (.claude, .codex).
//!
//! This crate holds ALL capability/dialect knowledge; `eprfs-core` stays
//! domain-neutral. A capability is authored once (markdown + normalized
//! frontmatter); each runtime is a projection, never a source.

pub mod binding;
pub mod canonical;
pub mod error;
pub mod project;

pub use binding::{normalize, FrontmatterDialect, ProjectionBinding};
pub use canonical::CanonicalAgent;
pub use error::{AgentProjectionError, Result};
pub use project::project;
