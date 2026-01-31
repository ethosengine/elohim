import { ContentNode } from '../models/content-node.model';

// @coverage: 97.8% (2026-02-04)

/**
 * Gherkin step (Given/When/Then/And/But)
 */
export interface GherkinStep {
  keyword: string;
  text: string;
  docString?: string;
  dataTable?: string[][];
}

/**
 * Background section
 */
export interface GherkinBackground {
  steps: GherkinStep[];
}

/**
 * Examples table for Scenario Outline
 */
export interface ScenarioExamples {
  headers: string[];
  rows: string[][];
}

/**
 * Parsed Feature (extends ContentNode)
 */
export interface FeatureNode extends ContentNode {
  category: string;
  epicIds: string[];
  scenarioIds: string[];
  featureDescription: string;
  background?: GherkinBackground;
  gherkinContent: string;
}

/**
 * Parsed Scenario (extends ContentNode)
 */
export interface ScenarioNode extends ContentNode {
  featureId: string;
  epicIds: string[];
  scenarioType: 'scenario' | 'scenario_outline';
  steps: GherkinStep[];
  examples?: ScenarioExamples[];
}

/**
 * Parser for Gherkin .feature files
 * Extracts features and scenarios into graph nodes
 */
export class GherkinParser {
  /**
   * Parse a .feature file content into FeatureNode and ScenarioNodes
   */
  static parseFeature(
    content: string,
    sourcePath: string,
    category: string
  ): { feature: FeatureNode; scenarios: ScenarioNode[] } {
    const lines = content.split('\n');
    const parseContext = { currentLine: 0 };

    const { tags, featureTitle, descriptionLines } = this.parseFeatureHeader(
      lines,
      parseContext,
      sourcePath
    );
    const featureId = this.generateId(sourcePath, 'feature');
    const background = this.parseBackground(lines, parseContext);
    const scenarios = this.parseScenarios(lines, parseContext, sourcePath, featureId, tags);
    const epicIds = this.extractEpicIds(tags);

    const feature: FeatureNode = {
      id: featureId,
      contentType: 'feature',
      title: featureTitle,
      description: descriptionLines.join(' '),
      tags,
      sourcePath,
      content: content,
      contentFormat: 'gherkin',
      relatedNodeIds: [...scenarios.map(s => s.id), ...epicIds],
      metadata: {},
      category,
      epicIds,
      scenarioIds: scenarios.map(s => s.id),
      featureDescription: descriptionLines.join('\n'),
      background,
      gherkinContent: content,
    };

    return { feature, scenarios };
  }

  private static parseFeatureHeader(
    lines: string[],
    parseContext: { currentLine: number },
    sourcePath: string
  ): { tags: string[]; featureTitle: string; descriptionLines: string[] } {
    const tags = this.extractTags(lines[parseContext.currentLine]);
    if (tags.length > 0) parseContext.currentLine++;

    // eslint-disable-next-line sonarjs/slow-regex -- Safe: parsing structured Gherkin content
    const featureMatch = /^Feature:\s*(.+)$/.exec(lines[parseContext.currentLine] ?? '');
    if (!featureMatch) {
      throw new Error(`Invalid feature file: ${sourcePath}`);
    }

    const featureTitle = featureMatch[1].trim();
    parseContext.currentLine++;

    const descriptionLines: string[] = [];
    while (
      parseContext.currentLine < lines.length &&
      !/^\s*(Scenario|Background|Scenario Outline|@)/.exec(lines[parseContext.currentLine])
    ) {
      const line = lines[parseContext.currentLine].trim();
      if (line) descriptionLines.push(line);
      parseContext.currentLine++;
    }

    return { tags, featureTitle, descriptionLines };
  }

  private static parseBackground(
    lines: string[],
    parseContext: { currentLine: number }
  ): GherkinBackground | undefined {
    if (!/^\s*Background:/.exec(lines[parseContext.currentLine] ?? '')) {
      return undefined;
    }

    parseContext.currentLine++;
    const bgSteps: GherkinStep[] = [];
    while (
      parseContext.currentLine < lines.length &&
      !/^\s*(Scenario|Scenario Outline|@)/.exec(lines[parseContext.currentLine])
    ) {
      const step = this.parseStep(lines[parseContext.currentLine]);
      if (step) bgSteps.push(step);
      parseContext.currentLine++;
    }
    return { steps: bgSteps };
  }

  private static parseScenarios(
    lines: string[],
    parseContext: { currentLine: number },
    sourcePath: string,
    featureId: string,
    featureTags: string[]
  ): ScenarioNode[] {
    const scenarios: ScenarioNode[] = [];

    while (parseContext.currentLine < lines.length) {
      const scenario = this.parseSingleScenario(
        lines,
        parseContext,
        sourcePath,
        featureId,
        featureTags
      );
      if (scenario) {
        scenarios.push(scenario);
      }
    }

    return scenarios;
  }

