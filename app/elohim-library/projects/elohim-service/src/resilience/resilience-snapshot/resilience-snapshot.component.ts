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

import type { ContentDocFields } from '../../angular/services/content-doc-sync.service';
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
   * Inline-axis pin for the icon-density fold-downs (hypercard panel +
   * hover tooltip). The HOST knows where its chrome sits; anchors in
   * end-side chrome (e.g. the protocol-omni toolbar) set 'end' so the
   * surfaces grow INTO the viewport instead of off its edge — the
   * 2026-06-12 phone regression (panel projected ~173px off a 390px
   * viewport). Forwarded to the panel's align attribute (spec §11.2).
   */
  @Input() panelAlign: 'start' | 'end' = 'start';

  /**
   * Doc-sync sibling signal (Automerge content-sync plane). DELIBERATELY a
   * separate signal from the custody-resilience verdict above — they fold
   * different planes and genuinely diverge (a doc converged on ONE peer is not
   * custody-resilient; a doc behind on heads can still be RS-distributed to many
   * households). See resilience-facings spec §12: doc-sync is a CLIENT-COMPOSED
   * sibling, never folded into the resilience `<dl>` verdict. The host supplies
   * the per-content `node:{id}` doc fields (or null while pending) — this
   * component never reaches the sync plane itself.
   *
   * Tri-state: `undefined` = host did not wire it; `null` = watching, no doc
   * yet (pending); a fields object = doc present (synced). The first two render
   * honestly as "not yet synced" — never a fake "synced" (mirrors `isUnmeasured`).
   */
  @Input() docSync?: ContentDocFields | null;

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

  /**
   * Unmeasured ≠ zero (2026-06-12): content that never entered the
   * distribution plane reports `distributionState: 'unmeasured'` — every
   * count is a non-measurement. Render a distinct neutral state, never a
   * fake at-risk verdict.
   */
  get isUnmeasured(): boolean {
    return this.snapshot?.distributionState === 'unmeasured';
  }

  get statusClass(): string {
    if (this.isUnmeasured) return 'status-unmeasured';
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

  /** Human status text — 'not yet distributed' for unmeasured content. */
  get statusLabel(): string {
    return this.isUnmeasured ? 'not yet distributed' : (this.snapshot?.protectionStatus ?? '');
  }

  /** "live/known peers live" pair, e.g. "2/3" — honest denominators. */
  get peersLiveSummary(): string {
    const peers = this.snapshot?.details?.onlinePeers;
    if (!peers) return '';
    return `${peers.live}/${peers.known} peers live`;
  }

  /**
   * Doc-sync sibling reading. Honest-by-construction (spec §12): a present doc
   * reads "synced" (+ its `updatedAt` document version when known); a pending
   * (null) or unwired (undefined) doc reads "not yet synced" — never a fake
   * "synced". This is per-content presence, NOT a cross-peer converged count
   * (that lives only in the unexposed StreamTracker).
   */
  get docSyncSummary(): string {
    const doc = this.docSync;
    if (!doc) return 'not yet synced';
    return doc.updatedAt ? `synced · ${doc.updatedAt}` : 'synced';
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
