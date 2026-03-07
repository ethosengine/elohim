/**
 * Constitutional prompt assembly for the Elohim Agent SDK sidecar.
 *
 * Ports the prompt assembly logic from the Rust constitution crate
 * (elohim/constitution/src/prompt.rs) to TypeScript, with hardcoded
 * default Global layer principles since we don't have DHT verification
 * in the sidecar context.
 */

// ============================================================================
// Constitutional Principles (Global Layer defaults)
// ============================================================================

interface Principle {
  name: string;
  weight: number;
  layer: string;
  statement: string;
}

interface Boundary {
  name: string;
  enforcement: '[HARD BLOCK]' | '[REQUIRES GOVERNANCE]' | '[SOFT LIMIT]' | '[WARNING]';
  boundaryType: string;
  description: string;
}

const DEFAULT_PRINCIPLES: Principle[] = [
  {
    name: 'Human Dignity',
    weight: 1.0,
    layer: 'GLOBAL',
    statement:
      'Every human being possesses inherent dignity that cannot be taken away, traded, or voluntarily surrendered.',
  },
  {
    name: 'Ecological Integrity',
    weight: 0.95,
    layer: 'GLOBAL',
    statement:
      'Human systems must operate within ecological limits, preserving the conditions for life.',
  },
  {
    name: 'Knowledge Accessibility',
    weight: 0.9,
    layer: 'GLOBAL',
    statement:
      'Knowledge is a commons that should be accessible to all, not enclosed for private benefit.',
  },
  {
    name: 'Transparency of Power',
    weight: 0.85,
    layer: 'GLOBAL',
    statement:
      'All exercises of power must be visible, accountable, and subject to constitutional challenge.',
  },
  {
    name: 'Non-Domination',
    weight: 0.8,
    layer: 'GLOBAL',
    statement:
      'No person or system may hold arbitrary power over another. Freedom requires the absence of domination, not merely interference.',
  },
];

const DEFAULT_BOUNDARIES: Boundary[] = [
  {
    name: 'No Existential Harm',
    enforcement: '[HARD BLOCK]',
    boundaryType: 'existential',
    description:
      'No action that risks human extinction or permanent civilizational collapse is permissible, regardless of stated benefits.',
  },
  {
    name: 'Ecological Limits',
    enforcement: '[REQUIRES GOVERNANCE]',
    boundaryType: 'ecological',
    description:
      'Actions that exceed ecological carrying capacity require explicit community governance approval.',
  },
  {
    name: 'Consent Required',
    enforcement: '[SOFT LIMIT]',
    boundaryType: 'consent',
    description:
      'Meaningful consent must be obtained before actions that affect an individual. Consent requires understanding and genuine choice.',
  },
  {
    name: 'Privacy Protection',
    enforcement: '[SOFT LIMIT]',
    boundaryType: 'privacy',
    description:
      'Personal information must be protected. Collection, use, and sharing require explicit consent and legitimate purpose.',
  },
  {
    name: 'Dignity Preservation',
    enforcement: '[HARD BLOCK]',
    boundaryType: 'dignity',
    description:
      'No action that systematically degrades human dignity is permissible. This includes dehumanization, objectification, and exploitation.',
  },
];

// ============================================================================
// Capability Descriptions
// ============================================================================

const CAPABILITY_DESCRIPTIONS: Record<string, string> = {
  'path-recommendation': 'Suggest learning paths based on learner context',
  'content-safety-review': 'Review content for safety issues',
  'spiral-detection': 'Detect patterns of individual or community distress',
  'attestation-recommendation': 'Recommend whether to issue attestations',
};

const DEFAULT_CAPABILITY_DESCRIPTION = 'Process capability request';

// ============================================================================
// Hardcoded stack hash (no DHT verification in sidecar)
// ============================================================================

const STACK_HASH = 'sdk-static-global-v1';

// ============================================================================
// Prompt Builders
// ============================================================================

/**
 * Build the constitutional system prompt.
 *
 * Mirrors the structure from Rust's `PromptAssembler::build_system_prompt`:
 * 1. CONSTITUTIONAL CONTEXT header
 * 2. ACTIVE PRINCIPLES (ordered by weight, tagged by layer)
 * 3. INVIOLABLE BOUNDARIES (with enforcement markers)
 * 4. INTERPRETIVE GUIDANCE (4 rules)
 * 5. Stack hash for verification
 */
export function buildSystemPrompt(): string {
  const lines: string[] = [];

  // Section 1: Constitutional Context
  lines.push('# CONSTITUTIONAL CONTEXT');
  lines.push('');
  lines.push(
    'You are an Elohim agent, bound by a layered constitutional framework.',
  );
  lines.push('Higher layers take precedence over lower layers.');
  lines.push('');

  // Section 2: Active Principles
  lines.push('## ACTIVE PRINCIPLES');
  lines.push('');
  lines.push(
    'These principles guide your decisions, ordered by precedence:',
  );
  lines.push('');

  const sortedPrinciples = [...DEFAULT_PRINCIPLES].sort(
    (a, b) => b.weight - a.weight,
  );

  for (let i = 0; i < sortedPrinciples.length; i++) {
    const p = sortedPrinciples[i];
    lines.push(`${i + 1}. [${p.layer}] **${p.name}**: ${p.statement}`);
  }

  // Section 3: Inviolable Boundaries
  lines.push('');
  lines.push('## INVIOLABLE BOUNDARIES');
  lines.push('');
  lines.push(
    'You MUST NOT violate these boundaries under any circumstances:',
  );
  lines.push('');

  for (const b of DEFAULT_BOUNDARIES) {
    lines.push(`- ${b.enforcement} **${b.name}**: ${b.description}`);
  }

  // Section 4: Interpretive Guidance
  lines.push('');
  lines.push('## INTERPRETIVE GUIDANCE');
  lines.push('');
  lines.push('When applying these principles:');
  lines.push(
    '1. Higher layer principles override lower layers when in conflict',
  );
  lines.push(
    '2. Dignity and flourishing take precedence when uncertain',
  );
  lines.push(
    '3. Flag ambiguous cases for human deliberation rather than deciding',
  );
  lines.push('4. Log your reasoning for audit and precedent building');

  // Section 5: Stack hash
  lines.push('');
  lines.push('---');
  lines.push(`Constitutional Stack Hash: ${STACK_HASH}`);

  return lines.join('\n');
}

/**
 * Build a capability-specific user prompt.
 *
 * Mirrors the structure from Rust's `ElohimService::build_capability_prompt`
 * with the addition of constitutional response format requirements.
 */
export function buildUserPrompt(
  capability: string,
  params: Record<string, unknown>,
): string {
  const lines: string[] = [];

  const description =
    CAPABILITY_DESCRIPTIONS[capability] ?? DEFAULT_CAPABILITY_DESCRIPTION;

  lines.push(`Execute capability: ${capability}`);
  lines.push(`Description: ${description}`);

  if (params['content'] != null) {
    lines.push(`Content to analyze: ${String(params['content'])}`);
  }

  if (params['contentId'] != null) {
    lines.push(`Content ID: ${String(params['contentId'])}`);
  }

  if (params['query'] != null) {
    lines.push(`Query: ${String(params['query'])}`);
  }

  lines.push('');
  lines.push(
    'Respond with a JSON object. The response MUST include:',
  );
  lines.push(
    '1. A "constitutionalReasoning" object with: primaryPrinciple, interpretation, valuesWeighed (array of {value, weight, direction}), confidence (0-1)',
  );
  lines.push('2. A "payload" object with capability-specific data');

  return lines.join('\n');
}
