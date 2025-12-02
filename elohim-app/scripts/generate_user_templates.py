#!/usr/bin/env python3
"""
Generate README.md and TODO.md templates for all user types across epics.
"""

import os
from pathlib import Path

# Define epic configurations
EPICS = {
    "value_scanner": {
        "description": "Care Economy - Value recognition and exchange",
        "users": [
            "adult", "caregiver", "child", "elderly", "grandparent",
            "idd_community", "middle_aged", "parent", "person_with_disabilities",
            "preteen", "retired", "senior", "single_parent", "student",
            "teen", "vulnerable_temporary", "worker", "young_adult", "young_child"
        ]
    },
    "public_observer": {
        "description": "Civic Democracy - Public oversight and participation",
        "users": [
            "activist", "board_member", "citizen", "community_organizer",
            "developer_interests", "journalist", "parent", "politician", "teacher"
        ]
    },
    "autonomous_entity": {
        "description": "Workplace Transformation - Distributed ownership and governance",
        "users": [
            "community_investor", "customer", "franchise_operator",
            "manager", "small_business_owner", "supplier", "worker"
        ]
    },
    "governance": {
        "description": "AI Governance - Constitutional oversight and appeals",
        "users": [
            "appellant", "community_leader", "constitutional_council_member",
            "policy_maker", "researcher", "technical_expert"
        ]
    },
    "social_medium": {
        "description": "Digital Communication - Relationship-centered social media",
        "users": [
            "activist", "child", "community_moderator", "content_creator",
            "displaced_person", "elder", "refugee"
        ]
    }
}

# Governance layers
GEOGRAPHIC_LAYERS = [
    "individual", "family", "neighborhood", "community", "district",
    "municipality", "county_regional", "provincial_state", "nation_state",
    "continental", "global"
]

FUNCTIONAL_LAYERS = [
    "workplace_organizational", "educational", "ecological_bioregional",
    "cultural_linguistic", "industry_sector", "affinity_network"
]

def generate_readme(epic, user_type, epic_desc):
    """Generate README.md template for a user type."""
    user_display = user_type.replace("_", " ").title()

    return f"""---
epic: {epic}
user_type: {user_type}
archetype_name: "{user_display}"
epic_domain: "{epic_desc}"
governance_scope: [TO_BE_DEFINED]
related_users: [TO_BE_DEFINED]
---

# {user_display} - {epic.replace("_", " ").title()}

## Archetype

[**TODO**: Define the archetype for this user type. Describe who they are, their role in the {epic} epic, age/demographics if relevant, and their relationship to the protocol.]

## Core Needs

[**TODO**: Identify 3-5 core needs this user type has that the {epic} epic addresses:]

- **Need 1**: Description
- **Need 2**: Description
- **Need 3**: Description
- **Need 4**: Description
- **Need 5**: Description

## Key Relationships

[**TODO**: List the key relationships this user type has with other users, agents, or system components:]

- **related_user_1**: Description of relationship
- **related_user_2**: Description of relationship
- **agent_type**: Description of how they interact with Elohim agents

## Relevant Governance Layers

[**TODO**: Determine which governance layers are relevant for this user type based on the principle of subsidiarity. Not all users operate at all layers.]

### Geographic/Political

- **layer_name**: Brief description of what happens at this layer for this user

### Functional

- **layer_name**: Brief description of functional domain relevance

## Implementation Notes

[**TODO**: Add any specific considerations for implementing scenarios for this user type:]

- Key technical requirements
- Privacy considerations
- Interface design needs
- Data handling requirements
- Unique constraints or opportunities
"""

