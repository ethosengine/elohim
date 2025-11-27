import { Component, Input, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ContentNode } from '../../models/content-node.model';

@Component({
  selector: 'app-gherkin-renderer',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="gherkin-container">
      <pre><code>{{ content }}</code></pre>
    </div>
  `,
  styleUrls: ['./gherkin-renderer.component.css']
})
export class GherkinRendererComponent implements OnChanges {
  @Input() node!: ContentNode;
  content: string = '';

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node'] && this.node) {
      this.content = typeof this.node.content === 'string' ? this.node.content : '';
    }
  }
}
