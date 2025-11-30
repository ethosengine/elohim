import { Injectable } from '@angular/core';
import { ContentIOPlugin, ContentIOImportResult, ContentIOExportInput } from '../../interfaces/content-io-plugin.interface';
import { FormatMetadata } from '../../interfaces/format-metadata.interface';
import { ValidationResult, ValidationError, ValidationWarning } from '../../interfaces/validation-result.interface';
import { GherkinParser, GherkinStep } from '../../../parsers/gherkin-parser';

/**
 * Gherkin I/O Plugin
 *
 * Handles import, export, and validation of Gherkin (.feature) files.
 * Rendering is handled separately by the RendererRegistryService.
 *
 * - Import: Uses GherkinParser to parse BDD feature files
 * - Export: Generates valid Gherkin syntax from ContentNode
 * - Validate: Checks Gherkin syntax and structure
 */
@Injectable({
  providedIn: 'root'
})
export class GherkinIOPlugin implements ContentIOPlugin {
  readonly formatId = 'gherkin';
  readonly displayName = 'Gherkin (BDD)';
  readonly fileExtensions = ['.feature'];
  readonly mimeTypes = ['text/x-gherkin', 'text/plain'];

  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;

  // ─────────────────────────────────────────────────────────────────────────────
  // Import
  // ─────────────────────────────────────────────────────────────────────────────

