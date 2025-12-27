import { Component, Input, OnChanges, SimpleChanges } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DomSanitizer, SafeResourceUrl } from '@angular/platform-browser';
import { ContentNode } from '../../models/content-node.model';
import { environment } from '../../../../environments/environment';

/**
 * HTML5 app content structure for doorway-served apps.
 * When contentFormat is 'html5-app', the content object should have this shape.
 */
export interface Html5AppContent {
  /** App identifier for URL namespace (e.g., 'evolution-of-trust') */
  appId: string;
  /** Entry point file within the zip (e.g., 'index.html') */
  entryPoint: string;
  /** Optional fallback URL if doorway is unavailable */
  fallbackUrl?: string;
}

/**
 * IframeRendererComponent - Renders external content in a sandboxed iframe.
 *
 * Supports two modes:
 * 1. Direct URL mode: content is a URL string (e.g., video-embed format)
 * 2. HTML5 App mode: content is Html5AppContent, served via doorway's /apps/ endpoint
 *
 * For HTML5 apps, the component builds the doorway URL:
 *   `${doorwayUrl}/apps/${appId}/${entryPoint}`
 *
 * The doorway handles zip extraction, caching, and serving with proper headers.
 */
@Component({
  selector: 'app-iframe-renderer',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="iframe-container" [class.loading]="loading">
      @if (loading) {
        <div class="loading-overlay">
          <div class="spinner"></div>
          <p>Loading application...</p>
        </div>
      }
      @if (errorMessage) {
        <div class="error-overlay">
          <p class="error-message">{{ errorMessage }}</p>
          @if (fallbackUrl) {
            <a [href]="fallbackUrl" target="_blank" rel="noopener" class="fallback-link">
              Open in new tab
            </a>
          }
        </div>
      }
      @if (safeUrl) {
        <iframe
          [src]="safeUrl"
          sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
          class="iframe-content"
          [class.hidden]="loading"
          allowfullscreen
          (load)="onIframeLoad()"
          (error)="onIframeError()"
        ></iframe>
      }
    </div>
  `,
  styleUrls: ['./iframe-renderer.component.css']
})
export class IframeRendererComponent implements OnChanges {
  @Input() node!: ContentNode;

  safeUrl: SafeResourceUrl | null = null;
  loading = true;
  errorMessage: string | null = null;
  fallbackUrl: string | null = null;
  // Note: sandbox is static in template due to Angular security restrictions (NG0910)

  constructor(private readonly sanitizer: DomSanitizer) {}

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node'] && this.node) {
      this.loading = true;
      this.errorMessage = null;
      this.configureIframe();
    }
  }

  onIframeLoad(): void {
    this.loading = false;
  }

  onIframeError(): void {
    this.loading = false;
    this.errorMessage = 'Failed to load application';
  }

  private configureIframe(): void {
    const { contentFormat, content, metadata } = this.node;

    // HTML5 App mode: content is Html5AppContent object
    if (contentFormat === 'html5-app' && this.isHtml5AppContent(content)) {
      const url = this.buildHtml5AppUrl(content);
      this.fallbackUrl = content.fallbackUrl || null;
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
      return;
    }

    // Direct URL mode: content is a string URL
    if (typeof content === 'string' && content.startsWith('http')) {
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(content);
      return;
    }

    // Legacy mode: check metadata for URL
    if (metadata?.['embedUrl'] || metadata?.['url']) {
      const url = (metadata['embedUrl'] || metadata['url']) as string;
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
      return;
    }

    // Fallback: try to use content as URL string
    const url = typeof content === 'string' ? content : '';
    if (url) {
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
    } else {
      this.loading = false;
      this.errorMessage = 'No content URL available';
    }
  }

  /**
   * Build the doorway URL for an HTML5 app.
   * Format: ${doorwayUrl}/apps/${appId}/${entryPoint}
   */
  private buildHtml5AppUrl(content: Html5AppContent): string {
    const doorwayUrl = environment.doorwayUrl || '';
    const { appId, entryPoint } = content;

    // If no doorway URL configured, try fallback
    if (!doorwayUrl && content.fallbackUrl) {
      return content.fallbackUrl;
    }

    return `${doorwayUrl}/apps/${appId}/${entryPoint}`;
  }

  /**
   * Type guard for Html5AppContent.
   */
  private isHtml5AppContent(content: unknown): content is Html5AppContent {
    if (typeof content !== 'object' || content === null) {
      return false;
    }
    const obj = content as Record<string, unknown>;
    return typeof obj['appId'] === 'string' && typeof obj['entryPoint'] === 'string';
  }

}
