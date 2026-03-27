import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  OnChanges,
  SimpleChanges,
  Output,
  EventEmitter,
  AfterViewInit,
  ElementRef,
  ViewChild,
  ViewContainerRef,
  ComponentRef,
  OnDestroy,
  inject,
} from '@angular/core';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';
import { Router } from '@angular/router';

// @coverage: 72.8% (2026-02-24)

import hljs from 'highlight.js';
import { Marked } from 'marked';
import { markedHighlight } from 'marked-highlight';
import { firstValueFrom, Subscription } from 'rxjs';

import { EprPopoverComponent } from '@app/elohim/components/epr-popover/epr-popover.component';
import { EprResolverService, type StepRef } from '@app/elohim/services/epr-resolver.service';
import { StorageClientService } from '@app/elohim/services/storage-client.service';
import { parseEpr } from '@app/elohim/utils/epr-ref';

import { ContentNode } from '../../models/content-node.model';
import { isConceptNode } from '../../generated/content-node-types';
import { PathContextService } from '../../services/path-context.service';
import { PathService } from '../../services/path.service';

/**
 * Table of Contents entry extracted from markdown headings.
 */
export interface TocEntry {
  id: string;
  text: string;
  level: number;
}

const EPR_ANCHOR_SELECTOR = 'a[data-epr]';

