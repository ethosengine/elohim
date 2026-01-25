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
  OnDestroy,
  inject,
} from '@angular/core';
import { DomSanitizer, SafeHtml } from '@angular/platform-browser';

import hljs from 'highlight.js';
import { Marked } from 'marked';
import { markedHighlight } from 'marked-highlight';

import { StorageClientService } from '@app/elohim/services/storage-client.service';

import { ContentNode } from '../../models/content-node.model';

/**
 * Table of Contents entry extracted from markdown headings.
 */
export interface TocEntry {
  id: string;
  text: string;
  level: number;
}

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
      >
        <span class="toc-icon">&#9776;</span>
      </button>

      <!-- TOC Backdrop - click to dismiss -->
      <div *ngIf="tocVisible" class="toc-backdrop" (click)="toggleToc()"></div>

      <!-- Table of Contents Sidebar -->
      <nav *ngIf="tocEntries.length > 0" class="toc-sidebar" [class.toc-visible]="tocVisible">
        <div class="toc-header">
          <span>Contents</span>
          <button class="toc-close" (click)="toggleToc()">&times;</button>
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
      <button *ngIf="showBackToTop" class="back-to-top" (click)="scrollToTop()" title="Back to top">
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
  @Output() tocGenerated = new EventEmitter<TocEntry[]>();

  @ViewChild('contentEl') contentEl!: ElementRef<HTMLElement>;

  renderedContent: SafeHtml = '';
  tocEntries: TocEntry[] = [];
  tocVisible = false;
  activeHeadingId = '';
  showBackToTop = false;

  private readonly marked: Marked;
  private scrollListener?: () => void;
  private headingElements: HTMLElement[] = [];
  private readonly storageClient = inject(StorageClientService);

  constructor(private readonly sanitizer: DomSanitizer) {
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

    // Configure custom renderer to transform blob URLs in images
    const self = this; // Capture reference for closure
    this.marked.use({
      renderer: {
        image(token) {
          // Transform blob URLs to full doorway URLs
          const resolvedHref = self.resolveBlobUrl(token.href);
          const title = token.title ? ` title="${token.title}"` : '';
          return `<img src="${resolvedHref}" alt="${token.text}"${title}>`;
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
      this.renderMarkdown();
    }
  }

  ngAfterViewInit(): void {
    this.setupScrollListener();
  }

  ngOnDestroy(): void {
    if (this.scrollListener) {
      window.removeEventListener('scroll', this.scrollListener);
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

    // Parse markdown to HTML
    let html = await this.marked.parse(this.node.content);

    // Post-process to transform any raw HTML img src attributes
    html = this.transformHtmlImageUrls(html);

    // Extract TOC and add heading IDs
    const { processedHtml, toc } = this.processHeadings(html);

    this.tocEntries = toc;
    this.renderedContent = this.sanitizer.bypassSecurityTrustHtml(processedHtml);
    this.tocGenerated.emit(toc);

    // Update heading elements after view updates
    setTimeout(() => this.cacheHeadingElements(), 0);
  }

  /**
   * Transform image src attributes in raw HTML to resolve blob URLs.
   * Handles <img src="/blob/..."> patterns that bypass marked's renderer.
   */
  private transformHtmlImageUrls(html: string): string {
    // Match img tags with src attributes containing blob paths
    return html.replace(
      /<img([^>]*)\ssrc=["'](\/?blob\/[^"']+)["']([^>]*)>/gi,
      (match, before, src, after) => {
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

    const processedHtml = html.replace(headingRegex, (match, level, attrs, content) => {
      // Strip HTML tags from content for TOC text
      const text = content.replace(/<[^>]+>/g, '').trim();

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
        level: parseInt(level, 10),
      });

      // Return heading with ID and anchor link
      return `<h${level}${attrs} id="${uniqueId}" class="heading-anchor">
        <a href="#${uniqueId}" class="anchor-link" aria-hidden="true">#</a>
        ${content}
      </h${level}>`;
    });

    return { processedHtml, toc };
  }

  private generateId(text: string): string {
    return text
      .toLowerCase()
      .replace(/[^\w\s-]/g, '')
      .replace(/\s+/g, '-')
      .replace(/-+/g, '-')
      .substring(0, 50);
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