  private static parseSingleScenario(
    lines: string[],
    parseContext: { currentLine: number },
    sourcePath: string,
    featureId: string,
    featureTags: string[]
  ): ScenarioNode | null {
    const scenarioTags = this.extractTags(lines[parseContext.currentLine]);
    if (scenarioTags.length > 0) parseContext.currentLine++;

    // eslint-disable-next-line sonarjs/slow-regex -- Safe: parsing structured Gherkin content
    const scenarioMatch = /^\s*(Scenario|Scenario Outline):\s*(.+)$/.exec(
      lines[parseContext.currentLine] ?? ''
    );
    if (!scenarioMatch) {
      parseContext.currentLine++;
      return null;
    }

    const scenarioType = scenarioMatch[1] === 'Scenario Outline' ? 'scenario_outline' : 'scenario';
    const scenarioTitle = scenarioMatch[2].trim();
    parseContext.currentLine++;

    const steps = this.parseScenarioSteps(lines, parseContext);
    const examples = this.parseScenarioExamples(lines, parseContext, scenarioType);

    const scenarioId = this.generateId(sourcePath, 'scenario', scenarioTitle);
    const epicIds = this.extractEpicIds(scenarioTags.concat(featureTags));

    return {
      id: scenarioId,
      contentType: 'scenario',
      title: scenarioTitle,
      description: scenarioTitle,
      tags: scenarioTags.concat(featureTags),
      sourcePath,
      content: scenarioTitle,
      contentFormat: 'gherkin',
      relatedNodeIds: [featureId, ...epicIds],
      metadata: {},
      featureId,
      epicIds,
      scenarioType,
      steps,
      examples,
    };
  }

  private static parseScenarioSteps(
    lines: string[],
    parseContext: { currentLine: number }
  ): GherkinStep[] {
    const steps: GherkinStep[] = [];
    while (
      parseContext.currentLine < lines.length &&
      !/^\s*(Scenario|Scenario Outline|Examples|@)/.exec(lines[parseContext.currentLine])
    ) {
      const step = this.parseStep(lines[parseContext.currentLine]);
      if (step) steps.push(step);
      parseContext.currentLine++;
    }
    return steps;
  }

  private static parseScenarioExamples(
    lines: string[],
    parseContext: { currentLine: number },
    scenarioType: string
  ): ScenarioExamples[] | undefined {
    if (
      scenarioType !== 'scenario_outline' ||
      !/^\s*Examples:/.exec(lines[parseContext.currentLine] ?? '')
    ) {
      return undefined;
    }

    const examples = this.parseExamples(lines, parseContext.currentLine);
    while (
      parseContext.currentLine < lines.length &&
      !/^\s*(Scenario|@)/.exec(lines[parseContext.currentLine])
    ) {
      parseContext.currentLine++;
    }
    return examples;
  }

  private static parseStep(line: string): GherkinStep | null {
    // eslint-disable-next-line sonarjs/slow-regex -- Safe: parsing structured Gherkin content
    const stepMatch = /^\s*(Given|When|Then|And|But)\s+(.+)$/.exec(line);
    if (!stepMatch) return null;

    return {
      keyword: stepMatch[1],
      text: stepMatch[2].trim(),
    };
  }

  private static parseExamples(lines: string[], startLine: number): ScenarioExamples[] {
    const examples: ScenarioExamples = {
      headers: [],
      rows: [],
    };

    let currentLine = startLine + 1;

    // Parse header row
    const headerMatch = /^\s*\|(.+)\|/.exec(lines[currentLine] ?? '');
    if (headerMatch) {
      examples.headers = headerMatch[1].split('|').map(h => h.trim());
      currentLine++;
    }

    // Parse data rows
    while (currentLine < lines.length) {
      const rowMatch = /^\s*\|(.+)\|/.exec(lines[currentLine] ?? '');
      if (!rowMatch) break;

      examples.rows.push(rowMatch[1].split('|').map(c => c.trim()));
      currentLine++;
    }

    return [examples];
  }

  private static extractTags(line: string): string[] {
    const tagMatch = /^\s*(@[\w:-]+(?:\s+@[\w:-]+)*)/.exec(line ?? '');
    if (!tagMatch) return [];

    return tagMatch[1]
      .split(/\s+/)
      .filter(t => t.startsWith('@'))
      .map(t => t.substring(1));
  }

  private static extractEpicIds(tags: string[]): string[] {
    return tags.filter(tag => tag.startsWith('epic:')).map(tag => tag.substring(5));
  }

  private static generateId(sourcePath: string, type: string, title?: string): string {
    const pathPart = sourcePath
      .split('/')
      .slice(-2)
      .join('_')
      .replace(/\.feature$/, '');

    if (title) {
      const titlePart = title
        .toLowerCase()
        .replace(/[^a-z0-9]+/g, '_')
        .replace(/(^_)|(_$)/g, '');
      return `${type}_${pathPart}_${titlePart}`;
    }

    return `${type}_${pathPart}`;
  }
}
