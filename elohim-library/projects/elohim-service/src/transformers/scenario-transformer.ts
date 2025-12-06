/**
 * Scenario Transformer
 *
 * Transforms .feature files (Gherkin/BDD) into 'scenario' ContentNodes.
 * Scenarios define behavioral specifications and user journeys.
 *
 * Source: data/content/elohim-protocol/governance/policy_maker/scenarios/*.feature
 * Output: ContentNode with contentType: 'scenario'
 */

import { ContentNode } from '../models/content-node.model';
import { ParsedContent, ParsedScenario } from '../models/import-context.model';
import { extractGherkinTags, extractGherkinDescription } from '../parsers/gherkin-parser';

/**
 * Transform Gherkin feature into scenario ContentNodes
 * Returns an array because one .feature file may contain multiple scenarios
 */
export function transformScenarios(
  parsed: ParsedContent,
  sourceNodeId?: string
): ContentNode[] {
  const nodes: ContentNode[] = [];
  const now = new Date().toISOString();

  // Create a feature-level node
  const featureNode = createFeatureNode(parsed, sourceNodeId, now);
  nodes.push(featureNode);

  // Create individual scenario nodes
  if (parsed.scenarios && parsed.scenarios.length > 0) {
    for (const scenario of parsed.scenarios) {
      const scenarioNode = createScenarioNode(
        parsed,
        scenario,
        featureNode.id,
        sourceNodeId,
        now
      );
      nodes.push(scenarioNode);
    }
  }

  return nodes;
}

/**
 * Create the feature-level ContentNode
 */
function createFeatureNode(
  parsed: ParsedContent,
  sourceNodeId: string | undefined,
  timestamp: string
): ContentNode {
  const id = generateFeatureId(parsed);
  const tags = extractGherkinTags(parsed);
  tags.push('feature');

  const description = extractGherkinDescription(parsed);

  const metadata: Record<string, unknown> = {
    category: 'feature',
    epic: parsed.pathMeta.epic,
    userType: parsed.pathMeta.userType,
    scenarioCount: parsed.scenarios?.length || 0,
    source: 'elohim-import',
    sourceVersion: '1.0.0'
  };

  if (sourceNodeId) {
    metadata.derivedFrom = sourceNodeId;
    metadata.extractionMethod = 'gherkin-parse';
  }

  // Add frontmatter fields from Gherkin tags
  if (parsed.frontmatter.priority) {
    metadata.priority = parsed.frontmatter.priority;
  }
  if (parsed.frontmatter.status) {
    metadata.status = parsed.frontmatter.status;
  }

  const relatedNodeIds: string[] = [];

  // Add source node
  if (sourceNodeId) {
    relatedNodeIds.push(sourceNodeId);
  }

  // Add epic node
  const epicNodeId = `epic-${parsed.pathMeta.epic}`;
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    relatedNodeIds.push(epicNodeId);
  }

  // Add archetype node
  if (parsed.pathMeta.userType) {
    const archetypeNodeId = `role-${parsed.pathMeta.epic}-${parsed.pathMeta.userType}`;
    relatedNodeIds.push(archetypeNodeId);
  }

  return {
    id,
    contentType: 'scenario',
    title: parsed.title,
    description,
    content: parsed.rawContent,
    contentFormat: 'gherkin',
    tags,
    sourcePath: parsed.pathMeta.fullPath,
    relatedNodeIds,
    metadata,
    reach: 'commons',
    createdAt: timestamp,
    updatedAt: timestamp
  };
}

/**
 * Create a ContentNode for an individual scenario
 */
function createScenarioNode(
  parsed: ParsedContent,
  scenario: ParsedScenario,
  featureNodeId: string,
  sourceNodeId: string | undefined,
  timestamp: string
): ContentNode {
  const id = generateScenarioId(parsed, scenario);

  // Combine feature tags with scenario-specific tags
  const tags = extractGherkinTags(parsed);
  for (const tag of scenario.tags) {
    const normalizedTag = tag.toLowerCase().replace(/:/g, '-');
    if (!tags.includes(normalizedTag)) {
      tags.push(normalizedTag);
    }
  }

  // Format steps as content
  const stepsContent = formatSteps(scenario.steps);

  const metadata: Record<string, unknown> = {
    category: 'scenario',
    scenarioType: scenario.type,
    epic: parsed.pathMeta.epic,
    userType: parsed.pathMeta.userType,
    featureId: featureNodeId,
    stepCount: scenario.steps.length,
    source: 'elohim-import',
    sourceVersion: '1.0.0'
  };

  if (sourceNodeId) {
    metadata.derivedFrom = sourceNodeId;
    metadata.extractionMethod = 'gherkin-parse';
  }

  const relatedNodeIds: string[] = [featureNodeId];

  if (sourceNodeId) {
    relatedNodeIds.push(sourceNodeId);
  }

  return {
    id,
    contentType: 'scenario',
    title: scenario.title,
    description: generateScenarioDescription(scenario),
    content: stepsContent,
    contentFormat: 'gherkin',
    tags,
    sourcePath: parsed.pathMeta.fullPath,
    relatedNodeIds,
    metadata,
    reach: 'commons',
    createdAt: timestamp,
    updatedAt: timestamp
  };
}

/**
 * Generate feature node ID
 */
function generateFeatureId(parsed: ParsedContent): string {
  const parts = ['feature'];

  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    parts.push(parsed.pathMeta.epic);
  }

  if (parsed.pathMeta.userType) {
    parts.push(parsed.pathMeta.userType);
  }

  // Add base name
  const baseName = parsed.pathMeta.baseName.toLowerCase();
  parts.push(baseName);

  return parts
    .map(p => p.toLowerCase().replace(/[^a-z0-9]/g, '-'))
    .join('-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Generate scenario node ID
 */
function generateScenarioId(parsed: ParsedContent, scenario: ParsedScenario): string {
  const parts = ['scenario'];

  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    parts.push(parsed.pathMeta.epic);
  }

  if (parsed.pathMeta.userType) {
    parts.push(parsed.pathMeta.userType);
  }

  // Add sanitized scenario title
  const titleSlug = scenario.title
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, '')
    .replace(/\s+/g, '-')
    .slice(0, 50);

  parts.push(titleSlug);

  return parts
    .map(p => p.toLowerCase().replace(/[^a-z0-9-]/g, '-'))
    .join('-')
    .replace(/-+/g, '-')
    .replace(/^-|-$/g, '');
}

/**
 * Format scenario steps as readable content
 */
function formatSteps(steps: Array<{ keyword: string; text: string }>): string {
  return steps
    .map(step => `${step.keyword} ${step.text}`)
    .join('\n');
}

/**
 * Generate description from scenario
 */
function generateScenarioDescription(scenario: ParsedScenario): string {
  const stepSummary = scenario.steps.slice(0, 3).map(s => s.text).join('; ');

  if (scenario.steps.length > 3) {
    return `${stepSummary}... (${scenario.steps.length} steps total)`;
  }

  return stepSummary || `Scenario: ${scenario.title}`;
}

/**
 * Check if content should be transformed as scenario
 */
export function isScenarioContent(parsed: ParsedContent): boolean {
  return parsed.pathMeta.isScenario;
}

/**
 * Transform a single feature file, returning all resulting nodes
 */
export function transformFeatureFile(
  parsed: ParsedContent,
  sourceNodeId?: string
): ContentNode[] {
  return transformScenarios(parsed, sourceNodeId);
}
