import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input, OnInit, signal } from '@angular/core';

/**
 * Protocol signal badge — DOM tier (Tier 1).
 *
 * Renders a fixed-position corner badge announcing that the displayed content
 * is sourced from the Elohim Protocol. Click to expand a provenance panel
 * (CID, author, attestations) — analogous to clicking the HTTPS padlock.
 *
 * TIER PROGRESSION:
 * - Tier 1 (DOM): this component. Lives in DOM; honest about being doorway-asserted.
 * - Tier 2 (Extension): TODO — browser extension verifies X-Elohim-Content-CID header
 *   client-side and decorates browser toolbar; sets `window.__elohimExtensionTakeover = true`.
 * - Tier 3 (Tauri-native): TODO — Tauri shell decorates OS window chrome; sets `window.__TAURI__`.
 *
 * This component suppresses itself when a higher tier takes over.
 */
@Component({
  selector: 'app-protocol-signal-badge',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './protocol-signal-badge.component.html',
  styleUrls: ['./protocol-signal-badge.component.css'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProtocolSignalBadgeComponent implements OnInit {
  @Input({ required: true }) contentId!: string;
  @Input() authorDisplay: string | null = null;
  @Input() attestationCount = 0;

  readonly suppressed = signal(false);
  readonly expanded = signal(false);

  ngOnInit(): void {
    const w = globalThis as Record<string, unknown>;
    if (w['__TAURI__'] !== undefined || w['__elohimExtensionTakeover'] === true) {
      this.suppressed.set(true);
    }
  }

  togglePanel(): void {
    this.expanded.update(v => !v);
  }

  shortCid(): string {
    if (this.contentId.length <= 20) return this.contentId;
    return `${this.contentId.slice(0, 6)}…${this.contentId.slice(-6)}`;
  }
}
