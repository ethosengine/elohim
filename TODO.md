# Elohim Protocol - Graph Structure TODO

## Overview

This repository contains the graph-based structure for the Elohim Protocol, organized to facilitate eventual import into a graph database. The structure represents user stories and scenarios across multiple dimensions:

1. **Epic Domains** (5) - High-level protocol applications
2. **User Types** (48) - Archetypes across all epics
3. **Governance Layers** (17) - Scales from individual to global
4. **Scenarios** (~816 possible combinations) - Specific situations at (epic, user, layer) intersections

## Repository Structure

```
elohim/
├── docs/                          # Core documentation
│   ├── manifesto.md
│   ├── elohim-governance-layers-architecture.md
│   ├── elohim-global-orchestra.md
│   ├── elohim-observer-protocol.md
│   └── elohim_hardware_spec.md
│
├── value_scanner/                 # Care Economy Epic (19 users)
│   ├── TODO.md                    # Epic-level planning
│   ├── elohim-value-scanner-protocol.md
│   └── [user_type]/
│       ├── README.md              # User archetype definition
│       ├── TODO.md                # Scenario planning checklist
│       └── scenarios/             # Governance layer scenarios
│           ├── individual.md
│           ├── family.md
│           └── [other_relevant_layers].md
│
├── public_observer/               # Civic Democracy Epic (9 users)
├── autonomous_entity/             # Workplace Transformation Epic (7 users)
├── governance/                    # AI Governance Epic (6 users)
├── social_medium/                 # Digital Communication Epic (7 users)
│
└── governance_layers/             # Constitutional Architecture Reference
    ├── geographic_political/      # 11 layers
    │   ├── individual/
    │   ├── family/
    │   ├── neighborhood/
    │   ├── community/
    │   ├── district/
    │   ├── municipality/
    │   ├── county_regional/
    │   ├── provincial_state/
    │   ├── nation_state/
    │   ├── continental/
    │   └── global/
    └── functional/                # 6 domains
        ├── workplace_organizational/
        ├── educational/
        ├── ecological_bioregional/
        ├── cultural_linguistic/
        ├── industry_sector/
        └── affinity_network/
```

## Epic Summaries

### 1. Value Scanner (19 users)
**Care Economy** - Value recognition and exchange across life stages and care contexts
- Life stages: young_child → elderly
- Roles: parent, caregiver, worker, student
- Special needs: disabilities, vulnerable populations

### 2. Public Observer (9 users)
**Civic Democracy** - Public oversight and democratic participation
- Civic roles: citizen, activist, community_organizer
- Institutional: politician, journalist, teacher, board_member
- Stakeholders: parent, developer_interests

### 3. Autonomous Entity (7 users)
**Workplace Transformation** - Distributed ownership and governance
- Internal: worker, manager
- Operators: small_business_owner, franchise_operator
- External: supplier, customer, community_investor

### 4. Governance (6 users)
**AI Governance** - Constitutional oversight and appeals
- Participants: appellant, community_leader
- Officials: constitutional_council_member, policy_maker
- Experts: researcher, technical_expert

### 5. Social Medium (7 users)
**Digital Communication** - Relationship-centered social media
- Life stages: child, elder
- Roles: content_creator, community_moderator, activist
- Vulnerable: refugee, displaced_person

## Development Phases

### Phase 1: Structure & Templates ✓ (Complete)
- [x] Create epic directories with user subdirectories
- [x] Create scenarios/ directories for each user
- [x] Generate README.md templates (48 users)
- [x] Generate TODO.md templates (48 users)
- [x] Create epic-level TODO.md files (5 epics)
- [x] Create this master TODO.md

### Phase 2: User Story Development (In Progress)
For each of 48 users, complete their README.md with:
- [ ] Define archetype (who they are, demographics, role)
- [ ] Identify 3-5 core needs
- [ ] Map key relationships to other users and agents
- [ ] Determine relevant governance layers (subsidiarity principle)
- [ ] Add implementation notes (technical, privacy, interface)

**Progress**: 1/48 complete (value_scanner/young_child)

### Phase 3: Scenario Planning (Pending)
For each user, in their TODO.md:
- [ ] Review completed archetype definition
- [ ] Check which governance layers are relevant
- [ ] Prioritize scenarios (high/medium/low)
- [ ] Identify cross-layer scenarios
- [ ] Note special considerations

### Phase 4: Scenario Creation (Pending)
For each user, create scenario files:
- [ ] Write scenario .md files for relevant layers
- [ ] Include proper YAML frontmatter
- [ ] Tell complete user stories
- [ ] Explain governance context
- [ ] Detail protocol interactions
- [ ] Describe outcomes

