import { Component, Input, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DomSanitizer, SafeResourceUrl } from '@angular/platform-browser';
import { ContentNode } from '../../models/content-node.model';

@Component({
  selector: 'app-iframe-renderer',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="iframe-container">
      @if (safeUrl) {
        <iframe
          [src]="safeUrl"
          sandbox="allow-scripts allow-same-origin"
          class="iframe-content"
          allowfullscreen
        ></iframe>
      }
    </div>
  `,
  styleUrls: ['./iframe-renderer.component.css']
})
export class IframeRendererComponent implements OnChanges {
  @Input() node!: ContentNode;

  safeUrl: SafeResourceUrl | null = null;

  constructor(private readonly sanitizer: DomSanitizer) {}

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node'] && this.node) {
      this.configureIframe();
    }
  }

  private configureIframe(): void {
    const url = typeof this.node.content === 'string'
      ? this.node.content
      : '';

    this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
  }
}
