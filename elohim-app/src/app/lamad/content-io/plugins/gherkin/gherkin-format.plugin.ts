import { Injectable, Type } from '@angular/core';

import { GherkinParser, GherkinStep } from '../../../parsers/gherkin-parser';
import { GherkinRendererComponent } from '../../../renderers/gherkin-renderer/gherkin-renderer.component';
import {
  ContentFormatPlugin,
  ContentRenderer,
  ContentEditorComponent,
  EditorConfig,
  DEFAULT_EDITOR_CONFIG,
} from '../../interfaces/content-format-plugin.interface';
import {
  ContentIOImportResult,
  ContentIOExportInput,
} from '../../interfaces/content-io-plugin.interface';
import { FormatMetadata } from '../../interfaces/format-metadata.interface';
import {
  ValidationResult,
  ValidationError,
  ValidationWarning,
} from '../../interfaces/validation-result.interface';

/**
 * GherkinFormatPlugin - Unified plugin for Gherkin/BDD content.
 *
 * Provides:
 * - Import: Parse .feature files with Given/When/Then steps
 * - Export: Generate valid Gherkin syntax from ContentNode
 * - Validate: Check Gherkin syntax and structure
 * - Render: GherkinRendererComponent with collapsible scenarios, step highlighting
 * - Edit: Uses default code editor (no custom editor yet)
 *
 * This unified plugin replaces the separate GherkinIOPlugin and
 * RendererRegistryService registration.
 */
@Injectable({
  providedIn: 'root',
})
export class GherkinFormatPlugin implements ContentFormatPlugin {
  // ═══════════════════════════════════════════════════════════════════════════
  // Identity
  // ═══════════════════════════════════════════════════════════════════════════

  readonly formatId = 'gherkin';
  readonly displayName = 'Gherkin (BDD)';
  readonly fileExtensions = ['.feature'];
  readonly mimeTypes = ['text/x-gherkin', 'text/plain'];

  // ═══════════════════════════════════════════════════════════════════════════
  // Capabilities
  // ═══════════════════════════════════════════════════════════════════════════

  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;
  readonly canRender = true;
  readonly canEdit = false; // Uses default editor for now

  // ═══════════════════════════════════════════════════════════════════════════
  // Import
  // ═══════════════════════════════════════════════════════════════════════════

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
          stepCount: s.steps.length,
        })),
      },
      relatedNodeIds: [...feature.scenarioIds, ...feature.epicIds],
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export
  // ═══════════════════════════════════════════════════════════════════════════

  async export(node: ContentIOExportInput): Promise<string> {
    // If content is already valid Gherkin, return it
    if (typeof node.content === 'string' && this.looksLikeGherkin(node.content)) {
      return node.content;
    }

    // Otherwise, generate Gherkin from node data
    return this.generateGherkin(node);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Validation
  // ═══════════════════════════════════════════════════════════════════════════

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
        line: 1,
      });
      return { valid: false, errors, warnings };
    }

    // Check for at least one scenario
    const scenarioCount = (content.match(/^\s*(Scenario|Scenario Outline):/gm) ?? []).length;
    if (scenarioCount === 0) {
      warnings.push({
        code: 'NO_SCENARIOS',
        message: 'No scenarios found. Feature files should contain at least one Scenario.',
        suggestion: 'Add a Scenario: block after the feature description',
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
          tags: imported.tags,
        };
      } catch {
        // Preview generation failed
      }
    }

    // Calculate stats
    const stats = {
      scenarioCount,
      stepCount: (content.match(/^\s*(Given|When|Then|And|But)\s+/gm) ?? []).length,
      lineCount: content.split('\n').length,
      tagCount: (content.match(/@[\w-]+/g) ?? []).length,
    };

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      parsedPreview,
      stats,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Rendering
  // ═══════════════════════════════════════════════════════════════════════════

  getRendererComponent(): Type<ContentRenderer> {
    return GherkinRendererComponent;
  }

  getRendererPriority(): number {
    return 10; // High priority for gherkin
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Editing
  // ═══════════════════════════════════════════════════════════════════════════

  getEditorComponent(): Type<ContentEditorComponent> | null {
    return null; // Use default code editor
  }

  getEditorConfig(): EditorConfig {
    return {
      ...DEFAULT_EDITOR_CONFIG,
      editorMode: 'code',
      supportsLivePreview: true,
      showLineNumbers: true,
      wordWrap: false, // Gherkin is structured, wrapping can be confusing
      toolbar: {
        enabled: true,
        position: 'top',
        actions: [
          { id: 'feature', label: 'Feature', icon: 'description', type: 'button' },
          { id: 'scenario', label: 'Scenario', icon: 'check_circle', type: 'button' },
          { id: 'given', label: 'Given', icon: 'play_arrow', type: 'button' },
          { id: 'when', label: 'When', icon: 'sync', type: 'button' },
          { id: 'then', label: 'Then', icon: 'done', type: 'button' },
          { id: 'save', label: 'Save', icon: 'save', shortcut: 'Ctrl+S', type: 'button' },
          { id: 'cancel', label: 'Cancel', icon: 'close', shortcut: 'Escape', type: 'button' },
        ],
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Detection
  // ═══════════════════════════════════════════════════════════════════════════

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

  // ═══════════════════════════════════════════════════════════════════════════
  // Metadata
  // ═══════════════════════════════════════════════════════════════════════════

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
      priority: 10,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private async readFile(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(reader.result as string);
      reader.onerror = () => reject(new Error(reader.error?.message ?? 'Failed to read file'));
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
    const scenarios =
      (node.metadata?.['scenarios'] as {
        id: string;
        title: string;
        type: string;
        steps?: GherkinStep[];
      }[]) || [];

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

  private validateSteps(content: string): {
    errors: ValidationError[];
    warnings: ValidationWarning[];
  } {
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
            line: lineNum - 1,
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
            suggestion: `Use "${properKeyword}" instead`,
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
      if (
        tagMatch &&
        tagMatch[1].length > 0 &&
        !line.includes('Feature:') &&
        !line.includes('Scenario')
      ) {
        // Tag is indented but not on a Feature/Scenario line
        const nextLine = lines[i + 1] || '';
        if (!/^\s*(Feature|Scenario|Scenario Outline):/.test(nextLine)) {
          warnings.push({
            code: 'ORPHAN_TAG',
            message: `Tag "${tagMatch[2]}" appears to be orphaned. Tags should precede Feature or Scenario.`,
            line: lineNum,
          });
        }
      }
    }

    return { warnings };
  }
}
