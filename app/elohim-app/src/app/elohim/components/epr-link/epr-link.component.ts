/**
 * EprLinkComponent — The `<epr-link>` element.
 *
 * Drop-in component that resolves an epr: URI and renders a navigable
 * content reference. Handles the full resolution chain:
 *
 *   epr:manifesto-foundations
 *     → EprResolverService.resolve()
 *       → StorageClientService.getContent()
 *         → { title, description, contentType, blobHash, reach, ... }
 *     → routerLink to /lamad/resource/manifesto-foundations
 *
 * Usage:
 *   <epr-link epr="epr:manifesto-foundations"></epr-link>
 *   <epr-link epr="epr:elohim-protocol#step/2" display="card"></epr-link>
 *   <epr-link epr="epr:systems-thinking" display="inline">custom text</epr-link>
 *
 * See: protocol-specification.md Appendix E
 */

import { CommonModule } from '@angular/common';
import {
  Component,
  ChangeDetectionStrategy,
  ComponentRef,
  ElementRef,
  HostListener,
  Input,
  OnChanges,
  SimpleChanges,
  ViewContainerRef,
  inject,
  ChangeDetectorRef,
  OnDestroy,
} from '@angular/core';
import { RouterModule } from '@angular/router';

import { Subject, takeUntil } from 'rxjs';

import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';
import { formatEpr, parseEpr } from '../../utils/epr-ref';
import { EprPopoverComponent } from '../epr-popover/epr-popover.component';

import type { EprHead } from '../../models/epr-head.model';

export type EprLinkDisplay = 'inline' | 'card' | 'chip';