  async import(input: string | File): Promise<ContentIOImportResult> {
    const content = typeof input === 'string' ? input : await this.readFile(input);
    const sourcePath = typeof input === 'string' ? 'imported.feature' : input.name;

    // Infer category from path or default
    const category = this.inferCategory(sourcePath);

    // Use existing GherkinParser
    const { feature, scenarios } = GherkinParser.parseFeature(content, sourcePath, category);

    // Return feature node info (scenarios are related nodes)
    return {
      content: content,
      contentFormat: 'gherkin',
      contentType: 'feature',
      title: feature.title,
      description: feature.featureDescription || feature.description,
      tags: feature.tags,
      metadata: {
        category: feature.category,
        epicIds: feature.epicIds,
        scenarioCount: scenarios.length,
        scenarioIds: feature.scenarioIds,
        hasBackground: !!feature.background,
        scenarios: scenarios.map(s => ({
          id: s.id,
          title: s.title,
          type: s.scenarioType,
          stepCount: s.steps.length
        }))
      },
      relatedNodeIds: [...feature.scenarioIds, ...feature.epicIds]
    };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Export
  // ─────────────────────────────────────────────────────────────────────────────

  async export(node: ContentIOExportInput): Promise<string> {
    // If content is already valid Gherkin, return it
    if (typeof node.content === 'string' && this.looksLikeGherkin(node.content)) {
      return node.content;
    }

    // Otherwise, generate Gherkin from node data
    return this.generateGherkin(node);
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Validation
  // ─────────────────────────────────────────────────────────────────────────────

  async validate(input: string | File): Promise<ValidationResult> {
    const content = typeof input === 'string' ? input : await this.readFile(input);
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    // Check for Feature: declaration
    const featureMatch = /^Feature:\s*(.+)$/m.exec(content);
    if (!featureMatch) {
      errors.push({
        code: 'NO_FEATURE',
        message: 'No Feature declaration found. Gherkin files must start with "Feature:"',
        line: 1
      });
      return { valid: false, errors, warnings };
    }

    // Check for at least one scenario
    const scenarioCount = (content.match(/^\s*(Scenario|Scenario Outline):/gm) || []).length;
    if (scenarioCount === 0) {
      warnings.push({
        code: 'NO_SCENARIOS',
        message: 'No scenarios found. Feature files should contain at least one Scenario.',
        suggestion: 'Add a Scenario: block after the feature description'
      });
    }

    // Check step structure
    const stepResult = this.validateSteps(content);
    errors.push(...stepResult.errors);
    warnings.push(...stepResult.warnings);

    // Check tag format
    const tagResult = this.validateTags(content);
    warnings.push(...tagResult.warnings);

    // Generate preview
    let parsedPreview: ContentIOImportResult['metadata'] | undefined;
    if (errors.length === 0) {
      try {
        const imported = await this.import(content);
        parsedPreview = {
          title: imported.title,
          description: imported.description,
          contentType: imported.contentType,
          tags: imported.tags
        };
      } catch {
        // Preview generation failed
      }
    }

    // Calculate stats
    const stats = {
      scenarioCount,
      stepCount: (content.match(/^\s*(Given|When|Then|And|But)\s+/gm) || []).length,
      lineCount: content.split('\n').length,
      tagCount: (content.match(/@[\w-]+/g) || []).length
    };

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      parsedPreview,
      stats
    };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Format Detection
  // ─────────────────────────────────────────────────────────────────────────────

  detectFormat(content: string): number | null {
    let confidence = 0;

    // Check for Feature: keyword (strong indicator)
    if (/^Feature:/m.test(content)) {
      confidence += 0.5;
    }

    // Check for Scenario/Scenario Outline
    if (/^\s*(Scenario|Scenario Outline):/m.test(content)) {
      confidence += 0.3;
    }

    // Check for Given/When/Then steps
    if (/^\s*(Given|When|Then|And|But)\s+/m.test(content)) {
      confidence += 0.2;
    }

    // Check for Gherkin tags
    if (/^@[\w-]+/m.test(content)) {
      confidence += 0.1;
    }

    return confidence > 0 ? Math.min(confidence, 1) : null;
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Metadata
  // ─────────────────────────────────────────────────────────────────────────────

  getFormatMetadata(): FormatMetadata {
    return {
      formatId: this.formatId,
      displayName: this.displayName,
      description: 'Behavior-Driven Development (BDD) feature files with Given/When/Then syntax',
      fileExtensions: this.fileExtensions,
      mimeTypes: this.mimeTypes,
      icon: 'checklist',
      category: 'code',
      supportsRoundTrip: true,
      priority: 10
    };
  }

  getDefaultOptions(): Record<string, unknown> {
    return {
      includeBackground: true,
      includeExamples: true
    };
  }

  // ─────────────────────────────────────────────────────────────────────────────
  // Private Helpers
  // ─────────────────────────────────────────────────────────────────────────────

  private async readFile(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(reader.error);
      reader.readAsText(file);
    });
  }

  private inferCategory(sourcePath: string): string {
    const lowerPath = sourcePath.toLowerCase();
    if (lowerPath.includes('governance')) return 'governance';
    if (lowerPath.includes('value') || lowerPath.includes('scanner')) return 'value-scanner';
    if (lowerPath.includes('observer')) return 'public-observer';
    if (lowerPath.includes('autonomous')) return 'autonomous-entity';
    if (lowerPath.includes('social') || lowerPath.includes('medium')) return 'social-medium';
    return 'general';
  }

  private looksLikeGherkin(content: string): boolean {
    return /^Feature:/m.test(content) || /^\s*Scenario:/m.test(content);
  }

  private generateGherkin(node: ContentIOExportInput): string {
    const lines: string[] = [];

    // Tags
    if (node.tags && node.tags.length > 0) {
      lines.push(node.tags.map(t => `@${t}`).join(' '));
    }

    // Feature header
    lines.push(`Feature: ${node.title}`);

    // Description
    if (node.description) {
      lines.push('');
      node.description.split('\n').forEach(line => {
        lines.push(`  ${line.trim()}`);
      });
    }

    // If we have scenario data in metadata, generate them
    const scenarios = (node.metadata?.['scenarios'] as Array<{
      id: string;
      title: string;
      type: string;
      steps?: GherkinStep[];
    }>) || [];

    for (const scenario of scenarios) {
      lines.push('');
      const keyword = scenario.type === 'scenario_outline' ? 'Scenario Outline' : 'Scenario';
      lines.push(`  ${keyword}: ${scenario.title}`);

      if (scenario.steps) {
        for (const step of scenario.steps) {
          lines.push(`    ${step.keyword} ${step.text}`);
        }
      }
    }

    // If no scenarios but content looks like Gherkin, include it
    if (scenarios.length === 0 && typeof node.content === 'string') {
      // Strip feature header if present in content
      const contentLines = node.content.split('\n');
      const startIndex = contentLines.findIndex(line => /^\s*Scenario/.test(line));
      if (startIndex >= 0) {
        lines.push('');
        lines.push(...contentLines.slice(startIndex));
      }
    }

    return lines.join('\n');
  }

  private validateSteps(content: string): { errors: ValidationError[]; warnings: ValidationWarning[] } {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];
    const lines = content.split('\n');

    let inScenario = false;
    let hasGivenInScenario = false;

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineNum = i + 1;

      // Track scenario context
      if (/^\s*(Scenario|Scenario Outline):/.test(line)) {
        if (inScenario && !hasGivenInScenario) {
          warnings.push({
            code: 'NO_GIVEN',
            message: 'Scenario without Given step. Consider adding context.',
            line: lineNum - 1
          });
        }
        inScenario = true;
        hasGivenInScenario = false;
      }

      // Track Given steps
      if (/^\s*Given\s+/.test(line)) {
        hasGivenInScenario = true;
      }

      // Check for invalid step keywords
      const stepMatch = /^\s*(given|when|then|and|but)\s+/i.exec(line);
      if (stepMatch) {
        const keyword = stepMatch[1];
        const properKeyword = keyword.charAt(0).toUpperCase() + keyword.slice(1).toLowerCase();
        if (keyword !== properKeyword && keyword.toLowerCase() !== keyword) {
          warnings.push({
            code: 'KEYWORD_CASE',
            message: `Step keyword "${keyword}" should be "${properKeyword}"`,
            line: lineNum,
            suggestion: `Use "${properKeyword}" instead`
          });
        }
      }
    }

    return { errors, warnings };
  }

  private validateTags(content: string): { warnings: ValidationWarning[] } {
    const warnings: ValidationWarning[] = [];
    const lines = content.split('\n');

    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const lineNum = i + 1;

      // Check for tags on wrong lines
      const tagMatch = /^(\s*)(@[\w-]+)/.exec(line);
      if (tagMatch && tagMatch[1].length > 0 && !line.includes('Feature:') && !line.includes('Scenario')) {
        // Tag is indented but not on a Feature/Scenario line
        const nextLine = lines[i + 1] || '';
        if (!/^\s*(Feature|Scenario|Scenario Outline):/.test(nextLine)) {
          warnings.push({
            code: 'ORPHAN_TAG',
            message: `Tag "${tagMatch[2]}" appears to be orphaned. Tags should precede Feature or Scenario.`,
            line: lineNum
          });
        }
      }
    }

    return { warnings };
  }
}
