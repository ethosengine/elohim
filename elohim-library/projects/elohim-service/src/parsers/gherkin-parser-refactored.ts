/**
 * Gherkin Parser (Refactored)
 *
 * Parses .feature files (Gherkin/BDD format) into structured content.
 * ONLY handles parsing structure - extraction of semantic meaning is handled by transformers.
 */

import { PathMetadata } from '../models/path-metadata.model';
import {
  GherkinParserResult,
  GherkinTag,
  ParsedScenario,
  ParsedStep,
  ParserError
} from './parser-result';
import {
  buildParserResult,
  splitLines,
  matchLine
} from './base-parser';

/**
 * Parse a Gherkin .feature file
 */
export function parseGherkin(
  content: string,
  pathMeta: PathMetadata
): GherkinParserResult {
  try {
    const lines = splitLines(content);
    const feature = parseFeature(lines, pathMeta.fullPath);

    // Convert tags to frontmatter-style object
    const frontmatter = tagsToFrontmatter(feature.tags);

    // Build base result
    const baseResult = buildParserResult(
      content,
      pathMeta,
      frontmatter,
      feature.title
    );

    return {
      ...baseResult,
      scenarios: feature.scenarios,
      featureTags: feature.tags
    };
  } catch (error) {
    if (error instanceof ParserError) {
      throw error;
    }
    throw new ParserError(
      `Failed to parse Gherkin: ${(error as Error).message}`,
      pathMeta.fullPath,
      error as Error
    );
  }
}

/**
 * Parsed Gherkin feature
 */
interface ParsedFeature {
  title: string;
  description: string;
  tags: GherkinTag[];
  background?: {
    steps: ParsedStep[];
  };
  scenarios: ParsedScenario[];
}

/**
 * Parse feature from lines
 */
function parseFeature(lines: string[], filePath: string): ParsedFeature {
  let lineIndex = 0;
  const tags: GherkinTag[] = [];
  let title = '';
  const descriptionLines: string[] = [];
  let background: { steps: ParsedStep[] } | undefined;
  const scenarios: ParsedScenario[] = [];

  // Skip empty lines and collect tags
  while (lineIndex < lines.length) {
    const line = lines[lineIndex].trim();

    if (line === '') {
      lineIndex++;
      continue;
    }

    if (line.startsWith('@')) {
      const lineTags = parseTags(line);
      tags.push(...lineTags);
      lineIndex++;
      continue;
    }

    break;
  }

  // Parse Feature line
  const featureMatch = matchLine(lines[lineIndex] || '', /^Feature:\s*(.+)$/);
  if (featureMatch) {
    title = featureMatch[1].trim();
    lineIndex++;

    // Collect description until Background or Scenario
    while (lineIndex < lines.length) {
      const line = lines[lineIndex].trim();
      if (line.startsWith('Background:') ||
          line.startsWith('Scenario:') ||
          line.startsWith('Scenario Outline:') ||
          line.startsWith('@')) {
        break;
      }
      if (line) {
        descriptionLines.push(line);
      }
      lineIndex++;
    }
  } else {
    throw new ParserError('Missing Feature declaration', filePath);
  }

  // Parse Background if present
  if (lines[lineIndex]?.trim().startsWith('Background:')) {
    lineIndex++;
    const bgSteps: ParsedStep[] = [];

    while (lineIndex < lines.length) {
      const line = lines[lineIndex].trim();
      if (line.startsWith('Scenario:') ||
          line.startsWith('Scenario Outline:') ||
          line.startsWith('@')) {
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
    scenarios
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
      scenarioTags.push(...tags.map(t => t.value ? `${t.name}:${t.value}` : t.name));
      lineIndex++;
      continue;
    }

    break;
  }

  // Check for Scenario or Scenario Outline
  const scenarioMatch = matchLine(
    lines[lineIndex] || '',
    /^(Scenario|Scenario Outline):\s*(.+)$/
  );

  if (!scenarioMatch) {
    return null;
  }

  const type = scenarioMatch[1] === 'Scenario Outline' ? 'scenario_outline' : 'scenario';
  const title = scenarioMatch[2].trim();
  lineIndex++;

  // Parse steps
  const steps: ParsedStep[] = [];

  while (lineIndex < lines.length) {
    const line = lines[lineIndex].trim();

    // Stop at next scenario, examples, or tags (next scenario coming)
    if (line.startsWith('Scenario:') ||
        line.startsWith('Scenario Outline:') ||
        line.startsWith('Examples:') ||
        (line.startsWith('@') && !line.includes(' '))) {
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
      if (line.startsWith('Scenario:') ||
          line.startsWith('Scenario Outline:') ||
          line.startsWith('@')) {
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
      steps
    },
    nextIndex: lineIndex
  };
}

/**
 * Parse a step line
 */
function parseStep(line: string): ParsedStep | null {
  const stepMatch = matchLine(line, /^\s*(Given|When|Then|And|But)\s+(.+)$/);
  if (!stepMatch) {
    return null;
  }

  return {
    keyword: stepMatch[1],
    text: stepMatch[2].trim()
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
      value: match[2]
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
