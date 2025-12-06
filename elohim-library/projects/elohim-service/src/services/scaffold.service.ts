/**
 * Scaffold Service
 *
 * Generates README.md and TODO.md templates for user types.
 * Ported from generate_user_templates.py
 */

import * as fs from 'fs';
import * as path from 'path';

/**
 * Epic configuration
 */
export interface EpicConfig {
  id: string;
  description: string;
  users: string[];
}

/**
 * Default epic configurations
 */
export const EPICS: Record<string, EpicConfig> = {
  value_scanner: {
    id: 'value-scanner',
    description: 'Care Economy - Value recognition and exchange',
    users: [
      'adult', 'caregiver', 'child', 'elderly', 'grandparent',
      'idd_community', 'middle_aged', 'parent', 'person_with_disabilities',
      'preteen', 'retired', 'senior', 'single_parent', 'student',
      'teen', 'vulnerable_temporary', 'worker', 'young_adult', 'young_child'
    ]
  },
  public_observer: {
    id: 'public-observer',
    description: 'Civic Democracy - Public oversight and participation',
    users: [
      'activist', 'board_member', 'citizen', 'community_organizer',
      'developer_interests', 'journalist', 'parent', 'politician', 'teacher'
    ]
  },
  autonomous_entity: {
    id: 'autonomous-entity',
    description: 'Workplace Transformation - Distributed ownership and governance',
    users: [
      'community_investor', 'customer', 'franchise_operator',
      'manager', 'small_business_owner', 'supplier', 'worker'
    ]
  },
  governance: {
    id: 'governance',
    description: 'AI Governance - Constitutional oversight and appeals',
    users: [
      'appellant', 'community_leader', 'constitutional_council_member',
      'policy_maker', 'researcher', 'technical_expert'
    ]
  },
  social_medium: {
    id: 'social-medium',
    description: 'Digital Communication - Relationship-centered social media',
    users: [
      'activist', 'child', 'community_moderator', 'content_creator',
      'displaced_person', 'elder', 'refugee'
    ]
  }
};

/**
 * Geographic governance layers
 */
export const GEOGRAPHIC_LAYERS = [
  'individual', 'family', 'neighborhood', 'community', 'district',
  'municipality', 'county_regional', 'provincial_state', 'nation_state',
  'continental', 'global'
];

/**
 * Functional governance layers
 */
export const FUNCTIONAL_LAYERS = [
  'workplace_organizational', 'educational', 'ecological_bioregional',
  'cultural_linguistic', 'industry_sector', 'affinity_network'
];

/**
 * Format user type for display (snake_case → Title Case)
 */