@Component({
  selector: 'app-markdown-renderer',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="markdown-wrapper" [class.embedded]="embedded">
      <!-- TOC Toggle Button (for mobile/compact view) - visible in embedded mode too -->
      <button
        *ngIf="tocEntries.length > 0"
        class="toc-toggle"
        (click)="toggleToc()"
        [class.toc-open]="tocVisible"
        title="Toggle Table of Contents"
        data-testid="markdown-toc-toggle"
      >
        <span class="toc-icon">&#9776;</span>
      </button>

      <!-- TOC Backdrop - click to dismiss -->
      <div *ngIf="tocVisible" class="toc-backdrop" (click)="toggleToc()"></div>

      <!-- Table of Contents Sidebar -->
      <nav *ngIf="tocEntries.length > 0" class="toc-sidebar" [class.toc-visible]="tocVisible">
        <div class="toc-header">
          <span>Contents</span>
          <button class="toc-close" (click)="toggleToc()" data-testid="markdown-toc-close">
            &times;
          </button>
        </div>
        <ul class="toc-list">
          <li
            *ngFor="let entry of tocEntries"
            [class]="'toc-level-' + entry.level"
            [class.toc-active]="activeHeadingId === entry.id"
          >
            <a [href]="'#' + entry.id" (click)="scrollToHeading($event, entry.id)">
              {{ entry.text }}
            </a>
          </li>
        </ul>
      </nav>

      <!-- Main Content -->
      <article
        #contentEl
        class="markdown-content"
        [class.has-toc]="tocEntries.length > 0 && !embedded"
      >
        <div [innerHTML]="renderedContent"></div>
      </article>

      <!-- Back to Top Button -->
      <button
        *ngIf="showBackToTop"
        class="back-to-top"
        (click)="scrollToTop()"
        title="Back to top"
        data-testid="markdown-back-to-top"
      >
        &uarr;
      </button>
    </div>
  `,
  styleUrls: ['./markdown-renderer.component.css'],
})
export class MarkdownRendererComponent implements OnChanges, AfterViewInit, OnDestroy {
  @Input() node!: ContentNode;
  /** When true, renderer adapts to fit within parent container without TOC/back-to-top */
  @Input() embedded = false;
  /** Path steps for context-aware EPR link resolution (provided by path step view) */
  @Input() pathSteps: StepRef[] = [];
  @Output() tocGenerated = new EventEmitter<TocEntry[]>();

  @ViewChild('contentEl') contentEl!: ElementRef<HTMLElement>;

  renderedContent: SafeHtml = '';
  tocEntries: TocEntry[] = [];
  tocVisible = false;
  activeHeadingId = '';
  showBackToTop = false;

  private readonly marked: Marked;
  private scrollListener?: () => void;
  private eprClickListener?: (e: Event) => void;
  private headingElements: HTMLElement[] = [];
  private readonly storageClient = inject(StorageClientService);
  private readonly eprResolver = inject(EprResolverService);
  private readonly pathContext = inject(PathContextService);
  private readonly pathService = inject(PathService);
  private readonly router = inject(Router);
  private readonly vcr = inject(ViewContainerRef);
  private activePopover: ComponentRef<EprPopoverComponent> | null = null;
  private popoverSub: Subscription | null = null;
  private eprHoverListener?: (e: Event) => void;
  private eprLeaveListener?: (e: Event) => void;
  private hoveredEprAnchor: HTMLAnchorElement | null = null;
  private popoverShowTimer: ReturnType<typeof setTimeout> | null = null;
  private popoverDismissTimer: ReturnType<typeof setTimeout> | null = null;

  /** Prefetched cross-path matches, keyed by content ID. */
  private readonly crossPathCache = new Map<
    string,
    import('@app/elohim/services/epr-resolver.service').CrossPathMatch[]
  >();

  private readonly sanitizer = inject(DomSanitizer);

  constructor() {
    // Configure marked with syntax highlighting
    this.marked = new Marked(
      markedHighlight({
        langPrefix: 'hljs language-',
        highlight(code, lang) {
          const language = hljs.getLanguage(lang) ? lang : 'plaintext';
          return hljs.highlight(code, { language }).value;
        },
      })
    );

    // Configure marked options
    this.marked.setOptions({
      gfm: true,
      breaks: false,
    });

    // Configure custom renderer to transform blob URLs and intercept epr: links
    this.marked.use({
      renderer: {
        image: token => {
          // Transform blob URLs to full doorway URLs
          const resolvedHref = this.resolveBlobUrl(token.href);
          const title = token.title ? ` title="${token.title}"` : '';
          return `<img src="${resolvedHref}" alt="${token.text}"${title}>`;
        },
        link: token => {
          if (token.href?.startsWith('epr:')) {
            // EPR link — render as data attribute for click handler interception
            const escapedHref = token.href.replaceAll('"', '&quot;');
            const escapedText = token.text ?? token.href;
            return `<a href="#" data-epr="${escapedHref}" class="epr-link-inline">${escapedText}</a>`;
          }
          // Non-EPR links render normally as standard anchors
          const href = token.href ?? '';
          const title = token.title ? ` title="${token.title}"` : '';
          const text = token.text ?? href;
          return `<a href="${href}"${title}>${text}</a>`;
        },
      },
    });
  }

  /**
   * Resolve blob URL references to full URLs.
   * Transforms /blob/hash and blob/hash to strategy-aware full URLs.
   */
  private resolveBlobUrl(url: string): string {
    if (!url) return url;

    // Already a full URL - pass through
    if (url.startsWith('http://') || url.startsWith('https://') || url.startsWith('data:')) {
      return url;
    }

    // Handle /blob/{hash} format
    if (url.startsWith('/blob/')) {
      const blobHash = url.slice(6);
      return this.storageClient.getBlobUrl(blobHash);
    }

    // Handle blob/{hash} format (no leading slash)
    if (url.startsWith('blob/')) {
      const blobHash = url.slice(5);
      return this.storageClient.getBlobUrl(blobHash);
    }

    // Not a blob URL, return as-is
    return url;
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node'] && this.node) {
      void this.renderMarkdown();
    }
  }

  ngAfterViewInit(): void {
    this.setupScrollListener();
    this.setupEprClickHandler();
    this.setupEprHoverHandler();
  }

  ngOnDestroy(): void {
    this.destroyPopover();
    if (this.scrollListener) {
      window.removeEventListener('scroll', this.scrollListener);
    }
    if (this.eprClickListener && this.contentEl) {
      this.contentEl.nativeElement.removeEventListener('click', this.eprClickListener);
    }
    if (this.eprHoverListener && this.contentEl) {
      this.contentEl.nativeElement.removeEventListener('mouseover', this.eprHoverListener);
    }
    if (this.eprLeaveListener && this.contentEl) {
      this.contentEl.nativeElement.removeEventListener('mouseout', this.eprLeaveListener);
    }
  }

  toggleToc(): void {
    this.tocVisible = !this.tocVisible;
  }

  scrollToHeading(event: Event, id: string): void {
    event.preventDefault();
    const element = document.getElementById(id);
    if (element) {
      element.scrollIntoView({ behavior: 'smooth', block: 'start' });
      this.activeHeadingId = id;
      // Close TOC on mobile after clicking
      if (window.innerWidth < 1024) {
        this.tocVisible = false;
      }
    }
  }

  scrollToTop(): void {
    window.scrollTo({ top: 0, behavior: 'smooth' });
  }

  private async renderMarkdown(): Promise<void> {
    if (typeof this.node.content !== 'string') {
      console.warn('Markdown renderer expects string content');
      return;
    }

    this.destroyPopover();

    // Parse markdown to HTML
    let html = await this.marked.parse(this.node.content);

    // Post-process to transform any raw HTML img src attributes
    html = this.transformHtmlImageUrls(html);

    // Extract TOC and add heading IDs
    const { processedHtml, toc } = this.processHeadings(html);

    this.tocEntries = toc;
    // Security: HTML is generated from trusted markdown content stored in backend
    // eslint-disable-next-line sonarjs/no-angular-bypass-sanitization
    this.renderedContent = this.sanitizer.bypassSecurityTrustHtml(processedHtml);
    this.tocGenerated.emit(toc);

    // Update heading elements after view updates
    setTimeout(() => this.cacheHeadingElements(), 0);

    // Prefetch cross-path data for EPR links found in the markdown
    this.prefetchCrossPathData(this.node.content);
  }

  /**
   * Transform image src attributes in raw HTML to resolve blob URLs.
   * Handles <img src="/blob/..."> patterns that bypass marked's renderer.
   */
  private transformHtmlImageUrls(html: string): string {
    // Match img tags with src attributes containing blob paths
    // Using negated character class without + to avoid backtracking
    return html.replaceAll(
      /<img([^>]*?)\ssrc=["'](\/?blob\/[^"']*?)["']([^>]*?)>/gi,
      (_match, before, src, after) => {
        const resolvedSrc = this.resolveBlobUrl(src);
        return `<img${before} src="${resolvedSrc}"${after}>`;
      }
    );
  }

  private processHeadings(html: string): { processedHtml: string; toc: TocEntry[] } {
    const toc: TocEntry[] = [];
    const usedIds = new Set<string>();

    // Regex to find headings
    const headingRegex = /<h([1-6])([^>]*)>(.*?)<\/h\1>/gi;

    const processedHtml = html.replaceAll(headingRegex, (_match, level, attrs, content) => {
      // Strip HTML tags from content for TOC text
      // Use a helper function to avoid regex backtracking issues
      const text = this.stripHtmlTags(content).trim();

      // Generate unique ID
      const id = this.generateId(text);
      let uniqueId = id;
      let counter = 1;
      while (usedIds.has(uniqueId)) {
        uniqueId = `${id}-${counter}`;
        counter++;
      }
      usedIds.add(uniqueId);

      // Add to TOC
      toc.push({
        id: uniqueId,
        text,
        level: Number.parseInt(level, 10),
      });

      // Return heading with ID and anchor link
      return `<h${level}${attrs} id="${uniqueId}" class="heading-anchor">
        <a href="#${uniqueId}" class="anchor-link" aria-hidden="true">#</a>
        ${content}
      </h${level}>`;
    });

    return { processedHtml, toc };
  }

  /**
   * Strip HTML tags from a string without using regex backtracking.
   * Uses DOM parser for reliable tag stripping.
   */
  private stripHtmlTags(html: string): string {
    // Use DOM to safely strip HTML tags - avoids regex backtracking
    const doc = new DOMParser().parseFromString(html, 'text/html');
    return doc.body.textContent ?? '';
  }

  private generateId(text: string): string {
    return text
      .toLowerCase()
      .replaceAll(/[^\w\s-]/g, '')
      .replaceAll(/\s+/g, '-')
      .replaceAll(/-+/g, '-')
      .substring(0, 50);
  }

  /**
   * Set up event delegation for epr: link clicks.
   * Uses event delegation on the content element to catch clicks on
   * dynamically rendered [data-epr] anchors.
   */
  private setupEprClickHandler(): void {
    if (!this.contentEl) return;

    this.eprClickListener = (e: Event) => {
      const target = (e.target as HTMLElement).closest<HTMLAnchorElement>(EPR_ANCHOR_SELECTOR);
      if (!target) return;

      e.preventDefault();

      const eprUri = target.dataset['epr'];
      if (!eprUri) return;

      void this.resolveAndNavigate(eprUri);
    };

    this.contentEl.nativeElement.addEventListener('click', this.eprClickListener);
  }

  /**
   * Resolve an EPR URI with cross-path context and navigate.
   * Uses prefetched cross-path data when available, falls back to fetch.
   */
  private async resolveAndNavigate(eprUri: string): Promise<void> {
    const ctx = this.pathContext.currentContext;
    const currentPathId = ctx?.pathId ?? null;

    const ref = parseEpr(eprUri);
    let crossPathMatches = this.crossPathCache.get(ref.id);

    // Fall back to live fetch if not prefetched
    if (crossPathMatches === undefined && currentPathId) {
      crossPathMatches = await firstValueFrom(
        this.pathService.findContentInPaths(ref.id, currentPathId)
      );
    }

    const resolved = this.eprResolver.resolveInContext(
      eprUri,
      currentPathId,
      this.pathSteps,
      crossPathMatches
    );

    this.destroyPopover();
    void this.router.navigate(resolved.route);
  }

  /**
   * Scan markdown for epr: URIs and prefetch cross-path data for each.
   * This makes click resolution instant instead of waiting for an API call.
   */
  private prefetchCrossPathData(markdown: string): void {
    const ctx = this.pathContext.currentContext;
    const currentPathId = ctx?.pathId ?? null;
    if (!currentPathId) return;

    this.crossPathCache.clear();

    // Find all epr: URIs in the markdown source
    const eprPattern = /epr:([a-z0-9][\w-]*)/gi;
    const contentIds = new Set<string>();
    let match: RegExpExecArray | null;
    while ((match = eprPattern.exec(markdown)) !== null) {
      contentIds.add(match[1]);
    }

    // Prefetch cross-path matches for each unique content ID
    for (const contentId of contentIds) {
      this.pathService.findContentInPaths(contentId, currentPathId).subscribe(matches => {
        this.crossPathCache.set(contentId, matches);
      });
    }
  }

  /**
   * Set up hover-based popover for epr: links.
   * Uses mouseover/mouseout event delegation on the content element.
   * Shows EPR Head metadata popover after a short debounce delay.
   */
  private setupEprHoverHandler(): void {
    if (!this.contentEl) return;
    const el = this.contentEl.nativeElement;

    this.eprHoverListener = (e: Event) => {
      const anchor = (e.target as HTMLElement).closest<HTMLAnchorElement>(EPR_ANCHOR_SELECTOR);
      if (!anchor || anchor === this.hoveredEprAnchor) return;

      this.hoveredEprAnchor = anchor;

      // Cancel any pending dismiss
      if (this.popoverDismissTimer) {
        clearTimeout(this.popoverDismissTimer);
        this.popoverDismissTimer = null;
      }

      // Debounce show to avoid flashing on quick mouse movements
      if (this.popoverShowTimer) clearTimeout(this.popoverShowTimer);
      this.popoverShowTimer = setTimeout(() => {
        this.showPopover(anchor);
      }, 300);
    };

    this.eprLeaveListener = (e: Event) => {
      const me = e as MouseEvent;
      const anchor = (me.target as HTMLElement).closest<HTMLAnchorElement>(EPR_ANCHOR_SELECTOR);
      if (!anchor) return;

      // Ignore if moving to a child within the same anchor
      const related = me.relatedTarget as HTMLElement | null;
      if (related && anchor.contains(related)) return;

      this.hoveredEprAnchor = null;

      // Cancel pending show
      if (this.popoverShowTimer) {
        clearTimeout(this.popoverShowTimer);
        this.popoverShowTimer = null;
      }

      // Delay dismiss to allow mouse to reach the popover
      this.scheduleDismiss();
    };

    el.addEventListener('mouseover', this.eprHoverListener);
    el.addEventListener('mouseout', this.eprLeaveListener);
  }

  private showPopover(anchor: HTMLAnchorElement): void {
    const eprUri = anchor.dataset['epr'];
    if (!eprUri) return;

    // Clean up any existing popover
    this.destroyPopover();
    this.hoveredEprAnchor = anchor; // Restore — destroyPopover clears it

    // Get route synchronously for the "Open resource" link
    const { route } = this.eprResolver.resolveUrl(eprUri);

    // Fetch EPR Head metadata
    this.popoverSub = this.eprResolver.resolveEprHead(eprUri).subscribe(head => {
      if (!head) return;

      // Position below the anchor
      const rect = anchor.getBoundingClientRect();
      const position = { top: rect.bottom + 8, left: rect.left };

      // Create popover component programmatically
      const ref = this.vcr.createComponent(EprPopoverComponent);

      // Ensure host is at viewport origin for correct fixed positioning
      const hostEl = ref.location.nativeElement as HTMLElement;
      hostEl.style.top = '0';
      hostEl.style.left = '0';

      ref.instance.head = head;
      ref.instance.position = position;
      ref.instance.route = route;
      ref.instance.dismissed.subscribe(() => this.destroyPopover());

      // Cancel dismiss when mouse enters the popover
      hostEl.addEventListener('mouseenter', () => {
        if (this.popoverDismissTimer) {
          clearTimeout(this.popoverDismissTimer);
          this.popoverDismissTimer = null;
        }
      });

      this.activePopover = ref;
    });
  }

  private scheduleDismiss(): void {
    if (this.popoverDismissTimer) clearTimeout(this.popoverDismissTimer);
    this.popoverDismissTimer = setTimeout(() => {
      this.destroyPopover();
    }, 200);
  }

  private destroyPopover(): void {
    if (this.popoverShowTimer) {
      clearTimeout(this.popoverShowTimer);
      this.popoverShowTimer = null;
    }
    if (this.popoverDismissTimer) {
      clearTimeout(this.popoverDismissTimer);
      this.popoverDismissTimer = null;
    }
    if (this.popoverSub) {
      this.popoverSub.unsubscribe();
      this.popoverSub = null;
    }
    if (this.activePopover) {
      this.activePopover.destroy();
      this.activePopover = null;
    }
    this.hoveredEprAnchor = null;
  }

  private setupScrollListener(): void {
    this.scrollListener = () => {
      // Show/hide back to top button
      this.showBackToTop = window.scrollY > 300;

      // Update active heading in TOC
      this.updateActiveHeading();
    };

    window.addEventListener('scroll', this.scrollListener, { passive: true });
  }

  private cacheHeadingElements(): void {
    this.headingElements = this.tocEntries
      .map(entry => document.getElementById(entry.id))
      .filter((el): el is HTMLElement => el !== null);
  }

  private updateActiveHeading(): void {
    if (this.headingElements.length === 0) return;

    const scrollPos = window.scrollY + 100; // Offset for better UX

    // Find the heading closest to current scroll position
    for (let i = this.headingElements.length - 1; i >= 0; i--) {
      const el = this.headingElements[i];
      if (el.offsetTop <= scrollPos) {
        const newActiveId = el.id;
        if (this.activeHeadingId !== newActiveId) {
          this.activeHeadingId = newActiveId;
        }
        return;
      }
    }

    // If no heading is above scroll position, activate first one
    if (this.headingElements.length > 0) {
      this.activeHeadingId = this.headingElements[0].id;
    }
  }
}
