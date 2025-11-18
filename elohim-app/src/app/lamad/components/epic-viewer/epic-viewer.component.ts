import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, RouterLink } from '@angular/router';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { DocumentGraphService } from '../../services/document-graph.service';
import { EpicNode, FeatureNode } from '../../models';

@Component({
  selector: 'app-epic-viewer',
  standalone: true,
  imports: [CommonModule, RouterLink],
  templateUrl: './epic-viewer.component.html',
  styleUrls: ['./epic-viewer.component.css']
})
export class EpicViewerComponent implements OnInit {
  epic: EpicNode | null = null;
  relatedFeatures: FeatureNode[] = [];
  renderedContent: SafeHtml = '';

  constructor(
    private readonly route: ActivatedRoute,
    private readonly documentGraphService: DocumentGraphService,
    private readonly sanitizer: DomSanitizer
  ) {}

  ngOnInit(): void {
    this.route.params.subscribe(params => {
      const epicId = params['id'];
      this.loadEpic(epicId);
    });
  }

  private loadEpic(epicId: string): void {
    const node = this.documentGraphService.getNode(epicId);

    if (node && node.type === 'epic') {
      this.epic = node as EpicNode;
      this.loadRelatedFeatures();
      this.renderMarkdown();
    }
  }

  private loadRelatedFeatures(): void {
    if (!this.epic) return;

    this.relatedFeatures = this.epic.featureIds
      .map(id => this.documentGraphService.getNode(id))
      .filter((node): node is FeatureNode => node !== undefined && node.type === 'feature');
  }

  private renderMarkdown(): void {
    if (!this.epic) return;

    let html = this.epic.markdownContent
      // Headings
      .replace(/^######\s+(.+)$/gm, '<h6>$1</h6>')
      .replace(/^#####\s+(.+)$/gm, '<h5>$1</h5>')
      .replace(/^####\s+(.+)$/gm, '<h4>$1</h4>')
      .replace(/^###\s+(.+)$/gm, '<h3>$1</h3>')
      .replace(/^##\s+(.+)$/gm, '<h2>$1</h2>')
      .replace(/^#\s+(.+)$/gm, '<h1>$1</h1>')
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
      // Paragraphs (basic)
      .split('\n\n')
      .map(para => {
        if (!/^<(h[1-6]|pre|ul|ol)/.exec(para)) {
          return `<p>${para}</p>`;
        }
        return para;
      })
      .join('\n');

    this.renderedContent = this.sanitizer.sanitize(1, html) ?? '';
  }

  getSectionAnchors(): { title: string; anchor: string; level: number }[] {
    if (!this.epic) return [];
    return this.epic.sections.map(s => ({
      title: s.title,
      anchor: s.anchor,
      level: s.level
    }));
  }

  scrollToSection(anchor: string): void {
    const element = document.getElementById(anchor);
    element?.scrollIntoView({ behavior: 'smooth' });
  }
}
