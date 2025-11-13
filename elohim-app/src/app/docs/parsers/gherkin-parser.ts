import { FeatureNode, ScenarioNode, GherkinStep, GherkinBackground, ScenarioExamples } from '../models';
import { NodeType } from '../models';

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
    let currentLine = 0;

    // Extract tags
    const tags = this.extractTags(lines[currentLine]);
    if (tags.length > 0) currentLine++;

    // Extract feature header
    const featureMatch = lines[currentLine]?.match(/^Feature:\s*(.+)$/);
    if (!featureMatch) {
      throw new Error(`Invalid feature file: ${sourcePath}`);
    }

    const featureTitle = featureMatch[1].trim();
    currentLine++;

    // Extract feature description (lines until first scenario/background)
    const descriptionLines: string[] = [];
    while (
      currentLine < lines.length &&
      !lines[currentLine].match(/^\s*(Scenario|Background|Scenario Outline|@)/)
    ) {
      const line = lines[currentLine].trim();
      if (line) descriptionLines.push(line);
      currentLine++;
    }

    const featureId = this.generateId(sourcePath, 'feature');

    // Parse background if present
    let background: GherkinBackground | undefined;
    if (lines[currentLine]?.match(/^\s*Background:/)) {
      currentLine++;
      const bgSteps: GherkinStep[] = [];
      while (
        currentLine < lines.length &&
        !lines[currentLine].match(/^\s*(Scenario|Scenario Outline|@)/)
      ) {
        const step = this.parseStep(lines[currentLine]);
        if (step) bgSteps.push(step);
        currentLine++;
      }
      background = { steps: bgSteps };
    }

    // Parse scenarios
    const scenarios: ScenarioNode[] = [];
    while (currentLine < lines.length) {
      // Check for tags before scenario
      const scenarioTags = this.extractTags(lines[currentLine]);
      if (scenarioTags.length > 0) currentLine++;

      const scenarioMatch = lines[currentLine]?.match(/^\s*(Scenario|Scenario Outline):\s*(.+)$/);
      if (!scenarioMatch) {
        currentLine++;
        continue;
      }

      const scenarioType = scenarioMatch[1] === 'Scenario Outline' ? 'scenario_outline' : 'scenario';
      const scenarioTitle = scenarioMatch[2].trim();
      currentLine++;

      // Parse scenario steps
      const steps: GherkinStep[] = [];
      while (
        currentLine < lines.length &&
        !lines[currentLine].match(/^\s*(Scenario|Scenario Outline|Examples|@)/)
      ) {
        const step = this.parseStep(lines[currentLine]);
        if (step) steps.push(step);
        currentLine++;
      }

      // Parse examples for scenario outlines
      let examples: ScenarioExamples[] | undefined;
      if (scenarioType === 'scenario_outline' && lines[currentLine]?.match(/^\s*Examples:/)) {
        examples = this.parseExamples(lines, currentLine);
        while (currentLine < lines.length && !lines[currentLine].match(/^\s*(Scenario|@)/)) {
          currentLine++;
        }
      }

      const scenarioId = this.generateId(sourcePath, 'scenario', scenarioTitle);
      const epicIds = this.extractEpicIds(scenarioTags.concat(tags));

      scenarios.push({
        id: scenarioId,
        type: NodeType.SCENARIO,
        title: scenarioTitle,
        description: scenarioTitle,
        tags: scenarioTags.concat(tags),
        sourcePath,
        content: scenarioTitle,
        relatedNodeIds: [featureId, ...epicIds],
        metadata: {},
        featureId,
        epicIds,
        scenarioType: scenarioType as 'scenario' | 'scenario_outline',
        steps,
        examples
      });
    }

    const epicIds = this.extractEpicIds(tags);

    const feature: FeatureNode = {
      id: featureId,
      type: NodeType.FEATURE,
      title: featureTitle,
      description: descriptionLines.join(' '),
      tags,
      sourcePath,
      content: content,
      relatedNodeIds: [...scenarios.map(s => s.id), ...epicIds],
      metadata: {},
      category,
      epicIds,
      scenarioIds: scenarios.map(s => s.id),
      featureDescription: descriptionLines.join('\n'),
      background,
      gherkinContent: content
    };

    return { feature, scenarios };
  }

  private static parseStep(line: string): GherkinStep | null {
    const stepMatch = line.match(/^\s*(Given|When|Then|And|But)\s+(.+)$/);
    if (!stepMatch) return null;

    return {
      keyword: stepMatch[1],
      text: stepMatch[2].trim()
    };
  }

  private static parseExamples(lines: string[], startLine: number): ScenarioExamples[] {
    const examples: ScenarioExamples = {
      headers: [],
      rows: []
    };

    let currentLine = startLine + 1;

    // Parse header row
    const headerMatch = lines[currentLine]?.match(/^\s*\|(.+)\|/);
    if (headerMatch) {
      examples.headers = headerMatch[1].split('|').map(h => h.trim());
      currentLine++;
    }

    // Parse data rows
    while (currentLine < lines.length) {
      const rowMatch = lines[currentLine]?.match(/^\s*\|(.+)\|/);
      if (!rowMatch) break;

      examples.rows.push(rowMatch[1].split('|').map(c => c.trim()));
      currentLine++;
    }

    return [examples];
  }

  private static extractTags(line: string): string[] {
    const tagMatch = line?.match(/^\s*(@[\w-]+(?:\s+@[\w-]+)*)/);
    if (!tagMatch) return [];

    return tagMatch[1]
      .split(/\s+/)
      .filter(t => t.startsWith('@'))
      .map(t => t.substring(1));
  }

  private static extractEpicIds(tags: string[]): string[] {
    return tags
      .filter(tag => tag.startsWith('epic:'))
      .map(tag => tag.substring(5));
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
        .replace(/^_|_$/g, '');
      return `${type}_${pathPart}_${titlePart}`;
    }

    return `${type}_${pathPart}`;
  }
}