def generate_todo(epic, user_type):
    """Generate TODO.md template for scenario planning."""
    user_display = user_type.replace("_", " ").title()

    return f"""# Scenarios TODO - {user_display} ({epic.replace("_", " ").title()})

## Overview

This file tracks which scenario files need to be created for the `{user_type}` user type in the `{epic}` epic.

## Instructions

1. **Review README.md** to understand this user's archetype, needs, and relationships
2. **Determine Relevant Layers**: Based on subsidiarity principle, identify which governance layers this user operates at
3. **Create Scenario Files**: For each relevant layer, create a scenario file in `scenarios/[layer_name].md`
4. **Update Checklist**: Check off items as scenario files are created

## Required Scenario Files

### Geographic/Political Layers

[**TODO**: Check which of these layers are relevant for {user_type}, then create scenario files accordingly]

- [ ] `scenarios/individual.md` - Personal/individual level scenarios
- [ ] `scenarios/family.md` - Family unit scenarios
- [ ] `scenarios/neighborhood.md` - Neighborhood level scenarios
- [ ] `scenarios/community.md` - Local community scenarios
- [ ] `scenarios/district.md` - District/ward level scenarios
- [ ] `scenarios/municipality.md` - City/municipal level scenarios
- [ ] `scenarios/county_regional.md` - County/regional level scenarios
- [ ] `scenarios/provincial_state.md` - State/provincial level scenarios
- [ ] `scenarios/nation_state.md` - National level scenarios
- [ ] `scenarios/continental.md` - Continental/bloc level scenarios
- [ ] `scenarios/global.md` - Global level scenarios

### Functional Layers

[**TODO**: Check which of these functional domains are relevant for {user_type}]

- [ ] `scenarios/workplace_organizational.md` - Workplace/organizational scenarios
- [ ] `scenarios/educational.md` - Educational institution scenarios
- [ ] `scenarios/ecological_bioregional.md` - Ecological/bioregional scenarios
- [ ] `scenarios/cultural_linguistic.md` - Cultural/linguistic community scenarios
- [ ] `scenarios/industry_sector.md` - Industry sector scenarios
- [ ] `scenarios/affinity_network.md` - Affinity network scenarios

## Scenario File Format

Each scenario file should follow this structure:

```yaml
---
epic: {epic}
user_type: {user_type}
governance_layer: [layer_name]
scene: [specific_scene_name]
related_users: [list_of_related_users]
related_layers: [list_of_related_layers]
interacts_with: [list_of_elohim_agents]
---

# [Layer Name] Scenario - {user_display}

## Context

[Describe the situation and setting]

## User Story

[Tell the story of what happens in this scenario]

## Governance Context

[Explain how this governance layer operates in this scenario]

## Protocol Interactions

[Detail how the Elohim Protocol components interact in this scenario]

## Outcomes

[Describe what happens as a result]
```

## Implementation Priority

[**TODO**: Once you've identified relevant scenarios, prioritize them:]

1. **High Priority**: [List most critical scenarios]
2. **Medium Priority**: [List important but not critical scenarios]
3. **Low Priority**: [List nice-to-have scenarios]

## Cross-Layer Scenarios

[**TODO**: Identify scenarios that span multiple governance layers]

- Scenario involving [layer1] and [layer2]
- Scenario showing transition from [layer] to [layer]

## Notes

[**TODO**: Add any special considerations for {user_type} scenarios in {epic} epic]

- Special requirement 1
- Special requirement 2
- Edge cases to consider
"""

def main():
    """Generate all README and TODO files."""
    base_path = Path("/home/user/elohim")

    for epic, config in EPICS.items():
        epic_desc = config["description"]

        for user in config["users"]:
            user_path = base_path / epic / user

            # Generate README.md
            readme_path = user_path / "README.md"
            if not readme_path.exists():
                readme_content = generate_readme(epic, user, epic_desc)
                readme_path.write_text(readme_content)
                print(f"Created: {readme_path}")
            else:
                print(f"Skipped (exists): {readme_path}")

            # Generate TODO.md
            todo_path = user_path / "TODO.md"
            if not todo_path.exists():
                todo_content = generate_todo(epic, user)
                todo_path.write_text(todo_content)
                print(f"Created: {todo_path}")
            else:
                print(f"Skipped (exists): {todo_path}")

    print("\n✓ Template generation complete!")
    print(f"\nGenerated files for {sum(len(config['users']) for config in EPICS.values())} user types across {len(EPICS)} epics.")

if __name__ == "__main__":
    main()
