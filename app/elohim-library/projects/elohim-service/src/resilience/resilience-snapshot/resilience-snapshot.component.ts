import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  EventEmitter,
  Input,
  Output,
} from '@angular/core';

import 'elohim-core/register';

import { ResilienceSnapshotView } from '../../generated/resilience-snapshot-view';

import { ResilienceSnapshotDensity } from './resilience-snapshot.types';

import type { ContextMenuItem } from 'elohim-core';

/** The built-in "drill into the full card" action — flips the panel body in place. */
const VIEW_FULL_ACTION: ContextMenuItem = { id: 'view-full', label: 'View full resilience' };

@Component({
  selector: 'elohim-resilience-snapshot',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './resilience-snapshot.component.html',
  styleUrls: ['./resilience-snapshot.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class ResilienceSnapshotComponent {
  @Input({ required: true }) snapshot!: ResilienceSnapshotView;
  @Input() density: ResilienceSnapshotDensity = 'icon';

  /**
   * Host-injected extra actions, appended after the built-in "View full
   * resilience" item (derivation is host-side per pillar-EPR-decomposition
   * §7.4 — this component never invents destinations). Epic E items
   * ("Steward this content" / "View network") arrive here once their
   * destinations exist (spec §11.6).
   */
  @Input() actions: ContextMenuItem[] = [];

  /**
   * Re-emits every action selection id, the built-in `view-full` included —
   * mirrors elohim-epr-link's re-emit contract so a host can observe the full
   * intent stream while the component still owns the in-place density flip.
   */
  @Output() actionSelect = new EventEmitter<{ id: string }>();

  /** Whether the icon-density hypercard panel is folded down. */
  panelOpen = false;

  /** Which density body the panel renders; reset to 'context' on close. */
  panelDensity: 'context' | 'full' = 'context';

  get statusClass(): string {
    switch (this.snapshot?.protectionStatus) {
      case 'protected':
        return 'status-protected';
      case 'partial':
        return 'status-partial';
      case 'at-risk':
        return 'status-at-risk';
      default:
        return 'status-unknown';
    }
  }

  get regionSummary(): string {
    const rd = this.snapshot?.regionalDistribution;
    if (!rd) return '';
    const parts: string[] = [];
    if (rd.local) parts.push(`${rd.local} local`);
    if (rd.regional) parts.push(`${rd.regional} regional`);
    if (rd.global) parts.push(`${rd.global} global`);
    if (rd.unknown) parts.push(`${rd.unknown} unknown region`);
    return parts.join(' · ');
  }

  /**
   * Action row shown in the panel. The built-in flip action is present only
   * while the body is at context density (no point flipping to full once
   * you're already there); host actions always follow.
   */
  get panelActions(): ContextMenuItem[] {
    const builtIn = this.panelDensity === 'context' ? [VIEW_FULL_ACTION] : [];
    return [...builtIn, ...this.actions];
  }

  /** Icon click toggles the panel (icon density only). */
  togglePanel(): void {
    this.panelOpen = !this.panelOpen;
    if (this.panelOpen) this.panelDensity = 'context';
  }

  onPanelAction(event: Event): void {
    const id = (event as CustomEvent<{ id: string }>).detail?.id;
    if (!id) return;
    if (id === 'view-full') {
      // In-place HyperCard flip — no full-resilience route exists or is needed.
      this.panelDensity = 'full';
    }
    // Re-emit every id (view-full included) so the host sees the full stream.
    this.actionSelect.emit({ id });
  }

  onPanelClose(): void {
    this.panelOpen = false;
    this.panelDensity = 'context';
  }
}
