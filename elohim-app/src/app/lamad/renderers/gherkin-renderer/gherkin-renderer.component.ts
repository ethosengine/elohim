import { Component, Input, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ContentNode } from '../../models/content-node.model';

/**
 * Parsed Gherkin step with keyword highlighting.
 */
interface GherkinStep {
  keyword: string;  // Given, When, Then, And, But
  text: string;
  docString?: string;
  dataTable?: string[][];
}

/**
 * Parsed Gherkin scenario/outline.
 */
interface GherkinScenario {
  type: 'scenario' | 'scenario_outline' | 'background';
  name: string;
  tags: string[];
  steps: GherkinStep[];
  examples?: {
    name: string;
    table: string[][];
  }[];
  collapsed: boolean;
}

/**
 * Parsed Gherkin feature file structure.
 */
interface GherkinFeature {
  name: string;
  description: string;
  tags: string[];
  background?: GherkinScenario;
  scenarios: GherkinScenario[];
}

@Component({
  selector: 'app-gherkin-renderer',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="gherkin-container" [class.embedded]="embedded" *ngIf="feature">
      <!-- Feature Header -->
      <header class="feature-header" *ngIf="feature.name">
        <div class="feature-tags" *ngIf="feature.tags.length">
          <span class="tag" *ngFor="let tag of feature.tags">{{ tag }}</span>
        </div>
        <h1 class="feature-title">
          <span class="keyword">Feature:</span> {{ feature.name }}
        </h1>
        <p class="feature-description" *ngIf="feature.description">
          {{ feature.description }}
        </p>
        <div class="feature-stats">
          <span class="stat">{{ feature.scenarios.length }} scenario{{ feature.scenarios.length !== 1 ? 's' : '' }}</span>
          <span class="stat" *ngIf="totalSteps > 0">{{ totalSteps }} steps</span>
        </div>
      </header>

      <!-- Background (if present) -->
      <section class="background-section" *ngIf="feature.background">
        <div class="scenario-header">
          <h2 class="scenario-title">
            <span class="keyword">Background:</span>
            {{ feature.background.name }}
          </h2>
        </div>
        <div class="steps">
          <ng-container *ngFor="let step of feature.background.steps">
            <ng-container *ngTemplateOutlet="stepTemplate; context: { step: step }"></ng-container>
          </ng-container>
        </div>
      </section>

      <!-- Scenarios -->
      <section
        class="scenario-section"
        *ngFor="let scenario of feature.scenarios; let i = index"
        [class.collapsed]="scenario.collapsed">

        <div class="scenario-header" (click)="toggleScenario(i)">
          <div class="scenario-tags" *ngIf="scenario.tags.length">
            <span class="tag" *ngFor="let tag of scenario.tags">{{ tag }}</span>
          </div>
          <h2 class="scenario-title">
            <span class="expand-icon">{{ scenario.collapsed ? '▶' : '▼' }}</span>
            <span class="keyword">{{ getScenarioKeyword(scenario.type) }}:</span>
            {{ scenario.name }}
          </h2>
          <span class="step-count">{{ scenario.steps.length }} steps</span>
        </div>

        <div class="scenario-body" *ngIf="!scenario.collapsed">
          <div class="steps">
            <ng-container *ngFor="let step of scenario.steps">
              <ng-container *ngTemplateOutlet="stepTemplate; context: { step: step }"></ng-container>
            </ng-container>
          </div>

          <!-- Examples table for Scenario Outline -->
          <div class="examples" *ngIf="scenario.examples?.length">
            <div class="example-block" *ngFor="let example of scenario.examples">
              <h3 class="example-title">
                <span class="keyword">Examples:</span>
                {{ example.name }}
              </h3>
              <table class="data-table" *ngIf="example.table.length">
                <thead>
                  <tr>
                    <th *ngFor="let cell of example.table[0]">{{ cell }}</th>
                  </tr>
                </thead>
                <tbody>
                  <tr *ngFor="let row of example.table.slice(1)">
                    <td *ngFor="let cell of row">{{ cell }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </section>

      <!-- Step Template -->
      <ng-template #stepTemplate let-step="step">
        <div class="step">
          <span class="step-keyword" [class]="'keyword-' + step.keyword.toLowerCase().trim()">
            {{ step.keyword }}
          </span>
          <span class="step-text" [innerHTML]="highlightStepText(step.text)"></span>
        </div>

        <!-- Doc String -->
        <pre class="doc-string" *ngIf="step.docString">{{ step.docString }}</pre>

        <!-- Data Table -->
        <table class="data-table step-table" *ngIf="step.dataTable?.length">
          <tbody>
            <tr *ngFor="let row of step.dataTable">
              <td *ngFor="let cell of row">{{ cell }}</td>
            </tr>
          </tbody>
        </table>
      </ng-template>
    </div>

    <!-- Fallback for unparseable content -->
    <div class="gherkin-fallback" *ngIf="!feature && content">
      <pre><code>{{ content }}</code></pre>
    </div>
  `,
  styleUrls: ['./gherkin-renderer.component.css']
})
export class GherkinRendererComponent implements OnChanges {
  @Input() node!: ContentNode;
  /** When true, renderer adapts to fit within parent container */
  @Input() embedded = false;

  feature: GherkinFeature | null = null;
  content: string = '';
  totalSteps = 0;

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node'] && this.node) {
      this.content = typeof this.node.content === 'string' ? this.node.content : '';
      this.parseGherkin();
    }
  }

  toggleScenario(index: number): void {
    if (this.feature) {
      this.feature.scenarios[index].collapsed = !this.feature.scenarios[index].collapsed;
    }
  }

  getScenarioKeyword(type: string): string {
    switch (type) {
      case 'scenario_outline': return 'Scenario Outline';
      case 'background': return 'Background';
      default: return 'Scenario';
    }
  }

  highlightStepText(text: string): string {
    // Highlight <placeholders> and "strings"
    return text
      .replace(/<([^>]+)>/g, '<span class="placeholder">&lt;$1&gt;</span>')
      .replace(/"([^"]+)"/g, '<span class="string">"$1"</span>');
  }

  private parseGherkin(): void {
    if (!this.content) {
      this.feature = null;
      return;
    }

    try {
      const lines = this.content.split('\n');
      const ctx = this.createParseContext();

      for (const line of lines) {
        this.parseLine(line, ctx);
      }

      this.finalizeFeature(ctx);
    } catch (error) {
      console.warn('Failed to parse Gherkin:', error);
      this.feature = null;
    }
  }

  /** Create initial parsing context */
  private createParseContext(): ParseContext {
    return {
      feature: { name: '', description: '', tags: [], scenarios: [] },
      currentTags: [],
      currentScenario: null,
      currentExample: null,
      inDocString: false,
      docStringContent: [],
      inDescription: false,
      descriptionLines: []
    };
  }

  /** Parse a single line of Gherkin */
  private parseLine(line: string, ctx: ParseContext): void {
    const trimmed = line.trim();

    // Handle doc strings first (they can contain anything)
    if (this.handleDocString(line, trimmed, ctx)) return;

    // Skip empty lines outside doc strings
    if (!trimmed) {
      ctx.inDescription = false;
      return;
    }

    // Try each line type in order
    if (this.handleTags(trimmed, ctx)) return;
    if (this.handleFeature(trimmed, ctx)) return;
    if (this.handleDescription(trimmed, ctx)) return;
    if (this.handleBackground(trimmed, ctx)) return;
    if (this.handleScenarioOutline(trimmed, ctx)) return;
    if (this.handleScenario(trimmed, ctx)) return;
    if (this.handleExamples(trimmed, ctx)) return;
    if (this.handleStep(trimmed, ctx)) return;
    this.handleDataTable(trimmed, ctx);
  }

  /** Handle doc string delimiters and content */
  private handleDocString(line: string, trimmed: string, ctx: ParseContext): boolean {
    if (trimmed.startsWith('"""') || trimmed.startsWith("'''")) {
      if (ctx.inDocString) {
        // End doc string
        if (ctx.currentScenario?.steps.length) {
          ctx.currentScenario.steps[ctx.currentScenario.steps.length - 1].docString = ctx.docStringContent.join('\n');
        }
        ctx.docStringContent = [];
        ctx.inDocString = false;
      } else {
        ctx.inDocString = true;
      }
      return true;
    }
    if (ctx.inDocString) {
      ctx.docStringContent.push(line);
      return true;
    }
    return false;
  }

  /** Handle @tag lines */
  private handleTags(trimmed: string, ctx: ParseContext): boolean {
    if (!trimmed.startsWith('@')) return false;
    const tags = trimmed.split(/\s+/).filter(t => t.startsWith('@'));
    ctx.currentTags.push(...tags);
    return true;
  }

  /** Handle Feature: line */
  private handleFeature(trimmed: string, ctx: ParseContext): boolean {
    if (!trimmed.startsWith('Feature:')) return false;
    ctx.feature.name = trimmed.substring(8).trim();
    ctx.feature.tags = [...ctx.currentTags];
    ctx.currentTags = [];
    ctx.inDescription = true;
    return true;
  }

  /** Handle feature description lines */
  private handleDescription(trimmed: string, ctx: ParseContext): boolean {
    if (!ctx.inDescription) return false;
    if (trimmed.startsWith('Scenario') || trimmed.startsWith('Background')) return false;
    ctx.descriptionLines.push(trimmed);
    return true;
  }

  /** Handle Background: line */
  private handleBackground(trimmed: string, ctx: ParseContext): boolean {
    if (!trimmed.startsWith('Background:')) return false;
    this.finalizeDescription(ctx);
    ctx.currentScenario = {
      type: 'background',
      name: trimmed.substring(11).trim(),
      tags: [],
      steps: [],
      collapsed: false
    };
    ctx.feature.background = ctx.currentScenario;
    return true;
  }

  /** Handle Scenario Outline: or Scenario Template: */
  private handleScenarioOutline(trimmed: string, ctx: ParseContext): boolean {
    if (!trimmed.startsWith('Scenario Outline:') && !trimmed.startsWith('Scenario Template:')) return false;
    this.finalizeDescription(ctx);
    this.saveCurrentScenario(ctx.feature, ctx.currentScenario);
    ctx.currentScenario = {
      type: 'scenario_outline',
      name: trimmed.replace(/^Scenario (Outline|Template):/, '').trim(),
      tags: [...ctx.currentTags],
      steps: [],
      examples: [],
      collapsed: false
    };
    ctx.currentTags = [];
    ctx.currentExample = null;
    return true;
  }

  /** Handle Scenario: line */
  private handleScenario(trimmed: string, ctx: ParseContext): boolean {
    if (!trimmed.startsWith('Scenario:')) return false;
    this.finalizeDescription(ctx);
    this.saveCurrentScenario(ctx.feature, ctx.currentScenario);
    ctx.currentScenario = {
      type: 'scenario',
      name: trimmed.substring(9).trim(),
      tags: [...ctx.currentTags],
      steps: [],
      collapsed: false
    };
    ctx.currentTags = [];
    ctx.currentExample = null;
    return true;
  }

  /** Handle Examples: line */
  private handleExamples(trimmed: string, ctx: ParseContext): boolean {
    if (!trimmed.startsWith('Examples:')) return false;
    ctx.currentExample = { name: trimmed.substring(9).trim(), table: [] };
    ctx.currentScenario?.examples?.push(ctx.currentExample);
    return true;
  }

  /** Handle step lines (Given/When/Then/And/But) */
  private handleStep(trimmed: string, ctx: ParseContext): boolean {
    const stepMatch = /^(Given|When|Then|And|But|\*)\s+(.+)$/.exec(trimmed);
    if (!stepMatch || !ctx.currentScenario) return false;
    ctx.currentScenario.steps.push({ keyword: stepMatch[1] + ' ', text: stepMatch[2] });
    return true;
  }

  /** Handle data table rows */
  private handleDataTable(trimmed: string, ctx: ParseContext): void {
    if (!trimmed.startsWith('|') || !trimmed.endsWith('|')) return;
    const cells = trimmed.slice(1, -1).split('|').map(c => c.trim());
    if (ctx.currentExample) {
      ctx.currentExample.table.push(cells);
    } else if (ctx.currentScenario?.steps.length) {
      const lastStep = ctx.currentScenario.steps[ctx.currentScenario.steps.length - 1];
      lastStep.dataTable ??= [];
      lastStep.dataTable.push(cells);
    }
  }

  /** Finalize description when entering scenario/background */
  private finalizeDescription(ctx: ParseContext): void {
    ctx.inDescription = false;
    if (!ctx.feature.description && ctx.descriptionLines.length) {
      ctx.feature.description = ctx.descriptionLines.join(' ').trim();
      ctx.descriptionLines = [];
    }
  }

  /** Finalize feature after parsing all lines */
  private finalizeFeature(ctx: ParseContext): void {
    this.saveCurrentScenario(ctx.feature, ctx.currentScenario);

    if (!ctx.feature.description && ctx.descriptionLines.length) {
      ctx.feature.description = ctx.descriptionLines.join(' ').trim();
    }

    // Handle standalone scenarios without Feature: line
    if (!ctx.feature.name && ctx.feature.scenarios.length > 0) {
      ctx.feature.name = this.node?.title || ctx.feature.scenarios[0].name || 'Untitled Feature';
    }

    // Calculate total steps
    this.totalSteps = ctx.feature.scenarios.reduce((sum, s) => sum + s.steps.length, 0);
    if (ctx.feature.background) {
      this.totalSteps += ctx.feature.background.steps.length;
    }

    this.feature = (ctx.feature.name || ctx.feature.scenarios.length > 0) ? ctx.feature : null;
  }

  private saveCurrentScenario(feature: GherkinFeature, scenario: GherkinScenario | null): void {
    if (scenario && scenario.type !== 'background') {
      feature.scenarios.push(scenario);
    }
  }
}

/** Internal parsing context */
interface ParseContext {
  feature: GherkinFeature;
  currentTags: string[];
  currentScenario: GherkinScenario | null;
  currentExample: { name: string; table: string[][] } | null;
  inDocString: boolean;
  docStringContent: string[];
  inDescription: boolean;
  descriptionLines: string[];
}