@Component({
  selector: 'app-epr-link',
  standalone: true,
  imports: [CommonModule, RouterModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <!-- Inline: renders as a styled routerLink -->
    <a
      *ngIf="display === 'inline' && resolved"
      [routerLink]="resolved.route"
      class="epr-link-inline"
      data-testid="epr-link-inline"
      [attr.data-epr]="eprUri"
      [attr.data-content-type]="resolved.content.contentType"
    >
      <ng-content>{{ resolved.content.title }}</ng-content>
    </a>

    <!-- Chip: compact pill with type icon + title -->
    <a
      *ngIf="display === 'chip' && resolved"
      [routerLink]="resolved.route"
      class="epr-link-chip"
      data-testid="epr-link-chip"
      [attr.data-epr]="eprUri"
    >
      <span class="epr-chip-type">{{ resolved.content.contentType }}</span>
      <span class="epr-chip-title">{{ resolved.content.title }}</span>
    </a>

    <!-- Card: preview card with thumbnail, title, description, pillar context -->
    <a
      *ngIf="display === 'card' && resolved"
      [routerLink]="resolved.route"
      class="epr-link-card"
      data-testid="epr-link-card"
      [attr.data-epr]="eprUri"
    >
      <img
        *ngIf="resolved.blobUrl"
        [src]="resolved.blobUrl"
        [alt]="resolved.content.title"
        class="epr-card-thumbnail"
        loading="lazy"
      />
      <div class="epr-card-body">
        <span class="epr-card-type">{{ resolved.content.contentType }}</span>
        <h3 class="epr-card-title">{{ resolved.content.title }}</h3>
        <p *ngIf="resolved.content.description" class="epr-card-desc">
          {{ resolved.content.description }}
        </p>
        <div class="epr-card-meta">
          <span *ngIf="resolved.content.reach" class="epr-card-reach">
            {{ resolved.content.reach }}
          </span>
          <span *ngIf="resolved.content.tags?.length" class="epr-card-tags">
            {{ resolved.content.tags.slice(0, 3).join(' / ') }}
          </span>
        </div>
      </div>
    </a>

    <!-- Loading state -->
    <span *ngIf="!resolved && !error" class="epr-link-loading">...</span>

    <!-- Error state: show the raw EPR URI as fallback -->
    <span *ngIf="error" class="epr-link-error" [title]="error">{{ eprUri }}</span>
  `,
  styles: [
    `
      :host {
        display: inline;
      }

      .epr-link-inline {
        color: var(--epr-link-color, #2563eb);
        text-decoration: underline;
        text-decoration-style: dotted;
        text-underline-offset: 2px;
        cursor: pointer;
      }
      .epr-link-inline:hover {
        text-decoration-style: solid;
      }

      .epr-link-chip {
        display: inline-flex;
        align-items: center;
        gap: 4px;
        padding: 2px 8px;
        border-radius: 12px;
        border: 1px solid var(--epr-chip-border, #d1d5db);
        background: var(--epr-chip-bg, #f9fafb);
        font-size: 0.85em;
        text-decoration: none;
        color: inherit;
        cursor: pointer;
      }
      .epr-link-chip:hover {
        background: var(--epr-chip-hover-bg, #f3f4f6);
      }
      .epr-chip-type {
        opacity: 0.6;
        font-size: 0.85em;
      }

      .epr-link-card {
        display: flex;
        flex-direction: column;
        border: 1px solid var(--epr-card-border, #e5e7eb);
        border-radius: 8px;
        overflow: hidden;
        text-decoration: none;
        color: inherit;
        cursor: pointer;
        transition: box-shadow 0.15s;
      }
      .epr-link-card:hover {
        box-shadow: 0 2px 8px rgba(0, 0, 0, 0.1);
      }
      .epr-card-thumbnail {
        width: 100%;
        height: 140px;
        object-fit: cover;
      }
      .epr-card-body {
        padding: 12px;
      }
      .epr-card-type {
        display: inline-block;
        font-size: 0.75em;
        text-transform: uppercase;
        letter-spacing: 0.05em;
        opacity: 0.6;
        margin-bottom: 4px;
      }
      .epr-card-title {
        margin: 0 0 4px;
        font-size: 1em;
        font-weight: 600;
      }
      .epr-card-desc {
        margin: 0 0 8px;
        font-size: 0.85em;
        opacity: 0.8;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
      }
      .epr-card-meta {
        display: flex;
        gap: 8px;
        font-size: 0.75em;
        opacity: 0.6;
      }

      .epr-link-loading {
        opacity: 0.4;
      }
      .epr-link-error {
        opacity: 0.5;
        font-style: italic;
      }
    `,
  ],
})
export class EprLinkComponent implements OnChanges, OnDestroy {
  /** The epr: URI to resolve. Accepts epr:, did:web:, or bare ID. */
  @Input({ required: true }) epr!: string;

  /** Display mode: inline link, compact chip, or preview card. */
  @Input() display: EprLinkDisplay = 'inline';

  resolved: ResolvedContent | null = null;
  error: string | null = null;
  eprUri = '';

  private readonly resolver = inject(EprResolverService);
  private readonly cdr = inject(ChangeDetectorRef);
  private readonly elRef = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly vcRef = inject(ViewContainerRef);
  private readonly destroy$ = new Subject<void>();

  private popoverRef: ComponentRef<EprPopoverComponent> | null = null;
  private hoverTimer: ReturnType<typeof setTimeout> | null = null;
  private dismissTimer: ReturnType<typeof setTimeout> | null = null;
  private eprHead: EprHead | null = null;

  @HostListener('mouseenter', ['$event'])
  onMouseEnter(event: MouseEvent): void {
    // Debounce: wait 300ms before showing popover
    this.hoverTimer = setTimeout(() => {
      this.showPopover(event);
    }, 300);
  }

  @HostListener('mouseleave')
  onMouseLeave(): void {
    if (this.hoverTimer) {
      clearTimeout(this.hoverTimer);
      this.hoverTimer = null;
    }
    // Delay dismiss to allow mouse to enter popover
    this.dismissTimer = setTimeout(() => {
      this.dismissPopover();
    }, 150);
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['epr']) {
      this.loadEpr(this.epr);
    }
  }

  ngOnDestroy(): void {
    this.dismissPopover();
    this.destroy$.next();
    this.destroy$.complete();
  }

  private showPopover(event: MouseEvent): void {
    if (this.popoverRef) return;

    // Lazy-load EPR Head if not cached
    if (!this.eprHead) {
      this.resolver
        .resolveEprHead(this.epr)
        .pipe(takeUntil(this.destroy$))
        .subscribe(head => {
          if (head) {
            this.eprHead = head;
            this.createPopover(event);
          }
        });
      return;
    }

    this.createPopover(event);
  }

  private createPopover(_event: MouseEvent): void {
    if (this.popoverRef || !this.eprHead) return;

    this.popoverRef = this.vcRef.createComponent(EprPopoverComponent);
    this.popoverRef.instance.head = this.eprHead;
    this.popoverRef.instance.route = this.resolved?.route ?? null;

    // Position below the link (applied to :host via HostBinding)
    const rect = this.elRef.nativeElement.getBoundingClientRect();
    this.popoverRef.instance.position = {
      top: rect.bottom + 4,
      left: Math.max(8, rect.left),
    };

    // Cancel dismiss when mouse enters the popover
    this.popoverRef.instance.entered.subscribe(() => {
      if (this.dismissTimer) {
        clearTimeout(this.dismissTimer);
        this.dismissTimer = null;
      }
    });

    this.popoverRef.instance.dismissed.subscribe(() => {
      this.dismissPopover();
    });

    this.popoverRef.changeDetectorRef.markForCheck();
  }

  private dismissPopover(): void {
    if (this.dismissTimer) {
      clearTimeout(this.dismissTimer);
      this.dismissTimer = null;
    }
    if (this.popoverRef) {
      this.popoverRef.destroy();
      this.popoverRef = null;
    }
  }

  private loadEpr(input: string): void {
    this.resolved = null;
    this.error = null;
    this.eprHead = null;

    const ref = parseEpr(input);
    this.eprUri = formatEpr(ref);

    this.resolver
      .resolve(input)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: result => {
          if (result) {
            this.resolved = result;
          } else {
            this.error = `Not found: ${this.eprUri}`;
          }
          this.cdr.markForCheck();
        },
        error: err => {
          this.error = `Failed to resolve: ${this.eprUri}`;
          console.warn('[epr-link] Resolution failed:', this.eprUri, err);
          this.cdr.markForCheck();
        },
      });
  }
}
