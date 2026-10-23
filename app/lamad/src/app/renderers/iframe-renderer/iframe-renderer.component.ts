import { CommonModule } from '@angular/common';
import {
  ChangeDetectorRef,
  Component,
  Input,
  OnChanges,
  SimpleChanges,
  inject,
} from '@angular/core';
import { DomSanitizer, SafeResourceUrl } from '@angular/platform-browser';

// @coverage: 80.7% (2026-02-24)

import { ContentNode } from '../../models/content-node.model';
import { AppsOriginService } from '../../services/apps-origin.service';

/**
 * HTML5 app content structure for doorway-served apps.
 * When contentFormat is 'html5-app', the content object should have this shape.
 */
export interface Html5AppContent {
  /** App slug for URL namespace (e.g., 'evolution-of-trust') */
  slug: string;
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
 *   `${appsOrigin}/apps/${slug}/${entryPoint}`
 *
 * On an apps-paired public name the raw bytes live on `apps.<name>`, so this
 * wrapper frames that origin's URL and the doorway serves the bare content to
 * the same-site frame (no second chrome). Unpaired names keep same-origin
 * `/apps/...`. The doorway handles zip extraction, caching, and headers.
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
        <!--
          TODO(capabilities): sandbox/allow are static here, not the record's declared
          capabilities. The one source to share is frameAttributes in
          elohim/elohim-chrome-asset/src/omni-element.js (mode x isolation x declared
          capabilities x the person's grants); do not grow a second capability table here.
        -->
        <iframe
          [src]="safeUrl"
          sandbox="allow-scripts allow-same-origin allow-forms allow-popups"
          data-testid="app-frame"
          [attr.data-frame-origin]="frameOrigin"
          class="iframe-content"
          [class.hidden]="loading"
          allowfullscreen
          (load)="onIframeLoad()"
          (error)="onIframeError()"
        ></iframe>
      }
    </div>
  `,
  styleUrls: ['./iframe-renderer.component.css'],
})
export class IframeRendererComponent implements OnChanges {
  @Input() node!: ContentNode;

  safeUrl: SafeResourceUrl | null = null;
  loading = true;
  errorMessage: string | null = null;
  fallbackUrl: string | null = null;
  /** The origin the html5-app frame loads from ('same-origin' when unpaired); null otherwise. */
  frameOrigin: string | null = null;
  // Note: sandbox is static in template due to Angular security restrictions (NG0910)

  private readonly sanitizer = inject(DomSanitizer);
  private readonly appsOrigin = inject(AppsOriginService);
  private readonly cdr = inject(ChangeDetectorRef);
  /** Bumped per configure, so a slow discovery read never frames a node that has moved on. */
  private configureSeq = 0;

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
    const seq = ++this.configureSeq;
    this.safeUrl = null;
    this.frameOrigin = null;
    const { contentFormat, metadata } = this.node;
    let content = this.node.content;

    // Parse JSON string content if needed (API returns content_body as string)
    if (typeof content === 'string') {
      try {
        content = JSON.parse(content) as string | object;
      } catch {
        // Not JSON, keep as string for URL mode below
      }
    }

    // HTML5 App mode: content is Html5AppContent object
    if (contentFormat === 'html5-app' && this.isHtml5AppContent(content)) {
      const app = content;
      this.fallbackUrl = app.fallbackUrl ?? null;
      const known = this.appsOrigin.appsOrigin();
      if (known !== undefined) {
        this.frameHtml5App(app, known);
      } else {
        // First frame this session: ask the doorway once, then frame. Framing
        // same-origin first and moving would load the app twice.
        void this.appsOrigin.resolve().then(origin => {
          if (seq === this.configureSeq) this.frameHtml5App(app, origin);
        });
      }
      return;
    }

    // Direct URL mode: content is a string URL
    if (typeof content === 'string' && content.startsWith('http')) {
      // Security: URL comes from trusted content node stored in backend
      // eslint-disable-next-line sonarjs/no-angular-bypass-sanitization
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(content);
      return;
    }

    // Legacy mode: check metadata for URL
    // Intentionally untyped: embedUrl/url are legacy metadata fields not in any domain schema
    if (metadata?.['embedUrl'] ?? metadata?.['url']) {
      const url = (metadata['embedUrl'] ?? metadata['url']) as string;
      // Security: URL comes from trusted content metadata stored in backend
      // eslint-disable-next-line sonarjs/no-angular-bypass-sanitization
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
      return;
    }

    // Fallback: try to use content as URL string
    const url = typeof content === 'string' ? content : '';
    if (url) {
      // Security: URL comes from trusted content node stored in backend
      // eslint-disable-next-line sonarjs/no-angular-bypass-sanitization
      this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
    } else {
      this.loading = false;
      this.errorMessage = 'No content URL available';
    }
  }

  /**
   * Frame an HTML5 app: `${appsOrigin}/apps/${slug}/${entryPoint}`.
   *
   * The frame shows the bundle the doorway serves and verifies by its hash.
   * `fallbackUrl` only feeds the "open elsewhere" link — an empty base means
   * same origin, never "use the fallback", which would frame an outside copy
   * instead of the protocol's own.
   *
   * Cold-cache delivery of `/apps/` is a service worker's job on whichever origin
   * the frame loads from: same-origin, the shell's worker; on an apps-paired name,
   * the worker registered on the apps origin (src/apps-sw.ts), not the name's.
   */
  private frameHtml5App(content: Html5AppContent, appsOrigin: string | null): void {
    const url = this.appsOrigin.appUrl(content.slug, content.entryPoint, appsOrigin);
    this.frameOrigin = appsOrigin ?? 'same-origin';
    // Security: URL is the page's own origin or the apps origin its doorway pairs
    // it with (AppsOriginService only accepts apps.<this host>), plus content metadata.
    // eslint-disable-next-line sonarjs/no-angular-bypass-sanitization
    this.safeUrl = this.sanitizer.bypassSecurityTrustResourceUrl(url);
    this.cdr.markForCheck();
  }

  /**
   * Type guard for Html5AppContent.
   */
  private isHtml5AppContent(content: unknown): content is Html5AppContent {
    if (typeof content !== 'object' || content === null) {
      return false;
    }
    const obj = content as Record<string, unknown>;
    return typeof obj['slug'] === 'string' && typeof obj['entryPoint'] === 'string';
  }
}
