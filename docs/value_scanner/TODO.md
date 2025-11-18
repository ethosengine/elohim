# Value Scanner Epic - TODO

## Epic Overview

**Description**: Care Economy - Value recognition and exchange
**Main Documentation**: `elohim-value-scanner-protocol.md`

This epic explores how the Elohim Protocol enables recognition and exchange of value across care economies, focusing on how different user types interact with value scanning technology at various governance layers.

## User Types (19)

### Life Stage Categories
- [ ] young_child - Pure Discovery phase (5-7 years)
- [ ] child - Childhood development
- [ ] preteen - Pre-adolescent phase
- [ ] teen - Adolescent phase
- [ ] young_adult - Early adulthood
- [ ] adult - Adult phase
- [ ] middle_aged - Middle age
- [ ] senior - Senior citizen
- [ ] elderly - Elderly care recipient
- [ ] retired - Retired individual

### Role-Based Categories
- [ ] parent - Primary caregiver
- [ ] single_parent - Single parent household
- [ ] caregiver - Professional/family caregiver
- [ ] grandparent - Grandparent role
- [ ] worker - Employed worker
- [ ] student - Student in educational system

### Special Needs Categories
- [ ] person_with_disabilities - Individual with disabilities
- [ ] idd_community - Intellectual/developmental disabilities community
- [ ] vulnerable_temporary - Temporarily vulnerable individual

## Progress Tracking

### Phase 1: Structure & Templates ✓
- [x] Create user directories
- [x] Create scenarios/ subdirectories
- [x] Generate README.md templates for each user
- [x] Generate TODO.md templates for each user

### Phase 2: User Story Development
For each user type, complete their README.md:
- [ ] Define archetype and demographics
- [ ] Identify core needs
- [ ] Map key relationships
- [ ] Determine relevant governance layers
- [ ] Add implementation notes

### Phase 3: Scenario Planning
For each user type, in their TODO.md:
- [ ] Review archetype definition
- [ ] Determine which governance layers are relevant
- [ ] Prioritize scenarios (high/medium/low)
- [ ] Identify cross-layer scenarios

### Phase 4: Scenario Creation
For each user type, create scenario files in `scenarios/`:
- [ ] Write individual.md scenarios
- [ ] Write family.md scenarios
- [ ] Write neighborhood.md scenarios
- [ ] Write community.md scenarios
- [ ] Write other relevant layer scenarios
- [ ] Write functional domain scenarios

### Phase 5: Graph Database Preparation
- [ ] Validate YAML frontmatter across all files
- [ ] Ensure consistent relationship mapping
- [ ] Document node and edge types
- [ ] Create import scripts for graph database

## Governance Layer Coverage

### Geographic/Political Layers (11)
- individual, family, neighborhood, community, district, municipality, county_regional, provincial_state, nation_state, continental, global

### Functional Layers (6)
- workplace_organizational, educational, ecological_bioregional, cultural_linguistic, industry_sector, affinity_network

## Implementation Notes

### Subsidiarity Principle
Not every user operates at every governance layer. The principle of subsidiarity means:
- Young children primarily operate at: individual, family, neighborhood
- Adults may operate across: individual → municipality or beyond
- Specialized roles (workers) include functional domains: workplace_organizational, industry_sector

### Graph Database Structure
Each scenario file is designed to become a node in the graph database with:
- **User Type Nodes**: Representing archetypes
- **Scenario Nodes**: Specific situations/stories
- **Layer Nodes**: Governance contexts
- **Edges**: Relationships between users, scenarios, and layers

### Naming Conventions
- User directories: `snake_case`
- Scenario files: `governance_layer_name.md`
- Epic: `value_scanner`
- YAML keys: `snake_case`

## Quick Start

1. Choose a user type to develop
2. Read their README.md template
3. Fill in the [TODO] sections in README.md
4. Review their TODO.md
5. Check relevant governance layers
6. Create scenario files in scenarios/
7. Add YAML frontmatter to each scenario
8. Cross-reference related users and layers

## Questions to Consider

- Which governance layers are most relevant for each user type?
- What cross-layer scenarios show progression through governance scales?
- How do functional domains intersect with geographic layers?
- What relationships exist between different user types?
- How does the Value Scanner Protocol serve different needs at different scales?
