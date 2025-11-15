import { Component, OnInit, DestroyRef } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { DocumentGraphService } from '../../services/document-graph.service';
import { EpicNode, FeatureNode, ScenarioNode, DocumentNode } from '../../models';

interface ModuleSection {
  type: 'epic' | 'scenario';
  title: string;
  content: SafeHtml | string;
  level?: number;
  scenarioSteps?: any[];
}

@Component({
  selector: 'app-module-viewer',
  standalone: true,
  imports: [CommonModule, RouterLink],
  templateUrl: './module-viewer.component.html',
  styleUrls: ['./module-viewer.component.css']
})
export class ModuleViewerComponent implements OnInit {
  moduleName: string = '';
  epic: EpicNode | null = null;
  feature: FeatureNode | null = null;
  scenarios: ScenarioNode[] = [];
  interleavedSections: ModuleSection[] = [];

  constructor(
    private route: ActivatedRoute,
    private documentGraphService: DocumentGraphService,
    private sanitizer: DomSanitizer,
    private destroyRef: DestroyRef
  ) {}

  ngOnInit(): void {
    this.route.params
      .pipe(takeUntilDestroyed(this.destroyRef))
      .subscribe(params => {
        const moduleId = params['id'];
        this.loadModule(moduleId);
      });
  }

  private loadModule(moduleId: string): void {
    // For value-scanner, load the epic and feature
    if (moduleId === 'value-scanner') {
      this.moduleName = 'Value Scanner: Care Economy';

      // Find the epic node
      const epics = this.documentGraphService.getNodesByType('epic');
      const epicNode = epics.find((node: DocumentNode) =>
        node.type === 'epic' && node.id.includes('value-scanner')
      );

      if (epicNode) {
        this.epic = epicNode as EpicNode;
      }

      // Find the feature node
      const features = this.documentGraphService.getNodesByType('feature');
      const featureNode = features.find((node: DocumentNode) =>
        node.type === 'feature' && node.id.includes('care-economy')
      );

      if (featureNode) {
        this.feature = featureNode as FeatureNode;
        this.loadScenarios();
      }

      this.buildInterleavedView();
    }
  }

  private loadScenarios(): void {
    if (!this.feature) return;

    this.scenarios = this.feature.scenarioIds
      .map(id => this.documentGraphService.getNode(id))
      .filter((node): node is ScenarioNode => node !== undefined && node.type === 'scenario');
  }

  private buildInterleavedView(): void {
    if (!this.epic) return;

    this.interleavedSections = [];

    // Parse epic sections
    const sections = this.parseEpicSections();

    // Strategy: Interleave scenarios between major epic sections
    let scenarioIndex = 0;

    for (let i = 0; i < sections.length; i++) {
      // Add epic section
      this.interleavedSections.push({
        type: 'epic',
        title: sections[i].title,
        content: this.renderMarkdown(sections[i].content),
        level: sections[i].level
      });

      // After certain sections, insert a related scenario
      // Insert scenarios after major sections (h2 level)
      if (sections[i].level === 2 && scenarioIndex < this.scenarios.length) {
        const scenario = this.scenarios[scenarioIndex];
        this.interleavedSections.push({
          type: 'scenario',
          title: scenario.title,
          content: scenario.description || '',
          scenarioSteps: scenario.steps
        });
        scenarioIndex++;
      }
    }

    // Add any remaining scenarios at the end
    while (scenarioIndex < this.scenarios.length) {
      const scenario = this.scenarios[scenarioIndex];
      this.interleavedSections.push({
        type: 'scenario',
        title: scenario.title,
        content: scenario.description || '',
        scenarioSteps: scenario.steps
      });
      scenarioIndex++;
    }
  }

  private parseEpicSections(): { title: string; content: string; level: number }[] {
    if (!this.epic) return [];

    const sections: { title: string; content: string; level: number }[] = [];
    const lines = this.epic.markdownContent.split('\n');

    let currentSection: { title: string; content: string; level: number } | null = null;

    for (const line of lines) {
      const headingMatch = line.match(/^(#{1,6})\s+(.+)$/);

      if (headingMatch) {
        // Save previous section
        if (currentSection) {
          sections.push(currentSection);
        }

        // Start new section
        currentSection = {
          title: headingMatch[2],
          content: '',
          level: headingMatch[1].length
        };
      } else if (currentSection) {
        currentSection.content += line + '\n';
      }
    }

    // Add last section
    if (currentSection) {
      sections.push(currentSection);
    }

    return sections;
  }

  private renderMarkdown(content: string): SafeHtml {
    let html = content
      // Bold
      .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
      // Italic
      .replace(/\*(.+?)\*/g, '<em>$1</em>')
      // Code blocks
      .replace(/```([a-z]*)\n([\s\S]*?)```/g, '<pre><code class="language-$1">$2</code></pre>')
      // Inline code
      .replace(/`([^`]+)`/g, '<code>$1</code>')
      // Links
      .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank">$1</a>')
      // Line breaks to paragraphs
      .split('\n\n')
      .filter(para => para.trim())
      .map(para => {
        if (!para.match(/^<(pre|code)/)) {
          return `<p>${para.replace(/\n/g, '<br>')}</p>`;
        }
        return para;
      })
      .join('\n');

    return this.sanitizer.sanitize(1, html) || '';
  }

  getStepKeywordClass(keyword: string): string {
    const k = keyword.toLowerCase().trim();
    if (k === 'given') return 'step-given';
    if (k === 'when') return 'step-when';
    if (k === 'then') return 'step-then';
    if (k === 'and' || k === 'but') return 'step-and';
    return '';
  }
}