**Estimated scenarios**: ~200-400 (not all 816 combinations are relevant)

### Phase 5: Graph Database Preparation (Pending)
- [ ] Validate all YAML frontmatter
- [ ] Ensure consistent relationship mapping
- [ ] Document node types (User, Scenario, Layer, Epic)
- [ ] Document edge types (operates_at, relates_to, interacts_with)
- [ ] Create graph import scripts
- [ ] Define graph queries for common patterns

## Design Principles

### 1. Subsidiarity Principle
Not every user operates at every governance layer. Users naturally operate at scales appropriate to their role:
- Young children: individual, family, neighborhood
- Adults: individual → municipality or beyond
- Civic leaders: community → nation_state or global
- Workers: individual + workplace_organizational + industry_sector

### 2. Graph-First Design
Each markdown file is designed to become a node in the graph database:
- **YAML frontmatter** → Node properties and edge definitions
- **File location** → Hierarchical relationships
- **Content** → Node data for human readability
- **Cross-references** → Edge relationships

### 3. Human-Readable, Machine-Parseable
- Markdown for human reading
- YAML for machine processing
- Directory structure for organization
- Git for version control
- Clear naming conventions

### 4. Incremental Development
- Start with core user stories
- Build out high-priority scenarios
- Gradually expand coverage
- Continuously refine relationships
- Evolve as protocol develops

## Quick Start Guide

### To Contribute a User Story:

1. Choose an epic and user type
2. Navigate to `[epic]/[user_type]/`
3. Edit `README.md` to complete the archetype definition
4. Review `TODO.md` to understand scenario requirements
5. Create scenario files in `scenarios/` for relevant layers
6. Use the young_child example as a template

### To Query the Structure:

```bash
# Find all scenarios for a user type
find value_scanner/young_child/scenarios -name "*.md"

# Find all users in an epic
ls -1 public_observer | grep -v ".md" | grep -v ".gitkeep"

# Find all individual-layer scenarios across all epics
find . -path "*/scenarios/individual.md"

# Count total user types
find . -type d -name "scenarios" | wc -l
```

## Governance Layer Reference

### Geographic/Political (11 layers)
Scale from most local to most global:
1. **individual** - Personal decisions and data
2. **family** - Household unit
3. **neighborhood** - Local neighbors (dozens)
4. **community** - Local community (hundreds to thousands)
5. **district** - Municipal district/ward
6. **municipality** - City/town
7. **county_regional** - County or region
8. **provincial_state** - State/province
9. **nation_state** - Country
10. **continental** - Continental bloc (EU, AU, etc.)
11. **global** - Worldwide

### Functional (6 domains)
Cross-cutting governance domains:
1. **workplace_organizational** - Work and organizations
2. **educational** - Educational institutions
3. **ecological_bioregional** - Environmental/bioregional
4. **cultural_linguistic** - Cultural and language communities
5. **industry_sector** - Industry-specific governance
6. **affinity_network** - Interest-based networks

## Graph Database Schema (Proposed)

### Node Types
- **Epic**: value_scanner, public_observer, autonomous_entity, governance, social_medium
- **UserType**: 48 archetypes
- **GovernanceLayer**: 17 layers (11 geographic + 6 functional)
- **Scenario**: Specific situations/stories
- **Agent**: Elohim agents (personal_agent, family_elohim, etc.)

### Edge Types
- **operates_at**: UserType → GovernanceLayer
- **has_scenario**: UserType → Scenario
- **occurs_at**: Scenario → GovernanceLayer
- **relates_to**: UserType → UserType
- **interacts_with**: UserType → Agent
- **part_of**: Scenario → Epic

## Next Steps

1. **Prioritize user stories**: Identify which 10-15 user archetypes are most critical
2. **Complete archetypes**: Finish README.md for priority users
3. **Write key scenarios**: Create 2-3 scenarios for each priority user
4. **Test graph import**: Validate YAML structure with graph database
5. **Iterate**: Expand coverage based on learnings

## Questions & Discussion

- Which user archetypes should we prioritize first?
- What graph database will we use? (Neo4j, ArangoDB, etc.)
- How do we handle scenarios that span multiple layers?
- What query patterns do we anticipate?
- How do we version control scenario evolution?

## Resources

- Main documentation: `/data/content/elohim-protocol/`
- Governance architecture: `/data/content/elohim-protocol/governance-layers-architecture.md`
- Governance reference: `/governance_layers/`
- Generation script: `/generate_user_templates.py`

---

**Status**: Phase 1 Complete ✓ | Phase 2 In Progress (2%)
**Last Updated**: 2025-11-18
