import { NgIf } from '@angular/common';
import { ChangeDetectionStrategy, Component, OnInit, computed, input, signal } from '@angular/core';

/**
 * Protocol signal badge — DOM tier (Tier 1).
 *
 * Renders a fixed-position corner badge announcing that the displayed content
 * is sourced from the Elohim Protocol. Click to expand a provenance panel
 * (CID, author, attestations) — analogous to clicking the HTTPS padlock.
 *
 * TIER PROGRESSION:
 * - Tier 1 (DOM): this component. Lives in DOM; honest about being doorway-asserted.
 * - Tier 2 (Extension): Planned — browser extension verifies X-Elohim-Content-CID header
 *   client-side and decorates browser toolbar; sets `window.__elohimExtensionTakeover = true`.
 * - Tier 3 (Tauri-native): Planned — Tauri shell decorates OS window chrome; sets `window.__TAURI__`.
 *
 * This component suppresses itself when a higher tier takes over.
 */
@Component({
  selector: 'app-protocol-signal-badge',
  standalone: true,
  imports: [NgIf],
  templateUrl: './protocol-signal-badge.component.html',
  styleUrls: ['./protocol-signal-badge.component.css'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProtocolSignalBadgeComponent implements OnInit {
  readonly contentId = input.required<string>();
  readonly authorDisplay = input<string | null>(null);
  readonly attestationCount = input<number>(0);

  readonly suppressed = signal(false);
  readonly expanded = signal(false);

  readonly shortCid = computed(() => {
    const id = this.contentId();
    if (id.length <= 20) return id;
    return `${id.slice(0, 6)}…${id.slice(-6)}`;
  });

  ngOnInit(): void {
    const w = globalThis as Record<string, unknown>;
    const tauriPresent = typeof w['__TAURI__'] === 'object' && w['__TAURI__'] !== null;
    const extensionPresent = w['__elohimExtensionTakeover'] === true;
    if (tauriPresent || extensionPresent) {
      this.suppressed.set(true);
    }
  }

  togglePanel(): void {
    this.expanded.update(v => !v);
  }
}
