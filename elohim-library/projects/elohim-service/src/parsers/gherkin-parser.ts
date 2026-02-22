/**
 * Gherkin Parser
 *
 * Parses .feature files (Gherkin/BDD format) into structured content.
 * Extracts features, scenarios, steps, and tags for transformation into ContentNodes.
 */

import * as crypto from 'crypto';

import { ParsedContent, ParsedScenario } from '../models/import-context.model';
import { PathMetadata } from '../models/path-metadata.model';

/**
 * Gherkin tag with name and optional value
 */
export interface GherkinTag {
  name: string;
  value?: string;
}

/**
 * Parsed Gherkin feature
 */
export interface ParsedFeature {
  title: string;
  description: string;
  tags: GherkinTag[];
  background?: {
    steps: { keyword: string; text: string }[];
  };
  scenarios: ParsedScenario[];
}

/**
 * Parse a Gherkin .feature file
 */
export function parseGherkin(content: string, pathMeta: PathMetadata): ParsedContent {
  const lines = content.split('\n');
  const feature = parseFeature(lines);

  // Calculate content hash
  const contentHash = crypto.createHash('sha256').update(content).digest('hex');

  // Convert tags to frontmatter-style object
  const frontmatter = tagsToFrontmatter(feature.tags);

  return {
    pathMeta,
    frontmatter,
    rawContent: content,
    scenarios: feature.scenarios,
    title: feature.title,
    contentHash,
  };
}

/**
 * Parse feature from lines
 */
function parseFeature(lines: string[]): ParsedFeature {
  let lineIndex = 0;
  const tags: GherkinTag[] = [];
  let title = '';
  const descriptionLines: string[] = [];
  let background: { steps: { keyword: string; text: string }[] } | undefined;
  const scenarios: ParsedScenario[] = [];

  // Skip empty lines and collect tags
  while (lineIndex < lines.length) {
    const line = lines[lineIndex].trim();

    if (line === '') {
      lineIndex++;
      continue;
    }

    if (line.startsWith('@')) {
      // Parse tags
      const lineTags = parseTags(line);
      tags.push(...lineTags);
      lineIndex++;
      continue;
    }

    break;
  }

  // Parse Feature line
  const featureMatch = /^Feature:\s*(.+)$/.exec(lines[lineIndex]?.trim() || '');
  if (featureMatch) {
    title = featureMatch[1].trim();
    lineIndex++;

    // Collect description until Background or Scenario
    while (lineIndex < lines.length) {
      const line = lines[lineIndex].trim();
      if (
        line.startsWith('Background:') ||
        line.startsWith('Scenario:') ||
        line.startsWith('Scenario Outline:') ||
        line.startsWith('@')
      ) {
        break;
      }
      if (line) {
        descriptionLines.push(line);
      }
      lineIndex++;
    }
  }

  // Parse Background if present
  if (lines[lineIndex]?.trim().startsWith('Background:')) {
    lineIndex++;
    const bgSteps: { keyword: string; text: string }[] = [];

    while (lineIndex < lines.length) {
      const line = lines[lineIndex].trim();
      if (
        line.startsWith('Scenario:') ||
        line.startsWith('Scenario Outline:') ||
        line.startsWith('@')
      ) {
        break;
      }

      const step = parseStep(line);
      if (step) {
        bgSteps.push(step);
      }
      lineIndex++;
    }

    if (bgSteps.length > 0) {
      background = { steps: bgSteps };
    }
  }

  // Parse Scenarios
  while (lineIndex < lines.length) {
    const scenario = parseScenario(lines, lineIndex);
    if (scenario) {
      scenarios.push(scenario.scenario);
      lineIndex = scenario.nextIndex;
    } else {
      lineIndex++;
    }
  }

  return {
    title,
    description: descriptionLines.join('\n'),
    tags,
    background,
    scenarios,
  };
}

/**
 * Parse a scenario starting at lineIndex
 */
