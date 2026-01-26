//! Constitutional Stack Management for Elohim Protocol
//!
//! This crate implements the 5-layer constitutional stack with immutability gradients:
//!
//! - **Global**: Universal principles, existential boundaries (most immutable)
//! - **Bioregional**: Ecological limits that human governance cannot override
//! - **National**: Cultural expressions, constitutional interpretations
//! - **Community**: Group norms, specific practices
//! - **Family**: Household values, private governance
//! - **Individual**: Personal sovereignty within bounds (most flexible)
//!
//! # Key Components
//!
//! - [`ConstitutionalStack`]: Assembled stack of constitutional documents for a context
//! - [`DhtVerifier`]: Trait for verifying document hashes against Holochain DHT
//! - [`PromptAssembler`]: Builds system prompts from constitutional layers
//! - [`ConflictResolver`]: Resolves conflicts between layers
//!
//! # Example
//!
//! ```ignore
//! use constitution::{ConstitutionalStack, StackContext, PromptAssembler};
//!
//! let context = StackContext {
//!     agent_id: "agent-123".to_string(),
//!     community_id: Some("community-456".to_string()),
//!     ..Default::default()
//! };
//!
//! let stack = ConstitutionalStack::build(context, &verifier, &resolver).await?;
//! let prompt = PromptAssembler::build_system_prompt(&stack);
//! ```

pub mod conflict;
pub mod layers;
pub mod precedent;
pub mod prompt;
pub mod stack;
pub mod types;
pub mod verification;

// Re-export main types
pub use conflict::ConflictResolver;
pub use layers::*;
pub use precedent::PrecedentStore;
pub use prompt::PromptAssembler;
pub use stack::{ConstitutionalStack, StackContext};
pub use types::*;
pub use verification::{DhtVerifier, HolochainDhtVerifier, OfflineDhtVerifier, VerificationError};
