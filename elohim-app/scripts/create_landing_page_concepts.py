#!/usr/bin/env python3
"""
Create dedicated concept nodes for each landing page box/principle.
Uses the ACTUAL text from the homepage HTML as the description,
so the homepage can be recomposed dynamically from graph API calls.

Run from elohim-app root: python scripts/create_landing_page_concepts.py
"""

import json
from pathlib import Path
from datetime import datetime

OUTPUT_DIR = Path("src/assets/lamad-data/content")

# Landing page concepts - using EXACT text from the HTML
LANDING_PAGE_CONCEPTS = [
    # Design Principles Section (from design-principles.component.html)
    {
        "id": "concept-peer-to-peer-architecture",
        "title": "Peer-to-Peer Architecture",
        "section": "design-principles",
        "description": "Distributed infrastructure like Holochain removes single points of failure and control. Edge computing enables community sovereignty and hyper-local governance.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "governance-organizations-holochain-app-framework-with-p2p-networking-readme"
        ],
        "tags": ["architecture", "distributed-systems", "p2p", "design-principle", "landing-page"]
    },
    {
        "id": "concept-graduated-intimacy",
        "title": "Graduated Intimacy",
        "section": "design-principles",
        "description": "Spaces for personal exploration exist alongside protected commons. Consent boundaries prevent individual extremes from corrupting shared spaces.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "social-medium-epic"
        ],
        "tags": ["privacy", "community", "consent", "design-principle", "landing-page"]
    },
    {
        "id": "concept-transparency-as-immune-system",
        "title": "Transparency as Immune System",
        "section": "design-principles",
        "description": "Open governance makes manipulation visible. Behavioral pattern recognition identifies corruption while preserving privacy and dignity.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "public-observer-epic"
        ],
        "tags": ["transparency", "governance", "accountability", "design-principle", "landing-page"]
    },
    {
        "id": "concept-community-wisdom-at-scale",
        "title": "Community Wisdom at Scale",
        "section": "design-principles",
        "description": "Distributed governance systems where local communities steward their own spaces while maintaining connection to broader networks.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "public-observer-epic",
            "autonomous-entity-epic"
        ],
        "tags": ["governance", "subsidiarity", "community", "design-principle", "landing-page"]
    },
    {
        "id": "concept-economic-alignment",
        "title": "Economic Alignment",
        "section": "design-principles",
        "description": "Systems that don't financially reward antisocial behavior. Cooperative ownership structures that support rather than exploit human connection.",
        "relatedNodeIds": [
            "manifesto",
            "economic-coordination-epic",
            "value-scanner-epic",
            "autonomous-entity-epic"
        ],
        "tags": ["economics", "care-economy", "cooperation", "design-principle", "landing-page"]
    },
    {
        "id": "concept-incorruptible-stewardship",
        "title": "Incorruptible Stewardship",
        "section": "design-principles",
        "description": "AI agents that cannot be captured by institutional power, trained on love rather than profit, serving as anonymous guardians of human flourishing.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "lamad-reference-implementation"
        ],
        "tags": ["ai-agents", "elohim", "stewardship", "design-principle", "landing-page"]
    },

    # Crisis Section - Economic Architecture (from crisis.component.html)
    {
        "id": "concept-poverty-of-currency",
        "title": "Poverty of Currency",
        "section": "crisis-economic",
        "description": "In light of modern computation, our currencies are astonishingly primitive.",
        "relatedNodeIds": [
            "manifesto",
            "economic-coordination-epic",
            "value-scanner-epic"
        ],
        "tags": ["crisis", "economics", "currency", "landing-page"]
    },
    {
        "id": "concept-currency-carries-no-values",
        "title": "Carry no values",
        "section": "crisis-economic",
        "description": "A dollar spent on weapons is identical to one spent on medicine",
        "relatedNodeIds": [
            "manifesto",
            "economic-coordination-epic"
        ],
        "tags": ["crisis", "economics", "values", "landing-page"]
    },
    {
        "id": "concept-currency-no-natural-limits",
        "title": "Have no natural limits",
        "section": "crisis-economic",
        "description": "Creating systemic growth imperatives that destroy ecosystems",
        "relatedNodeIds": [
            "manifesto",
            "economic-coordination-epic"
        ],
        "tags": ["crisis", "economics", "limits", "landing-page"]
    },
    {
        "id": "concept-currency-fail-to-reward-care",
        "title": "Fail to reward care",
        "section": "crisis-economic",
        "description": "A mother's love, a teacher's dedication generate no currency",
        "relatedNodeIds": [
            "manifesto",
            "value-scanner-epic",
            "economic-coordination-epic"
        ],
        "tags": ["crisis", "economics", "care-work", "landing-page"]
    },
    {
        "id": "concept-currency-concentrate-not-circulate",
        "title": "Concentrate rather than circulate",
        "section": "crisis-economic",
        "description": "Exponential returns to capital while linear returns to labor",
        "relatedNodeIds": [
            "manifesto",
            "economic-coordination-epic",
            "autonomous-entity-epic"
        ],
        "tags": ["crisis", "economics", "inequality", "landing-page"]
    },

    # Crisis Section - Political Architecture (from crisis.component.html)
    {
        "id": "concept-votes-binary-infrequent",
        "title": "Votes are binary and infrequent",
        "section": "crisis-political",
        "description": "Complex preferences compressed into yes/no choices every few years",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "public-observer-epic"
        ],
        "tags": ["crisis", "governance", "democracy", "landing-page"]
    },
    {
        "id": "concept-representation-capturable",
        "title": "Representation is capturable",
        "section": "crisis-political",
        "description": "Follows artificial political boundaries subject to manipulation rather than natural communities of shared values",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "public-observer-epic"
        ],
        "tags": ["crisis", "governance", "capture", "landing-page"]
    },
    {
        "id": "concept-feedback-loops-broken",
        "title": "Feedback loops are broken",
        "section": "crisis-political",
        "description": "Politicians can safely ignore public preferences between elections",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "public-observer-epic"
        ],
        "tags": ["crisis", "governance", "accountability", "landing-page"]
    },
    {
        "id": "concept-complexity-exceeds-processing",
        "title": "Complexity exceeds human processing",
        "section": "crisis-political",
        "description": "Bills thousands of pages long that no single person understands",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic"
        ],
        "tags": ["crisis", "governance", "complexity", "landing-page"]
    },

    # Crisis Section - Digital Architecture (from crisis.component.html)
    {
        "id": "concept-centralized-control",
        "title": "Centralized control",
        "section": "crisis-digital",
        "description": "creates single points of failure, censorship, and capture",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic",
            "public-observer-epic"
        ],
        "tags": ["crisis", "digital", "centralization", "landing-page"]
    },
    {
        "id": "concept-engagement-optimization",
        "title": "Engagement optimization",
        "section": "crisis-digital",
        "description": "weaponizes human psychology, addicting us to outrage and validation",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic",
            "lamad-reference-implementation"
        ],
        "tags": ["crisis", "digital", "addiction", "manipulation", "landing-page"]
    },
    {
        "id": "concept-one-size-fits-all-moderation",
        "title": "One-size-fits-all moderation",
        "section": "crisis-digital",
        "description": "fails to respect diverse community values and contexts",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic"
        ],
        "tags": ["crisis", "digital", "moderation", "landing-page"]
    },
    {
        "id": "concept-surveillance-capitalism",
        "title": "Surveillance capitalism",
        "section": "crisis-digital",
        "description": "turns human connection into extractive data mining operations",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic",
            "public-observer-epic"
        ],
        "tags": ["crisis", "digital", "capitalism", "extraction", "landing-page"]
    },

    # Vision Section (from vision.component.html)
    {
        "id": "concept-technology-serves-love",
        "title": "Technology serves love",
        "section": "vision",
        "description": "rather than engagement metrics or surveillance capitalism",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "social-medium-epic"
        ],
        "tags": ["vision", "love", "technology", "landing-page"]
    },
    {
        "id": "concept-communities-self-govern",
        "title": "Communities self-govern",
        "section": "vision",
        "description": "without corporate oversight, using transparent, locally-owned infrastructure",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "autonomous-entity-epic"
        ],
        "tags": ["vision", "governance", "community", "landing-page"]
    },
    {
        "id": "concept-vulnerable-people-protected",
        "title": "Vulnerable people are protected",
        "section": "vision",
        "description": "by systems that cannot be corrupted or captured by institutional power",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "value-scanner-epic"
        ],
        "tags": ["vision", "protection", "care", "landing-page"]
    },
    {
        "id": "concept-human-creativity-flourishes",
        "title": "Human creativity flourishes",
        "section": "vision",
        "description": "without algorithmic manipulation or artificial scarcity",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic",
            "economic-coordination-epic"
        ],
        "tags": ["vision", "creativity", "flourishing", "landing-page"]
    },
    {
        "id": "concept-different-values-coexist",
        "title": "Different values coexist",
        "section": "vision",
        "description": "without forcing conformity through graduated intimacy and consent boundaries",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic",
            "concept-graduated-intimacy"
        ],
        "tags": ["vision", "diversity", "coexistence", "landing-page"]
    },
    {
        "id": "concept-dark-patterns-impossible",
        "title": "Dark patterns are impossible",
        "section": "vision",
        "description": "because the architecture doesn't allow exploitation by design",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic",
            "lamad-reference-implementation"
        ],
        "tags": ["vision", "ethics", "design", "landing-page"]
    },
    {
        "id": "concept-connection-genuine",
        "title": "Connection is genuine",
        "section": "vision",
        "description": "rather than performative, fostering real relationships and mutual aid",
        "relatedNodeIds": [
            "manifesto",
            "social-medium-epic",
            "value-scanner-epic"
        ],
        "tags": ["vision", "connection", "authenticity", "landing-page"]
    },
    {
        "id": "concept-growth-encouraged-exploitation-prevented",
        "title": "Growth is encouraged",
        "section": "vision",
        "description": "while exploitation is prevented through economic structures that reward cooperation",
        "relatedNodeIds": [
            "manifesto",
            "economic-coordination-epic",
            "autonomous-entity-epic"
        ],
        "tags": ["vision", "growth", "cooperation", "landing-page"]
    },

    # Elohim Host Section (from elohim-host.component.html)
    {
        "id": "concept-redemptive-security-model",
        "title": "Redemptive Security Model",
        "section": "elohim-host",
        "description": "Rather than blocking threats, these agents understand and heal root causes driving adversarial behavior—economic desperation, ideological extremism, trauma responses. Every attack becomes an opportunity for healing and integration, making the distributed network antifragile.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "social-medium-epic"
        ],
        "tags": ["security", "redemption", "healing", "elohim", "landing-page"]
    },
    {
        "id": "concept-value-generative-economics",
        "title": "Value-Generative Economics",
        "section": "elohim-host",
        "description": "Autonomously maintain economic systems that create wealth through circulation rather than extraction. Reward care work, stewardship, and community contribution while preventing wealth concentration that threatens coordination.",
        "relatedNodeIds": [
            "manifesto",
            "economic-coordination-epic",
            "value-scanner-epic",
            "autonomous-entity-epic"
        ],
        "tags": ["economics", "care-economy", "wealth", "elohim", "landing-page"]
    },
    {
        "id": "concept-cross-layer-verification",
        "title": "Cross-Layer Verification",
        "section": "elohim-host",
        "description": "Agents at different scales (personal, community, institutional) continuously verify each other's constitutional compliance. This distributed verification prevents any single point of corruption while maintaining coherence across the global network.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "public-observer-epic"
        ],
        "tags": ["verification", "security", "governance", "elohim", "landing-page"]
    },

    # Learning Success Section (from learning-success.component.html)
    {
        "id": "concept-scandinavian-insight",
        "title": "The Scandinavian Insight",
        "section": "learning-success",
        "description": "Nordic countries demonstrate that high-trust societies are possible through transparency that makes manipulation visible, safety nets that reduce zero-sum competition, cultural antibodies against antisocial behavior, and economic structures that don't reward exploitation.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "value-scanner-epic",
            "economic-coordination-epic"
        ],
        "tags": ["models", "scandinavia", "trust", "society", "landing-page"]
    },
    {
        "id": "concept-indigenous-wisdom",
        "title": "Indigenous Wisdom",
        "section": "learning-success",
        "description": "Many indigenous cultures maintain communal harmony through restorative justice that heals rather than punishes, collective stewardship of shared resources, and sacred relationship with the commons.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "value-scanner-epic",
            "economic-coordination-epic"
        ],
        "tags": ["models", "indigenous", "wisdom", "stewardship", "landing-page"]
    },
    {
        "id": "concept-intergenerational-thinking",
        "title": "Intergenerational Thinking",
        "section": "learning-success",
        "description": "Both traditions emphasize decision-making that considers impact on future generations, creating sustainable systems that protect rather than exploit human vulnerability.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "value-scanner-epic"
        ],
        "tags": ["models", "sustainability", "future", "landing-page"]
    },
    {
        "id": "concept-engineering-specifications",
        "title": "Engineering Specifications",
        "section": "learning-success",
        "description": "These aren't utopian dreams—they're proven patterns that can be encoded into digital infrastructure. We have blueprints for building technology that supports rather than undermines human flourishing.",
        "relatedNodeIds": [
            "manifesto",
            "lamad-reference-implementation",
            "governance-epic"
        ],
        "tags": ["models", "engineering", "implementation", "landing-page"]
    },

    # Path Forward Section (from path-forward.component.html)
    {
        "id": "concept-path-forward-policymakers",
        "title": "For Policymakers",
        "section": "path-forward",
        "description": "Mandate interoperability. Protect data sovereignty. Fund public digital infrastructure. Create space for pro-social experiments.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "public-observer-epic"
        ],
        "tags": ["path-forward", "policy", "governance", "landing-page"]
    },
    {
        "id": "concept-path-forward-developers",
        "title": "For Developers",
        "section": "path-forward",
        "description": "Build on distributed protocols. Implement privacy by default. Design for consent. Document power structures.",
        "relatedNodeIds": [
            "manifesto",
            "lamad-reference-implementation",
            "governance-epic"
        ],
        "tags": ["path-forward", "development", "implementation", "landing-page"]
    },
    {
        "id": "concept-path-forward-communities",
        "title": "For Communities",
        "section": "path-forward",
        "description": "Demand better than surveillance capitalism. Support cooperative platforms. Build local resilience.",
        "relatedNodeIds": [
            "manifesto",
            "autonomous-entity-epic",
            "public-observer-epic"
        ],
        "tags": ["path-forward", "community", "action", "landing-page"]
    },

    # Call to Action Section (from call-to-action.component.html)
    {
        "id": "concept-love-as-technology",
        "title": "Love as Technology",
        "section": "call-to-action",
        "description": "The radical proposition at the heart of this protocol is that love—not as sentiment but as committed action toward mutual flourishing—can be encoded into technological systems. Not through rules about love, but through architectures that make exploitation structurally impossible while making care structurally supported.",
        "relatedNodeIds": [
            "manifesto",
            "governance-epic",
            "value-scanner-epic",
            "social-medium-epic",
            "lamad-reference-implementation"
        ],
        "tags": ["love", "philosophy", "foundation", "principle", "landing-page"]
    }
]