function formatUserType(userType: string): string {
  return userType
    .split('_')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Format epic name for display (snake_case → Title Case)
 */
function formatEpic(epic: string): string {
  return epic
    .split('_')
    .map(word => word.charAt(0).toUpperCase() + word.slice(1))
    .join(' ');
}

/**
 * Generate README.md template for a user type
 */
export function generateReadme(epic: string, userType: string, epicDesc: string): string {
  const userDisplay = formatUserType(userType);
  const epicDisplay = formatEpic(epic);

  return `---
epic: ${epic}
user_type: ${userType}
archetype_name: "${userDisplay}"
epic_domain: "${epicDesc}"
governance_scope: [TO_BE_DEFINED]
related_users: [TO_BE_DEFINED]
---

# ${userDisplay} - ${epicDisplay}

## Archetype

[**TODO**: Define the archetype for this user type. Describe who they are, their role in the ${epic} epic, age/demographics if relevant, and their relationship to the protocol.]

## Core Needs

[**TODO**: Identify 3-5 core needs this user type has that the ${epic} epic addresses:]

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
`;
}

/**
 * Generate TODO.md template for scenario planning
 */
export function generateTodo(epic: string, userType: string): string {
  const userDisplay = formatUserType(userType);
  const epicDisplay = formatEpic(epic);

  const geographicChecklist = GEOGRAPHIC_LAYERS.map(layer =>
    `- [ ] \`scenarios/${layer}.md\` - ${formatUserType(layer)} level scenarios`
  ).join('\n');

  const functionalChecklist = FUNCTIONAL_LAYERS.map(layer =>
    `- [ ] \`scenarios/${layer}.md\` - ${formatUserType(layer)} scenarios`
  ).join('\n');

  return `# Scenarios TODO - ${userDisplay} (${epicDisplay})

## Overview

This file tracks which scenario files need to be created for the \`${userType}\` user type in the \`${epic}\` epic.

## Instructions

1. **Review README.md** to understand this user's archetype, needs, and relationships
2. **Determine Relevant Layers**: Based on subsidiarity principle, identify which governance layers this user operates at
3. **Create Scenario Files**: For each relevant layer, create a scenario file in \`scenarios/[layer_name].md\`
4. **Update Checklist**: Check off items as scenario files are created

## Required Scenario Files

### Geographic/Political Layers

[**TODO**: Check which of these layers are relevant for ${userType}, then create scenario files accordingly]

${geographicChecklist}

### Functional Layers

[**TODO**: Check which of these functional domains are relevant for ${userType}]

${functionalChecklist}

## Scenario File Format

Each scenario file should follow this structure:

\`\`\`yaml
---
epic: ${epic}
user_type: ${userType}
governance_layer: [layer_name]
scene: [specific_scene_name]
related_users: [list_of_related_users]
related_layers: [list_of_related_layers]
interacts_with: [list_of_elohim_agents]
---

# [Layer Name] Scenario - ${userDisplay}

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
\`\`\`

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

[**TODO**: Add any special considerations for ${userType} scenarios in ${epic} epic]

- Special requirement 1
- Special requirement 2
- Edge cases to consider
`;
}

/**
 * Scaffold result
 */
export interface ScaffoldResult {
  created: string[];
  skipped: string[];
  errors: string[];
}

/**
 * Scaffold templates for a specific user type
 */
export function scaffoldUserType(
  basePath: string,
  epic: string,
  userType: string
): ScaffoldResult {
  const result: ScaffoldResult = {
    created: [],
    skipped: [],
    errors: []
  };

  const epicConfig = EPICS[epic];
  if (!epicConfig) {
    result.errors.push(`Unknown epic: ${epic}`);
    return result;
  }

  const userPath = path.join(basePath, epic, userType);

  // Ensure directory exists
  if (!fs.existsSync(userPath)) {
    fs.mkdirSync(userPath, { recursive: true });
  }

  // Generate README.md
  const readmePath = path.join(userPath, 'README.md');
  if (!fs.existsSync(readmePath)) {
    const readme = generateReadme(epic, userType, epicConfig.description);
    fs.writeFileSync(readmePath, readme, 'utf-8');
    result.created.push(readmePath);
  } else {
    result.skipped.push(readmePath);
  }

  // Generate TODO.md
  const todoPath = path.join(userPath, 'TODO.md');
  if (!fs.existsSync(todoPath)) {
    const todo = generateTodo(epic, userType);
    fs.writeFileSync(todoPath, todo, 'utf-8');
    result.created.push(todoPath);
  } else {
    result.skipped.push(todoPath);
  }

  return result;
}

/**
 * Scaffold templates for an entire epic
 */
export function scaffoldEpic(basePath: string, epic: string): ScaffoldResult {
  const result: ScaffoldResult = {
    created: [],
    skipped: [],
    errors: []
  };

  const epicConfig = EPICS[epic];
  if (!epicConfig) {
    result.errors.push(`Unknown epic: ${epic}`);
    return result;
  }

  for (const userType of epicConfig.users) {
    const userResult = scaffoldUserType(basePath, epic, userType);
    result.created.push(...userResult.created);
    result.skipped.push(...userResult.skipped);
    result.errors.push(...userResult.errors);
  }

  return result;
}

/**
 * Scaffold templates for all epics and user types
 */
export function scaffoldAll(basePath: string): ScaffoldResult {
  const result: ScaffoldResult = {
    created: [],
    skipped: [],
    errors: []
  };

  for (const epic of Object.keys(EPICS)) {
    const epicResult = scaffoldEpic(basePath, epic);
    result.created.push(...epicResult.created);
    result.skipped.push(...epicResult.skipped);
    result.errors.push(...epicResult.errors);
  }

  return result;
}

/**
 * List all epics and their user types
 */
export function listEpicsAndUsers(): Array<{ epic: string; description: string; users: string[] }> {
  return Object.entries(EPICS).map(([epic, config]) => ({
    epic,
    description: config.description,
    users: config.users
  }));
}

/**
 * Get total user type count
 */
export function getTotalUserCount(): number {
  return Object.values(EPICS).reduce((sum, config) => sum + config.users.length, 0);
}
