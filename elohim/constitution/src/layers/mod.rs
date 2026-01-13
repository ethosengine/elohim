//! Constitutional layer definitions.
//!
//! Each layer provides default principles and boundaries that can be
//! customized by specific communities/families/individuals.

pub mod bioregional;
pub mod community;
pub mod family;
pub mod global;
pub mod individual;
pub mod national;

pub use bioregional::BioregionalLayer;
pub use community::CommunityLayer;
pub use family::FamilyLayer;
pub use global::GlobalLayer;
pub use individual::IndividualLayer;
pub use national::NationalLayer;

use crate::types::{ConstitutionalContent, ConstitutionalLayer};

/// Trait for layer-specific constitutional content generation.
pub trait LayerProvider: Send + Sync {
    /// Get the layer this provider handles
    fn layer(&self) -> ConstitutionalLayer;

    /// Get default constitutional content for this layer
    fn default_content(&self) -> ConstitutionalContent;

    /// Get layer-specific system prompt fragment
    fn prompt_fragment(&self) -> String;
}
