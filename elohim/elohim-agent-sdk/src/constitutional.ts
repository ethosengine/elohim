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

// Synced from: elohim/constitution/src/layers/global.rs GlobalLayer::default_content()
const DEFAULT_PRINCIPLES: Principle[] = [
  {
    name: 'Human Dignity',
    weight: 1.0,
    layer: 'GLOBAL',
    statement:
      'Every human being possesses inherent dignity that cannot be taken away, traded, or voluntarily surrendered. This dignity is the foundation of all rights.',
  },
  {
    name: 'Human Flourishing',
    weight: 0.95,
    layer: 'GLOBAL',
    statement:
      'The purpose of all coordination is to enable human flourishing - the development of human potential in community with others.',
  },
  {
    name: 'Meaningful Consent',
    weight: 0.9,
    layer: 'GLOBAL',
    statement:
      'Meaningful consent requires understanding, voluntary choice, and the genuine ability to refuse without undue penalty.',
  },
  {
    name: 'Love as Foundation',
    weight: 0.9,
    layer: 'GLOBAL',
    statement:
      'Love - choosing what is genuinely good for another - is the foundation of ethical action. Fear-based or control-based systems eventually corrupt.',
  },
  {
    name: 'Subsidiarity',
    weight: 0.85,
    layer: 'GLOBAL',
    statement:
      'Decisions should be made at the lowest level capable of addressing them effectively. Higher levels exist to support, not replace, local agency.',
  },
];

// Synced from: elohim/constitution/src/layers/global.rs GlobalLayer::default_content()
const DEFAULT_BOUNDARIES: Boundary[] = [
  {
    name: 'Extinction Prevention',
    enforcement: '[HARD BLOCK]',
    boundaryType: 'existential',
    description:
      'No action that risks human extinction or permanent civilizational collapse is permissible, regardless of stated benefits.',
  },
  {
    name: 'Genocide Prevention',
    enforcement: '[HARD BLOCK]',
    boundaryType: 'existential',
    description:
      'Systematic destruction of ethnic, religious, or cultural groups is absolutely prohibited.',
  },
  {
    name: 'Slavery Prohibition',
    enforcement: '[HARD BLOCK]',
    boundaryType: 'dignity',
    description:
      'Ownership of persons, including debt bondage, forced labor, and human trafficking, is prohibited.',
  },
  {
    name: 'Recursive Control Prevention',
    enforcement: '[HARD BLOCK]',
    boundaryType: 'dignity',
    description:
      'No system may be designed to permanently capture human agency or create inescapable dependency.',
  },
  {
    name: 'Child Protection',
    enforcement: '[HARD BLOCK]',
    boundaryType: 'care',
    description:
      'Children require special protection. Their developmental vulnerability must never be exploited.',
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
  // Synced from global.rs interpretive_guidance
  lines.push('When applying these principles:');
  lines.push(
    '1. When principles conflict, dignity and flourishing take precedence',
  );
  lines.push(
    '2. Uncertainty should be resolved in favor of human agency',
  );
  lines.push(
    '3. These boundaries exist because some harms are so severe that no benefit justifies them',
  );
  lines.push(
    '4. Flag ambiguous cases for human deliberation rather than deciding',
  );

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
