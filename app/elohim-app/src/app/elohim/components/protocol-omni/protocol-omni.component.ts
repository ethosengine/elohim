import { NgIf } from '@angular/common';
import {
  ChangeDetectionStrategy,
  CUSTOM_ELEMENTS_SCHEMA,
  Component,
  OnInit,
  computed,
  inject,
  input,
  signal,
} from '@angular/core';
import { Router, RouterLink } from '@angular/router';

import 'elohim-core/register';

import { ProtocolNavigationService } from '@app/elohim/services/protocol-navigation.service';
import { ConfigService } from '../../../services/config.service';
import type { ServingContext } from '../../models/serving-context.model';

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
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class ProtocolOmniComponent implements OnInit {
  readonly contentId = input.required<string>();
  readonly authorDisplay = input<string | null>(null);
  readonly authenticated = input<boolean>(false);
  readonly accountHref = input<string>('/account');
  /** Opt-in: render the serving-context segment (spec §5). Default off — the trust surface never cries wolf. */
  readonly showEnvContext = input<boolean>(false);
  /** Opt-in: render <elohim-theme-toggle> in the expanded toolbar. Default off. */
  readonly showThemeToggle = input<boolean>(false);

  readonly suppressed = signal(false);
  readonly expanded = signal(false);

  private readonly nav = inject(ProtocolNavigationService);
  private readonly router = inject(Router);
  private readonly configService = inject(ConfigService);

  readonly back = computed(() => this.nav.back());
  readonly forward = computed(() => this.nav.forward());

  readonly servingContext = signal<ServingContext | null>(null);

  readonly envVisible = computed(() => {
    const ctx = this.servingContext();
    return this.showEnvContext() && !!ctx?.tier && ctx.tier !== 'production';
  });

  readonly shortBuildId = computed(() => {
    const id = this.servingContext()?.buildId;
    return id ? id.slice(0, 7) : '';
  });

  readonly envLabel = computed(() => {
    const ctx = this.servingContext();
    if (!ctx) return '';
    const facets = [`You're viewing this EPR through the ${ctx.tier} environment`];
    if (this.shortBuildId()) facets.push(`build ${this.shortBuildId()}`);
    if (ctx.logLevel) facets.push(`log: ${ctx.logLevel}`);
    return `${facets.join(' · ')} — backend details`;
  });

  readonly shortCid = computed(() => {
    const id = this.contentId();
    if (id.length <= 20) return id;
    return `${id.slice(0, 6)}…${id.slice(-6)}`;
  });

  ngOnInit(): void {
    this.configService.getConfig().subscribe(cfg => {
      const buildId =
        cfg.gitHash && cfg.gitHash !== 'GIT_HASH_PLACEHOLDER' && cfg.gitHash !== 'local-dev'
          ? cfg.gitHash
          : undefined;
      this.servingContext.set({
        tier: cfg.environment as ServingContext['tier'],
        logLevel: cfg.logLevel,
        buildId,
      });
    });

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
