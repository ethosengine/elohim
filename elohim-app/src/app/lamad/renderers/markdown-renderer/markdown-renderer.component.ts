import { Component, Input, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { marked } from 'marked';
import { ContentNode } from '../../models/content-node.model';

@Component({
  selector: 'app-markdown-renderer',
  standalone: true,
  imports: [CommonModule],
  template: `
    <article class="markdown-content prose prose-slate dark:prose-invert max-w-none">
      <div [innerHTML]="renderedContent"></div>
    </article>
  `,
  styleUrls: ['./markdown-renderer.component.css']
})
export class MarkdownRendererComponent implements OnChanges {
  @Input() node!: ContentNode;

  renderedContent: SafeHtml = '';

  constructor(private sanitizer: DomSanitizer) {}

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node'] && this.node) {
      this.renderMarkdown();
    }
  }

  private async renderMarkdown(): Promise<void> {
    if (typeof this.node.content !== 'string') {
      console.warn('Markdown renderer expects string content');
      return;
    }

    const html = await marked(this.node.content);
    this.renderedContent = this.sanitizer.bypassSecurityTrustHtml(html);
  }
}