function parseScenario(
  lines: string[],
  startIndex: number
): { scenario: ParsedScenario; nextIndex: number } | null {
  let lineIndex = startIndex;
  const scenarioTags: string[] = [];

  // Skip empty lines and collect scenario-level tags
  while (lineIndex < lines.length) {
    const line = lines[lineIndex].trim();

    if (line === '') {
      lineIndex++;
      continue;
    }

    if (line.startsWith('@')) {
      const tags = parseTags(line);
      scenarioTags.push(...tags.map(t => (t.value ? `${t.name}:${t.value}` : t.name)));
      lineIndex++;
      continue;
    }

    break;
  }

  // Check for Scenario or Scenario Outline
  const scenarioMatch = /^(Scenario|Scenario Outline):\s*(.+)$/.exec(
    lines[lineIndex]?.trim() || ''
  );
  if (!scenarioMatch) {
    return null;
  }

  const type = scenarioMatch[1] === 'Scenario Outline' ? 'scenario_outline' : 'scenario';
  const title = scenarioMatch[2].trim();
  lineIndex++;

  // Parse steps
  const steps: { keyword: string; text: string }[] = [];

  while (lineIndex < lines.length) {
    const line = lines[lineIndex].trim();

    // Stop at next scenario, examples, or tags (next scenario coming)
    if (
      line.startsWith('Scenario:') ||
      line.startsWith('Scenario Outline:') ||
      line.startsWith('Examples:') ||
      (line.startsWith('@') && !line.includes(' '))
    ) {
      break;
    }

    const step = parseStep(line);
    if (step) {
      steps.push(step);
    }
    lineIndex++;
  }

  // Skip Examples section for Scenario Outline
  if (type === 'scenario_outline' && lines[lineIndex]?.trim().startsWith('Examples:')) {
    while (lineIndex < lines.length) {
      const line = lines[lineIndex].trim();
      if (
        line.startsWith('Scenario:') ||
        line.startsWith('Scenario Outline:') ||
        line.startsWith('@')
      ) {
        break;
      }
      lineIndex++;
    }
  }

  return {
    scenario: {
      title,
      type,
      tags: scenarioTags,
      steps,
    },
    nextIndex: lineIndex,
  };
}

/**
 * Parse a step line
 */
function parseStep(line: string): { keyword: string; text: string } | null {
  const stepMatch = /^\s*(Given|When|Then|And|But)\s+(.+)$/.exec(line);
  if (!stepMatch) {
    return null;
  }

  return {
    keyword: stepMatch[1],
    text: stepMatch[2].trim(),
  };
}

/**
 * Parse tags from a line
 */
function parseTags(line: string): GherkinTag[] {
  const tags: GherkinTag[] = [];
  const tagMatches = line.matchAll(/@([\w-]+)(?::([\w-]+))?/g);

  for (const match of tagMatches) {
    tags.push({
      name: match[1],
      value: match[2],
    });
  }

  return tags;
}

/**
 * Convert Gherkin tags to frontmatter-style object
 */
function tagsToFrontmatter(tags: GherkinTag[]): Record<string, unknown> {
  const frontmatter: Record<string, unknown> = {};
  const simpleTags: string[] = [];

  for (const tag of tags) {
    if (tag.value) {
      // Key-value tag (e.g., @epic:governance)
      frontmatter[tag.name] = tag.value;
    } else {
      // Simple tag
      simpleTags.push(tag.name);
    }
  }

  if (simpleTags.length > 0) {
    frontmatter.tags = simpleTags;
  }

  return frontmatter;
}

/**
 * Extract description from Gherkin feature
 */
export function extractGherkinDescription(parsed: ParsedContent): string {
  // Use feature description if available
  if (parsed.frontmatter.description && typeof parsed.frontmatter.description === 'string') {
    return parsed.frontmatter.description;
  }

  // Generate from scenarios
  if (parsed.scenarios && parsed.scenarios.length > 0) {
    const scenarioCount = parsed.scenarios.length;
    const scenarioTitles = parsed.scenarios.slice(0, 3).map(s => s.title);

    if (scenarioCount <= 3) {
      return `Scenarios: ${scenarioTitles.join(', ')}`;
    }

    return `${scenarioCount} scenarios including: ${scenarioTitles.join(', ')}...`;
  }

  return `Feature: ${parsed.title}`;
}

/**
 * Extract tags from Gherkin content
 */
export function extractGherkinTags(parsed: ParsedContent): string[] {
  const tags = new Set<string>();

  // From frontmatter (converted from Gherkin tags)
  if (Array.isArray(parsed.frontmatter.tags)) {
    for (const tag of parsed.frontmatter.tags) {
      if (typeof tag === 'string') {
        tags.add(tag.toLowerCase());
      }
    }
  }

  // Key-value tags
  for (const [key, value] of Object.entries(parsed.frontmatter)) {
    if (key !== 'tags' && typeof value === 'string') {
      tags.add(key.toLowerCase());
      tags.add(value.toLowerCase());
    }
  }

  // From path metadata
  if (parsed.pathMeta.epic && parsed.pathMeta.epic !== 'other') {
    tags.add(parsed.pathMeta.epic.toLowerCase());
  }

  if (parsed.pathMeta.userType) {
    tags.add(parsed.pathMeta.userType.toLowerCase().replace(/_/g, '-'));
  }

  // Always add 'scenario' tag
  tags.add('scenario');

  return Array.from(tags);
}
