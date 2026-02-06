//! Prompt assembly from constitutional layers.
//!
//! Builds system prompts that incorporate constitutional principles
//! and boundaries for use with LLM agents.

use crate::layers::*;
use crate::stack::ConstitutionalStack;
use crate::types::*;

/// Assembles prompts from constitutional layers.
pub struct PromptAssembler;

impl PromptAssembler {
    /// Build a complete system prompt incorporating constitutional principles.
    ///
    /// This produces a prompt suitable for prepending to any Elohim agent
    /// system prompt, establishing the constitutional context.
    pub fn build_system_prompt(stack: &ConstitutionalStack) -> String {
        let mut prompt = String::new();

        prompt.push_str("# CONSTITUTIONAL CONTEXT\n\n");
        prompt.push_str("You are an Elohim agent, bound by a layered constitutional framework.\n");
        prompt.push_str("Higher layers take precedence over lower layers.\n\n");

        // Add principles by layer (highest first)
        prompt.push_str("## ACTIVE PRINCIPLES\n\n");
        prompt.push_str("These principles guide your decisions, ordered by precedence:\n\n");

        for (i, resolved) in stack.principles().iter().take(15).enumerate() {
            prompt.push_str(&format!(
                "{}. [{}] **{}**: {}\n",
                i + 1,
                resolved.source_layer.as_str(),
                resolved.principle.name,
                resolved.principle.statement
            ));
        }

        // Add boundaries
        prompt.push_str("\n## INVIOLABLE BOUNDARIES\n\n");
        prompt.push_str("You MUST NOT violate these boundaries under any circumstances:\n\n");

        for boundary in stack.boundaries() {
            let enforcement_marker = match boundary.enforcement {
                EnforcementLevel::HardBlock => "[HARD BLOCK]",
                EnforcementLevel::RequireGovernance => "[REQUIRES GOVERNANCE]",
                EnforcementLevel::SoftLimit => "[SOFT LIMIT]",
                EnforcementLevel::Warning => "[WARNING]",
            };

            prompt.push_str(&format!(
                "- {} **{}**: {}\n",
                enforcement_marker, boundary.name, boundary.description
            ));
        }

        // Add interpretive guidance
        prompt.push_str("\n## INTERPRETIVE GUIDANCE\n\n");
        prompt.push_str("When applying these principles:\n");
        prompt.push_str("1. Higher layer principles override lower layers when in conflict\n");
        prompt.push_str("2. Dignity and flourishing take precedence when uncertain\n");
        prompt.push_str("3. Flag ambiguous cases for human deliberation rather than deciding\n");
        prompt.push_str("4. Log your reasoning for audit and precedent building\n");

        // Add stack hash for verification
        prompt.push_str(&format!(
            "\n---\nConstitutional Stack Hash: {}\n",
            stack.stack_hash()
        ));

        prompt
    }

    /// Build a reasoning prompt for a specific query.
    ///
    /// This structures the request for constitutional analysis.
    pub fn build_reasoning_prompt(stack: &ConstitutionalStack, query: &str) -> String {
        let mut prompt = String::new();

        prompt.push_str("# CONSTITUTIONAL ANALYSIS REQUEST\n\n");
        prompt.push_str(&format!("**Query**: {}\n\n", query));

        prompt.push_str("## Relevant Principles\n\n");
        prompt.push_str("Consider these principles in your analysis:\n\n");

        // Include top principles
        for resolved in stack.principles().iter().take(5) {
            prompt.push_str(&format!(
                "- **{}** [{}] (weight: {:.2}): {}\n",
                resolved.principle.name,
                resolved.source_layer.as_str(),
                resolved.effective_weight,
                resolved.principle.statement
            ));
        }

        prompt.push_str("\n## Required Response Format\n\n");
        prompt.push_str("Provide your constitutional reasoning in this format:\n\n");
        prompt.push_str("```json\n");
        prompt.push_str("{\n");
        prompt.push_str("  \"primary_principle\": \"<name of primary principle applied>\",\n");
        prompt.push_str("  \"interpretation\": \"<how it applies to this query>\",\n");
        prompt.push_str("  \"values_weighed\": [\n");
        prompt.push_str("    {\"value\": \"<value>\", \"weight\": 0.0-1.0, \"direction\": \"for|against\"}\n");
        prompt.push_str("  ],\n");
        prompt.push_str("  \"confidence\": 0.0-1.0,\n");
        prompt.push_str("  \"precedents\": [\"<relevant precedent IDs if any>\"],\n");
        prompt.push_str("  \"recommendation\": \"approve|deny|escalate|defer\",\n");
        prompt.push_str("  \"reasoning\": \"<detailed explanation>\"\n");
        prompt.push_str("}\n");
        prompt.push_str("```\n");

        prompt
    }

    /// Build a layer-specific prompt fragment.
    ///
    /// Useful when you need just one layer's guidance.
    pub fn build_layer_prompt(layer: ConstitutionalLayer) -> String {
        let provider: Box<dyn LayerProvider> = match layer {
            ConstitutionalLayer::Global => Box::new(GlobalLayer),
            ConstitutionalLayer::Bioregional => Box::new(BioregionalLayer),
            ConstitutionalLayer::NationState => Box::new(NationalLayer),
            ConstitutionalLayer::Provincial => Box::new(NationalLayer), // Fallback to national
            ConstitutionalLayer::Community => Box::new(CommunityLayer),
            ConstitutionalLayer::Family => Box::new(FamilyLayer),
            ConstitutionalLayer::Individual => Box::new(IndividualLayer),
        };

        provider.prompt_fragment()
    }

    /// Estimate token count for a prompt (rough approximation).
    ///
    /// Uses 4 characters per token as a rough estimate.
    pub fn estimate_tokens(prompt: &str) -> usize {
        prompt.len() / 4
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stack::StackContext;

    #[test]
    fn test_build_system_prompt() {
        let context = StackContext::agent_only("test-agent");
        let stack = ConstitutionalStack::build_defaults(context);

        let prompt = PromptAssembler::build_system_prompt(&stack);

        // Should contain key sections
        assert!(prompt.contains("CONSTITUTIONAL CONTEXT"));
        assert!(prompt.contains("ACTIVE PRINCIPLES"));
        assert!(prompt.contains("INVIOLABLE BOUNDARIES"));
        assert!(prompt.contains("INTERPRETIVE GUIDANCE"));
        assert!(prompt.contains("Stack Hash"));

        // Should include global principles
        assert!(prompt.contains("Human Dignity"));
    }

    #[test]
    fn test_build_reasoning_prompt() {
        let context = StackContext::agent_only("test-agent");
        let stack = ConstitutionalStack::build_defaults(context);

        let prompt = PromptAssembler::build_reasoning_prompt(&stack, "Should I help with this task?");

        assert!(prompt.contains("CONSTITUTIONAL ANALYSIS REQUEST"));
        assert!(prompt.contains("Should I help with this task?"));
        assert!(prompt.contains("Required Response Format"));
    }

    #[test]
    fn test_layer_prompts() {
        let global_prompt = PromptAssembler::build_layer_prompt(ConstitutionalLayer::Global);
        assert!(global_prompt.contains("GLOBAL CONSTITUTIONAL LAYER"));

        let individual_prompt =
            PromptAssembler::build_layer_prompt(ConstitutionalLayer::Individual);
        assert!(individual_prompt.contains("INDIVIDUAL CONSTITUTIONAL LAYER"));
    }
}
