import { NgIf } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  OnInit,
  computed,
  inject,
  input,
  signal,
} from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import { ProtocolNavigationService } from '@app/elohim/services/protocol-navigation.service';

/**
 * ProtocolOmniComponent — DOM-tier protocol chrome.
 *
 * A chip at the top of the viewport announcing protocol provenance.
 * Click expands a full-width toolbar with context-aware affordances:
 *   - EPR identifier (CID, click-to-copy)
 *   - Resilience indicator (placeholder; real wiring is a follow-up)
 *   - In-network back/forward (gated on ProtocolNavigationService —
 *     substrate-derived for cold visitors, session-derived for
 *     walked-from-protocol visitors)
 *   - Account link (gated on the `authenticated` input)
 *
 * Theme: matches OS prefers-color-scheme via CSS variables. System font
 * stack. Restrained palette so the bar reads as protocol chrome, not page
 * content.
 *
 * Tier progression:
 *   - Tier 1 (DOM): this component.
 *   - Tier 2 (Extension): Planned — browser extension owns the toolbar
 *     in browser chrome; sets window.__elohimExtensionTakeover = true.
 *   - Tier 3 (Tauri-native): Planned — Tauri shell decorates OS window
 *     chrome; window.__TAURI__ is the injected object.
 * The component suppresses itself entirely when a higher tier owns the
 * chrome.
 */
@Component({
  selector: 'app-protocol-omni',
  standalone: true,
  imports: [NgIf, RouterLink],
  templateUrl: './protocol-omni.component.html',
  styleUrls: ['./protocol-omni.component.css'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProtocolOmniComponent implements OnInit {
  readonly contentId = input.required<string>();
  readonly authorDisplay = input<string | null>(null);
  readonly authenticated = input<boolean>(false);
  readonly accountHref = input<string>('/account');

  readonly suppressed = signal(false);
  readonly expanded = signal(false);

  private readonly nav = inject(ProtocolNavigationService);
  private readonly router = inject(Router);

  readonly back = computed(() => this.nav.back());
  readonly forward = computed(() => this.nav.forward());

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
      return;
    }
    void this.nav.activate(this.contentId(), this.router.url);
  }

  toggleExpanded(): void {
    this.expanded.update(v => !v);
  }

  collapse(): void {
    this.expanded.set(false);
  }

  navigateBack(): void {
    const target = this.back();
    if (target) void this.router.navigate(['/resource', target.cid]);
  }

  navigateForward(): void {
    const target = this.forward();
    if (target) void this.router.navigate(['/resource', target.cid]);
  }

  copyCid(): void {
    void navigator.clipboard?.writeText(this.contentId());
  }
}