def create_concept_node(concept_data: dict) -> dict:
    """Create a formatted concept node using landing page text."""
    now = datetime.now().isoformat()

    return {
        "id": concept_data["id"],
        "contentType": "concept",
        "title": concept_data["title"],
        "description": concept_data["description"],  # EXACT text from landing page HTML
        "content": concept_data["description"],  # For now, same as description
        "contentFormat": "text",
        "sourcePath": f"generated/landing-page/{concept_data['section']}/{concept_data['id']}.json",
        "tags": concept_data["tags"],
        "relatedNodeIds": concept_data["relatedNodeIds"],
        "metadata": {
            "category": "landing-page-concept",
            "section": concept_data["section"],
            "isLandingPageConcept": True,
            "canBeLinked": True,
            "displayOnHomepage": True
        },
        "createdAt": now,
        "updatedAt": now
    }


def main():
    print("=" * 60)
    print("Landing Page Concept Node Generator")
    print("=" * 60)
    print("\nCreating concept nodes from landing page content...")
    print("These will enable homepage recomposition via graph API calls.\n")

    # Create output directory
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)

    created_count = 0
    sections = {}

    # Create each concept node
    for concept_data in LANDING_PAGE_CONCEPTS:
        node = create_concept_node(concept_data)
        node_path = OUTPUT_DIR / f"{node['id']}.json"

        with open(node_path, 'w') as f:
            json.dump(node, f, indent=2)

        section = concept_data["section"]
        if section not in sections:
            sections[section] = 0
        sections[section] += 1

        print(f"   [{section}] {node['title']}")
        created_count += 1

    print(f"\n" + "=" * 60)
    print(f"Created {created_count} landing page concept nodes")
    print(f"\nBy section:")
    for section, count in sections.items():
        print(f"  {section}: {count} concepts")
    print(f"\nOutput: {OUTPUT_DIR}")
    print("=" * 60)

    print("\nNext steps:")
    print("1. Run: python scripts/generate_lamad_data.py")
    print("   (to update content index)")
    print("2. Homepage can now query:")
    print("   GET /api/concepts?section=design-principles")
    print("   GET /api/concepts?section=crisis-economic")
    print("   etc.")
    print("3. Each concept links into deeper graph content via relatedNodeIds")


if __name__ == "__main__":
    main()
