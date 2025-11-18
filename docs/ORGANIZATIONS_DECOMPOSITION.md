# Organizations Decomposition Summary

**Date:** 2025-11-18
**Source:** `docs/keen.json`
**Total Organizations:** 103

## Overview

This document summarizes the decomposition of organizations from `keen.json` into the Elohim Protocol project documentation structure. Each organization from the keen.json collection has been classified and distributed across the five epics based on their primary alignment.

## Key Principles

### Single Source of Truth
- **NO DUPLICATION**: Each organization lives in ONE location (its primary epic)
- **YAML Declares Relationships**: All cross-epic connections are defined in YAML frontmatter
- **Graph Edges from YAML**: The graph database will create edges from YAML metadata
- **103 Organizations → 103 Files → ~850-1300 Graph Edges**

### Organization Node Structure

Each organization file includes:
- `node_type: organization`
- `primary_epic`: Where the file physically lives
- `related_epics`: Creates graph edges to other epics (no file duplication)
- `inspires_users`: Can reference users from ANY epic
- `operates_at_layers`: All relevant governance layers
- `demonstrates_principles`: Which principles they exemplify

## Distribution by Epic

### Value Scanner (18 organizations)
**Focus:** Care economy, mutual aid, commons, value recognition

Key organizations include:
- B Lab Global Site
- Commons Engine
- P2P Foundation
- Transition Network
- ValueFlows

### Public Observer (12 organizations)
**Focus:** Civic tech, democracy, transparency, deliberation

Key organizations include:
- New_ Public (multiple instances)
- BlockchainGov
- Democracy Earth Foundation
- Forward Labs
- Sociocracy principles

### Autonomous Entity (3 organizations)
**Focus:** Workplace democracy, worker cooperatives, self-management

Key organizations include:
- Conversence's IdeaLoom platform
- Design Justice Network
- Integrity Institute

### Governance (60 organizations)
**Focus:** Coordination, systems thinking, meta-crisis, collective intelligence

Key organizations include:
- The Consilience Project
- IFTF (Institute for the Future)
- RadicalxChange
- Holochain
- Polis
- Various indices (Happy Planet Index, Human Development Index, etc.)
- Systems thinkers and frameworks

### Social Medium (10 organizations)
**Focus:** Humane tech, digital well-being, online community

Key organizations include:
- Foundations of Humane Technology Course
- Humane by Design
- Ground News
- PubHubs
- Open Collective

## Cross-Epic Relationships

**Total related_epics connections:** 47
**Organizations with cross-epic connections:** 35

### Organizations with Multiple Epic Connections

Top organizations with multiple epic relationships:
1. **Home - Initiative for Digital Public Infrastructure** → [public_observer, social_medium]
2. **A Problem Well-Stated is Half-Solved | Your Undivided Attention** → [public_observer, social_medium]
3. **Conversence's IdeaLoom platform** → [governance, social_medium]
4. **Bonfire** → [social_medium, value_scanner]
5. **Hylo — Social Coordination for a Thriving Planet** → [value_scanner, autonomous_entity]

## Directory Structure

```
elohim/docs/
├── value_scanner/organizations/        (18 orgs)
│   ├── b_lab_global_site/
│   │   └── README.md
│   ├── commons_engine_designing_commons_oriented_economies/
│   │   └── README.md
│   └── ...
│
├── public_observer/organizations/      (12 orgs)
│   ├── new__public_for_better_digital_public_spaces/
│   │   └── README.md
│   ├── blockchaingov/
│   │   └── README.md
│   └── ...
│
├── autonomous_entity/organizations/    (3 orgs)
│   ├── conversence39s_idealoom_platform/
│   │   └── README.md
│   ├── design_justice_network/
│   │   └── README.md
│   └── ...
│
├── governance/organizations/           (60 orgs)
│   ├── the_consilience_project/
│   │   └── README.md
│   ├── iftf_home/
│   │   └── README.md
│   └── ...
│
└── social_medium/organizations/        (10 orgs)
    ├── foundations_of_humane_technology_course/
    │   └── README.md
    ├── humane_by_design/
    │   └── README.md
    └── ...
```

## YAML Frontmatter Structure

Each organization README.md includes:

```yaml
---
node_type: organization
org_id: [unique_slug]
name: "[Display Name]"
url: "[Primary URL]"
gem_id: "[original keen.json gemId]"

# Epic relationships (NO DUPLICATION NEEDED)
primary_epic: [epic_name]
related_epics: [epic1, epic2]

epic_relationships:
  primary_epic:
    inspiration: "[How they inspire this epic]"
    parallel_work: []
  related_epic:
    inspiration: "[How they inspire this epic]"
    parallel_work: []

demonstrates_principles: []
inspires_users: []
operates_at_layers: []

edge_types:
  - inspires_epic
  - demonstrates_principle
  - aligns_with_user
  - operates_at_layer
---
```

## Validation Results

✅ **All 103 organization files validated successfully**
- All required YAML fields present
- All `primary_epic` values match directory location
- All `org_id` values match directory names
- All `related_epics` reference valid epic names
- All YAML frontmatter parses correctly

## Graph Import Implications

From these 103 organization files, the graph database will generate approximately:

- **103 Organization nodes**
- **~150-200 inspires_epic edges** (primary + related)
- **~300-500 aligns_with_user edges** (to be enriched)
- **~200-300 operates_at_layer edges** (to be enriched)
- **~200-300 demonstrates_principle edges** (to be enriched)

**Total: ~850-1300 edges from 103 single-location files**

## Next Steps for Enrichment

Each organization file includes placeholder sections that can be enriched with:

1. **Vision Alignment**: Specific connections to Elohim Protocol vision
2. **Multi-Epic Inspiration**: Detailed explanations of how they inspire each epic
3. **Parallel Work**: Specific initiatives that parallel epic scenarios
4. **Demonstrations of Principles**: Concrete examples of principles in action
5. **User Inspiration**: Which specific users they inspire and why
6. **Layer Operations**: Which governance layers they operate at
7. **Key Resources**: Important links, people, and reading materials

## Classification Methodology

Organizations were classified using keyword scoring across five dimensions:

- **value_scanner**: care, economy, commons, mutual aid, cooperation
- **public_observer**: civic, democracy, transparency, governance
- **autonomous_entity**: workplace, worker, cooperative, labor
- **governance**: coordination, meta-crisis, systems, collective intelligence
- **social_medium**: social media, attention, digital, humane tech

Primary epic = highest score; related epics = secondary scores.

## Reference to Source

All organizations traced back to original `keen.json` via `gem_id` field in YAML frontmatter.

---

**Mission Accomplished:** 103 organizations decomposed into project documentation structure with NO DUPLICATION, ready for graph import and future enrichment.
