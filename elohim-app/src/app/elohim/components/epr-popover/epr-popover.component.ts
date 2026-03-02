/**
 * EprPopoverComponent — Three-pillar metadata popover for EPR links.
 *
 * Shows a lightweight popover with EPR Head metadata when hovering
 * an epr: link. Displays pillar-specific context:
 *
 * - Lamad: title, content type, format, tags, description
 * - Shefa: stewards
 * - Qahal: reach, constitutional layer
 *
 * Usage (programmatic — created by EprLinkComponent on hover):
 *   const ref = viewContainerRef.createComponent(EprPopoverComponent);
 *   ref.instance.head = eprHead;
 *   ref.instance.position = { top: 100, left: 200 };
 */

import { CommonModule } from '@angular/common';
import {
  Component,
  ChangeDetectionStrategy,
  Input,
  HostListener,
  EventEmitter,
  Output,
} from '@angular/core';
import { RouterModule } from '@angular/router';

import type { EprHead } from '../../models/epr-head.model';

@Component({
  selector: 'app-epr-popover',
  standalone: true,
  imports: [CommonModule, RouterModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div
      *ngIf="head"
      class="epr-popover"
      [style.top.px]="position.top"
      [style.left.px]="position.left"
      data-testid="epr-popover"
    >
      <!-- Header: content type badge + title -->
      <div class="epr-popover-header">
        <span class="epr-popover-type" data-testid="epr-popover-type">
          {{ head.lamad.contentType }}
        </span>
        <span
          *ngIf="head.lamad.contentFormat"
          class="epr-popover-format"
          data-testid="epr-popover-format"
        >
          {{ head.lamad.contentFormat }}
        </span>
      </div>

      <h4 class="epr-popover-title" data-testid="epr-popover-title">
        {{ head.lamad.title }}
      </h4>

      <p *ngIf="head.lamad.description" class="epr-popover-desc" data-testid="epr-popover-desc">
        {{ head.lamad.description }}
      </p>

      <!-- Lamad: tags -->
      <div *ngIf="head.lamad.tags?.length" class="epr-popover-tags" data-testid="epr-popover-tags">
        <span *ngFor="let tag of head.lamad.tags" class="epr-popover-tag">
          {{ tag }}
        </span>
      </div>

      <!-- Shefa: stewards -->
      <div
        *ngIf="head.shefa?.stewards?.length"
        class="epr-popover-section"
        data-testid="epr-popover-shefa"
      >
        <span class="epr-popover-label">Stewards</span>
        <span class="epr-popover-value">
          {{ head.shefa.stewards!.length }}
        </span>
      </div>

      <!-- Qahal: reach + layer -->
      <div
        *ngIf="head.qahal?.reach || head.qahal?.layer"
        class="epr-popover-section"
        data-testid="epr-popover-qahal"
      >
        <span *ngIf="head.qahal.reach" class="epr-popover-reach">
          {{ head.qahal.reach }}
        </span>
        <span *ngIf="head.qahal.layer" class="epr-popover-layer">
          {{ head.qahal.layer }}
        </span>
      </div>

      <!-- Footer: link to full resource -->
      <a *ngIf="route" [routerLink]="route" class="epr-popover-link" data-testid="epr-popover-link">
        Open resource
      </a>
    </div>
  `,
  styles: [
    `
      :host {
        position: fixed;
        z-index: 1000;
        pointer-events: auto;
      }

      .epr-popover {
        position: absolute;
        width: 280px;
        padding: 12px;
        background: var(--epr-popover-bg, #ffffff);
        color: var(--epr-popover-color, #1e293b);
        border: 1px solid var(--epr-popover-border, #e5e7eb);
        border-radius: 8px;
        box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
        font-size: 0.85rem;
        line-height: 1.4;
      }

      .epr-popover-header {
        display: flex;
        align-items: center;
        gap: 6px;
        margin-bottom: 6px;
      }

      .epr-popover-type {
        display: inline-block;
        padding: 1px 6px;
        border-radius: 4px;
        background: var(--epr-type-bg, #e0f2fe);
        color: var(--epr-type-color, #0369a1);
        font-size: 0.75em;
        text-transform: uppercase;
        letter-spacing: 0.04em;
        font-weight: 600;
      }

      .epr-popover-format {
        font-size: 0.75em;
        opacity: 0.5;
      }

      .epr-popover-title {
        margin: 0 0 4px;
        font-size: 0.95em;
        font-weight: 600;
      }

      .epr-popover-desc {
        margin: 0 0 8px;
        font-size: 0.8em;
        opacity: 0.75;
        display: -webkit-box;
        -webkit-line-clamp: 2;
        -webkit-box-orient: vertical;
        overflow: hidden;
      }

      .epr-popover-tags {
        display: flex;
        flex-wrap: wrap;
        gap: 4px;
        margin-bottom: 8px;
      }

      .epr-popover-tag {
        padding: 1px 6px;
        border-radius: 10px;
        background: var(--epr-tag-bg, #f3f4f6);
        font-size: 0.75em;
        opacity: 0.8;
      }

      .epr-popover-section {
        display: flex;
        align-items: center;
        gap: 6px;
        margin-bottom: 4px;
        font-size: 0.8em;
      }

      .epr-popover-label {
        opacity: 0.5;
      }

      .epr-popover-value {
        font-weight: 500;
      }

      .epr-popover-reach,
      .epr-popover-layer {
        padding: 1px 6px;
        border-radius: 4px;
        font-size: 0.75em;
      }

      .epr-popover-reach {
        background: var(--epr-reach-bg, #ecfdf5);
        color: var(--epr-reach-color, #047857);
      }

      .epr-popover-layer {
        background: var(--epr-layer-bg, #fef3c7);
        color: var(--epr-layer-color, #92400e);
      }

      .epr-popover-link {
        display: block;
        margin-top: 8px;
        padding-top: 8px;
        border-top: 1px solid var(--epr-popover-border, #e5e7eb);
        color: var(--epr-link-color, #2563eb);
        font-size: 0.8em;
        text-decoration: none;
      }
      .epr-popover-link:hover {
        text-decoration: underline;
      }
    `,
  ],
})
export class EprPopoverComponent {
  /** EPR Head metadata to display */
  @Input() head: EprHead | null = null;

  /** Position relative to the viewport */
  @Input() position: { top: number; left: number } = { top: 0, left: 0 };

  /** Angular route for the "Open resource" link */
  @Input() route: string[] | null = null;

  /** Emitted when the mouse leaves the popover */
  @Output() dismissed = new EventEmitter<void>();

  @HostListener('mouseleave')
  onMouseLeave(): void {
    this.dismissed.emit();
  }
}
