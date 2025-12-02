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
      const feature: GherkinFeature = {
        name: '',
        description: '',
        tags: [],
        scenarios: []
      };

      let currentTags: string[] = [];
      let currentScenario: GherkinScenario | null = null;
      let currentExample: { name: string; table: string[][] } | null = null;
      let inDocString = false;
      let docStringContent: string[] = [];
      let inDescription = false;
      let descriptionLines: string[] = [];

      for (const line of lines) {
        const trimmed = line.trim();

        // Skip empty lines (unless in doc string)
        if (!trimmed && !inDocString) {
          if (inDescription) {
            inDescription = false;
          }
          continue;
        }

        // Doc string handling
        if (trimmed.startsWith('"""') || trimmed.startsWith("'''")) {
          if (inDocString) {
            // End doc string
            if (currentScenario && currentScenario.steps.length > 0) {
              currentScenario.steps[currentScenario.steps.length - 1].docString = docStringContent.join('\n');
            }
            docStringContent = [];
            inDocString = false;
          } else {
            // Start doc string
            inDocString = true;
          }
          continue;
        }

        if (inDocString) {
          docStringContent.push(line);
          continue;
        }

        // Tags
        if (trimmed.startsWith('@')) {
          const tags = trimmed.split(/\s+/).filter(t => t.startsWith('@'));
          currentTags.push(...tags);
          continue;
        }

        // Feature
        if (trimmed.startsWith('Feature:')) {
          feature.name = trimmed.substring(8).trim();
          feature.tags = [...currentTags];
          currentTags = [];
          inDescription = true;
          continue;
        }

        // Feature description (lines after Feature: before first Scenario/Background)
        if (inDescription && !trimmed.startsWith('Scenario') && !trimmed.startsWith('Background')) {
          descriptionLines.push(trimmed);
          continue;
        }

        // Background
        if (trimmed.startsWith('Background:')) {
          inDescription = false;
          feature.description = descriptionLines.join(' ').trim();
          descriptionLines = [];

          currentScenario = {
            type: 'background',
            name: trimmed.substring(11).trim(),
            tags: [],
            steps: [],
            collapsed: false
          };
          feature.background = currentScenario;
          continue;
        }

        // Scenario / Scenario Outline
        if (trimmed.startsWith('Scenario Outline:') || trimmed.startsWith('Scenario Template:')) {
          inDescription = false;
          if (!feature.description && descriptionLines.length) {
            feature.description = descriptionLines.join(' ').trim();
            descriptionLines = [];
          }

          this.saveCurrentScenario(feature, currentScenario);
          currentScenario = {
            type: 'scenario_outline',
            name: trimmed.replace(/^Scenario (Outline|Template):/, '').trim(),
            tags: [...currentTags],
            steps: [],
            examples: [],
            collapsed: false
          };
          currentTags = [];
          currentExample = null;
          continue;
        }

        if (trimmed.startsWith('Scenario:')) {
          inDescription = false;
          if (!feature.description && descriptionLines.length) {
            feature.description = descriptionLines.join(' ').trim();
            descriptionLines = [];
          }

          this.saveCurrentScenario(feature, currentScenario);
          currentScenario = {
            type: 'scenario',
            name: trimmed.substring(9).trim(),
            tags: [...currentTags],
            steps: [],
            collapsed: false
          };
          currentTags = [];
          currentExample = null;
          continue;
        }

        // Examples
        if (trimmed.startsWith('Examples:')) {
          currentExample = {
            name: trimmed.substring(9).trim(),
            table: []
          };
          if (currentScenario?.examples) {
            currentScenario.examples.push(currentExample);
          }
          continue;
        }

        // Steps
        const stepRegex = /^(Given|When|Then|And|But|\*)\s+(.+)$/;
        const stepMatch = stepRegex.exec(trimmed);
        if (stepMatch && currentScenario) {
          currentScenario.steps.push({
            keyword: stepMatch[1] + ' ',
            text: stepMatch[2]
          });
          continue;
        }

        // Data table row
        if (trimmed.startsWith('|') && trimmed.endsWith('|')) {
          const cells = trimmed
            .slice(1, -1)
            .split('|')
            .map(c => c.trim());

          if (currentExample) {
            currentExample.table.push(cells);
          } else if (currentScenario && currentScenario.steps.length > 0) {
            const lastStep = currentScenario.steps[currentScenario.steps.length - 1];
            lastStep.dataTable ??= [];
            lastStep.dataTable.push(cells);
          }
        }
      }

      // Save final scenario
      this.saveCurrentScenario(feature, currentScenario);

      // Set description if not already set
      if (!feature.description && descriptionLines.length) {
        feature.description = descriptionLines.join(' ').trim();
      }

      // Handle standalone scenarios (missing Feature: line)
      // If we found scenarios but no feature name, valid case for standalone scenarios
      if (!feature.name && feature.scenarios.length > 0) {
        // Use the title from the node if available, or first scenario name
        feature.name = this.node?.title || feature.scenarios[0].name || 'Untitled Feature';
      }

      // Calculate total steps
      this.totalSteps = feature.scenarios.reduce((sum, s) => sum + s.steps.length, 0);
      if (feature.background) {
        this.totalSteps += feature.background.steps.length;
      }

      // Valid if it has a name (found or inferred) and scenarios
      this.feature = (feature.name || feature.scenarios.length > 0) ? feature : null;

    } catch (error) {
      console.warn('Failed to parse Gherkin:', error);
      this.feature = null;
    }
  }

  private saveCurrentScenario(feature: GherkinFeature, scenario: GherkinScenario | null): void {
    if (scenario && scenario.type !== 'background') {
      feature.scenarios.push(scenario);
    }
  }
}
