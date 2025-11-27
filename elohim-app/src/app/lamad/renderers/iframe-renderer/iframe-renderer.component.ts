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
      <iframe
        [src]="safeUrl"
        [attr.sandbox]="sandboxAttr"
        class="iframe-content"
        allowfullscreen
      ></iframe>
    </div>
  `,
  styleUrls: ['./iframe-renderer.component.css']
})
export class IframeRendererComponent implements OnChanges {
  @Input() node!: ContentNode;

  safeUrl: SafeResourceUrl | null = null;
  sandboxAttr: string = 'allow-scripts allow-same-origin';

  constructor(private sanitizer: DomSanitizer) {}

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

    // Apply security policy from metadata
    if (this.node.metadata?.securityPolicy?.sandbox) {
      this.sandboxAttr = this.node.metadata.securityPolicy.sandbox.join(' ');
    }
  }
}
