//! Layer context management.
//!
//! Determines which constitutional layer applies to a given context.

use constitution::ConstitutionalLayer;

/// Context for determining applicable constitutional layer.
#[derive(Debug, Clone)]
pub struct LayerContext {
    /// Agent ID
    pub agent_id: String,
    /// Family ID (if applicable)
    pub family_id: Option<String>,
    /// Community ID (if applicable)
    pub community_id: Option<String>,
    /// Province/region (if applicable)
    pub province: Option<String>,
    /// Nation (if applicable)
    pub nation: Option<String>,
    /// Bioregion (if applicable)
    pub bioregion: Option<String>,
    /// The determined layer
    determined_layer: ConstitutionalLayer,
}

impl LayerContext {
    /// Create context for individual-only decisions.
    pub fn individual(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            family_id: None,
            community_id: None,
            province: None,
            nation: None,
            bioregion: None,
            determined_layer: ConstitutionalLayer::Individual,
        }
    }

    /// Create a builder.
    pub fn builder(agent_id: impl Into<String>) -> LayerContextBuilder {
        LayerContextBuilder::new(agent_id)
    }

    /// Get the determined constitutional layer.
    pub fn layer(&self) -> ConstitutionalLayer {
        self.determined_layer
    }

    /// Check if context includes family.
    pub fn has_family_context(&self) -> bool {
        self.family_id.is_some()
    }

    /// Check if context includes community.
    pub fn has_community_context(&self) -> bool {
        self.community_id.is_some()
    }

    /// Get the highest layer with context.
    pub fn highest_layer(&self) -> ConstitutionalLayer {
        if self.bioregion.is_some() {
            ConstitutionalLayer::Bioregional
        } else if self.nation.is_some() {
            ConstitutionalLayer::NationState
        } else if self.province.is_some() {
            ConstitutionalLayer::Provincial
        } else if self.community_id.is_some() {
            ConstitutionalLayer::Community
        } else if self.family_id.is_some() {
            ConstitutionalLayer::Family
        } else {
            ConstitutionalLayer::Individual
        }
    }

    /// Check if this context is within a specific layer.
    pub fn is_within(&self, layer: ConstitutionalLayer) -> bool {
        match layer {
            ConstitutionalLayer::Global => true, // Everyone is within global
            ConstitutionalLayer::Bioregional => self.bioregion.is_some(),
            ConstitutionalLayer::NationState => self.nation.is_some(),
            ConstitutionalLayer::Provincial => self.province.is_some(),
            ConstitutionalLayer::Community => self.community_id.is_some(),
            ConstitutionalLayer::Family => self.family_id.is_some(),
            ConstitutionalLayer::Individual => true, // Everyone has individual layer
        }
    }
}

/// Builder for LayerContext.
pub struct LayerContextBuilder {
    context: LayerContext,
}

impl LayerContextBuilder {
    /// Create a new builder.
    pub fn new(agent_id: impl Into<String>) -> Self {
        Self {
            context: LayerContext::individual(agent_id),
        }
    }

    /// Set family context.
    pub fn family(mut self, family_id: impl Into<String>) -> Self {
        self.context.family_id = Some(family_id.into());
        self.context.determined_layer = ConstitutionalLayer::Family;
        self
    }

    /// Set community context.
    pub fn community(mut self, community_id: impl Into<String>) -> Self {
        self.context.community_id = Some(community_id.into());
        self.context.determined_layer = ConstitutionalLayer::Community;
        self
    }

    /// Set province context.
    pub fn province(mut self, province: impl Into<String>) -> Self {
        self.context.province = Some(province.into());
        self.context.determined_layer = ConstitutionalLayer::Provincial;
        self
    }

    /// Set nation context.
    pub fn nation(mut self, nation: impl Into<String>) -> Self {
        self.context.nation = Some(nation.into());
        self.context.determined_layer = ConstitutionalLayer::NationState;
        self
    }

    /// Set bioregion context.
    pub fn bioregion(mut self, bioregion: impl Into<String>) -> Self {
        self.context.bioregion = Some(bioregion.into());
        self.context.determined_layer = ConstitutionalLayer::Bioregional;
        self
    }

    /// Force a specific layer.
    pub fn layer(mut self, layer: ConstitutionalLayer) -> Self {
        self.context.determined_layer = layer;
        self
    }

    /// Build the context.
    pub fn build(self) -> LayerContext {
        self.context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_individual_context() {
        let ctx = LayerContext::individual("agent-1");
        assert_eq!(ctx.layer(), ConstitutionalLayer::Individual);
        assert!(!ctx.has_family_context());
    }

    #[test]
    fn test_builder() {
        let ctx = LayerContext::builder("agent-1")
            .family("family-1")
            .community("community-1")
            .build();

        assert!(ctx.has_family_context());
        assert!(ctx.has_community_context());
        assert_eq!(ctx.highest_layer(), ConstitutionalLayer::Community);
    }
}
