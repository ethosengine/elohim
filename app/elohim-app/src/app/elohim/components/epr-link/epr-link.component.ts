/**
 * EprLinkComponent — thin Angular wrapper around <elohim-epr-link>.
 *
 * Bridges the Lit web component into the Angular component tree:
 *   - Passes epr / display as reflected attributes
 *   - Injects EprResolverService as the Lit element's resolver function
 *   - Translates the 'navigate' CustomEvent into Angular Router navigation
 *
 * Previously this component owned full EPR resolution logic (resolution chain,
 * popover management, hover state, display-mode rendering). That logic now
 * lives in <elohim-epr-link> (elohim-core). This wrapper is the transitional
 * bridge for any monolith Angular templates still using <app-epr-link>.
 *
 * Usage (unchanged for consumers):
 *   <app-epr-link epr="epr:manifesto-foundations"></app-epr-link>
 *   <app-epr-link epr="epr:elohim-protocol" display="card"></app-epr-link>
 *
 * See: protocol-specification.md Appendix E
 */

import {
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  ElementRef,
  EventEmitter,
  Input,
  OnDestroy,
  OnInit,
  Output,
  inject,
  signal,
} from '@angular/core';
import { type NavigationExtras, Router } from '@angular/router';

import { type Subscription, firstValueFrom } from 'rxjs';

import 'elohim-core/register';

import { type ContextMenuAction } from '@app/qahal';

import { AcquisitionService, type PullStatusInfo } from '../../services/acquisition.service';
import { EprNavService } from '../../services/epr-nav.service';
import { EprResolverService, type ResolvedContent } from '../../services/epr-resolver.service';
import { PinProgressComponent } from '../pin-progress/pin-progress.component';

import type { ContextMenuItem, ElohimEprLink, EprLinkDisplay } from 'elohim-core';

export type { EprLinkDisplay } from 'elohim-core';

@Component({
  selector: 'app-epr-link',
  standalone: true,
  imports: [PinProgressComponent],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  template: `
    <elohim-epr-link [attr.epr]="epr" [attr.display]="display"></elohim-epr-link>
    @if (activePeerPin() !== null) {
      <app-pin-progress
        data-testid="epr-link-pin-progress"
        [total]="peerPinStatus()?.total ?? null"
        [fetched]="peerPinStatus()?.fetched ?? 0"
        [pending]="peerPinStatus()?.pending ?? 0"
        [failed]="peerPinStatus()?.failed ?? 0"
        [caughtUp]="peerPinStatus()?.caughtUp ?? null"
        (cancel)="cancelPeerPin()"
        (retry)="retryPeerPin()"
      ></app-pin-progress>
    }
  `,
  styles: [
    `
      :host {
        display: inline;
      }
    `,
  ],
})
export class EprLinkComponent implements OnInit, OnDestroy {
  /** The epr: URI to resolve. Accepts epr:, did:web:, or bare ID. */
  @Input({ required: true }) epr!: string;

  /** Display mode: inline link, compact chip, preview card, or popover. */
  @Input() display: EprLinkDisplay = 'inline';

  /**
   * Optional override for the context-menu action set. When omitted, the
   * component injects the full Epic E action list below. The Lit element
   * stays blank-slate; this Angular host owns the app-specific handling.
   */
  @Input() contextMenuItems?: ContextMenuItem[];

  /** Emitted when "About this EPR" is selected — host can show a popover. */
  @Output() about = new EventEmitter<string>();

  /** Emitted when "Steward this content" is selected (no backend call — REA lives in the zome). */
  @Output() steward = new EventEmitter<string>();

  /**
   * Emitted for governance selections (flag / challenge / open feedback).
   * Carries qahal's ContextMenuAction shape so a parent gateway can route it
   * — identical pattern to qahal's ContextMenuOnlyComponent (this component
   * never calls qahal services directly).
   */
  @Output() governance = new EventEmitter<ContextMenuAction>();

  private readonly eprResolver = inject(EprResolverService);
  private readonly router = inject(Router);
  private readonly eprNav = inject(EprNavService);
  private readonly acquisition = inject(AcquisitionService);
  private readonly elRef = inject<ElementRef<HTMLElement>>(ElementRef);

  /**
   * Rung-4 provide UI state. `activePeerPin` holds the epr id of the in-flight
   * pin (null = no progress bar). `peerPinStatus` holds the latest poll rollup
   * the <app-pin-progress> element binds. Both are host-owned — the stateless
   * element renders what it's fed and never fetches (blank-slate discipline).
   */
  protected readonly activePeerPin = signal<string | null>(null);
  protected readonly peerPinStatus = signal<PullStatusInfo | null>(null);

  /** The pull-status poll subscription. Disposed on caught-up, cancel, destroy. */
  private pullSub: Subscription | null = null;

  /**
   * The full Epic E action set the menu offers. Built-in conveniences
   * (open / about / copy) stay first so the de-@wip'd "right-click opens
   * the context menu" scenario (which asserts those three are present) keeps
   * passing as the list grows.
   */
  private get fullActionList(): ContextMenuItem[] {
    return (
      this.contextMenuItems ?? [
        { id: 'open', label: 'Open' },
        { id: 'about', label: 'About this EPR' },
        { id: 'copy', label: 'Copy EPR link' },
        // Rung 2 (spec §8): generic floor — routes identically to 'open' via
        // claims-aware nav today; the per-pillar enumerated sub-menu is Slice 3.
        { id: 'open-in', label: 'Open in app' },
        // Rung 3: acquisition / download for offline.
        { id: 'download', label: 'Download for offline' },
        { id: 'network', label: 'View network & resilience' },
        { id: 'relationships', label: 'See relationships' },
        { id: 'steward', label: 'Steward this content' },
        { id: 'flag', label: 'Flag' },
        { id: 'challenge', label: 'Challenge' },
        { id: 'feedback', label: 'Open feedback' },
      ]
    );
  }

  private readonly navigateListener = (e: Event): void => {
    const epr = (e as CustomEvent<{ epr: string }>).detail.epr;
    this.eprResolver.resolve(epr).subscribe(resolved => {
      this.navigateResolved(resolved);
    });
  };

  private readonly eprMenuSelectListener = (e: Event): void => {
    const { id, epr } = (e as CustomEvent<{ id: string; epr: string }>).detail;
    this.handleMenuSelect(id, epr);
  };

  ngOnInit(): void {
    const host = this.elRef.nativeElement;

    // Resolution is AMBIENT (I2): the shell root installs one EprResolutionProvider
    // via @lit/context, so <elohim-epr-link> resolves its head through the provider
    // with no per-element `.resolver` wiring here. Head-first previews and typed
    // degradation (forbidden / missing / error) come from that provider — this
    // wrapper only injects host-owned menu actions and translates events.
    const litEl = host.querySelector('elohim-epr-link') as ElohimEprLink;

    // Inject the full Epic E action set as a DOM property (a property, not an
    // attribute — must be set after the element upgrades).
    litEl.contextMenuItems = this.fullActionList;

    // Rung 4 (provide): pin-as-peer is gated to peer-capable nodes serving
    // commons-reach content. Resolve reach async, then re-set the property with
    // the action appended. Browser/non-commons leaves the base list untouched.
    void this.maybeOfferPinAsPeer(litEl);

    // Translate the Lit 'navigate' event into Angular Router navigation.
    host.addEventListener('navigate', this.navigateListener);
    // Handle context-menu selections the element re-emits.
    host.addEventListener('epr-menu-select', this.eprMenuSelectListener);
  }

  /**
   * Append the rung-4 "Pin as peer" action when (a) the node is peer-capable and
   * (b) the EPR resolves to commons reach. Only commons content is serveable to
   * arbitrary peers; private/dwelling reach has no provide path. Browser nodes
   * have no peer surface, so the action never appears there — the own-node
   * GET /api/v1/pins/{eprId}/pull route is unreachable in doorway mode anyway.
   */
  private async maybeOfferPinAsPeer(litEl: ElohimEprLink): Promise<void> {
    if (this.acquisition.capability() !== 'peer') return;
    const resolved = await firstValueFrom(this.eprResolver.resolve(this.epr));
    if (resolved?.content?.reach !== 'commons') return;
    litEl.contextMenuItems = [...this.fullActionList, { id: 'pin-as-peer', label: 'Pin as peer' }];
  }

  ngOnDestroy(): void {
    const host = this.elRef.nativeElement;
    host.removeEventListener('navigate', this.navigateListener);
    host.removeEventListener('epr-menu-select', this.eprMenuSelectListener);
    this.stopPolling();
  }

  /**
   * Rung-4 provide loop (host-owned lifecycle). POST the provide pin, then
   * subscribe the pull-status poll and feed each emission to the stateless
   * <app-pin-progress>. Peer-only — pinAsPeer() rejects on a browser node, but
   * the action isn't offered there, so this path is unreachable in doorway mode.
   */
  private startPeerPin(epr: string): void {
    this.activePeerPin.set(epr);
    this.peerPinStatus.set(null);
    // Attach .then/.catch synchronously (no await) so the handled-rejection job
    // is registered before any zone drain-end uncaught check — the native-await
    // phantom-unhandled caveat (MEMORY: zone.js native-await phantom uncaught).
    this.acquisition
      .pinAsPeer(epr)
      .then(() => this.subscribePull(epr))
      .catch(() => {
        console.warn('[EprLink] pin-as-peer failed for', epr);
        this.clearPeerPin();
      });
  }

  /** Subscribe the poll stream; stop polling once the backend asserts caught-up. */
  private subscribePull(epr: string): void {
    this.stopPolling();
    this.pullSub = this.acquisition.pullStatus$(epr).subscribe(status => {
      this.peerPinStatus.set(status);
      // null≠done: only a computable total + caughtUp:true ends the poll. The
      // element keeps showing the "✓ serving" badge; we just stop fetching.
      if (status.total !== null && status.caughtUp === true) {
        this.stopPolling();
      }
    });
  }

  /** Cancel: stop polling and clear the bar. No own-node un-pin route exists in
   *  the service yet, so cancel is a UI-local disposition (no DELETE). */
  protected cancelPeerPin(): void {
    this.clearPeerPin();
  }

  /** Retry: re-issue the provide pin and re-subscribe the poll for the same epr. */
  protected retryPeerPin(): void {
    const epr = this.activePeerPin();
    if (epr === null) return;
    this.startPeerPin(epr);
  }

  /** Tear down the poll subscription without clearing the rendered state. */
  private stopPolling(): void {
    this.pullSub?.unsubscribe();
    this.pullSub = null;
  }

  /** Stop polling AND remove the progress bar (cancel / pin failure). */
  private clearPeerPin(): void {
    this.stopPolling();
    this.activePeerPin.set(null);
    this.peerPinStatus.set(null);
  }

  /**
   * Owns all app-specific dispositions of a context-menu selection. The Lit
   * element decides nothing about routing/governance/stewardship — it only
   * surfaces the id (blank-slate discipline, app/elohim-elements/CLAUDE.md).
   */
  private handleMenuSelect(id: string, epr: string): void {
    switch (id) {
      case 'open':
      // Rung 2: claims-routed open-in (generic floor — routes via the same
      // claims-aware navigation as 'open'; per-pillar enumerated menu is Slice-3).
      // falls through
      case 'open-in':
        this.navigateToResource(epr);
        break;
      case 'about':
        this.about.emit(epr);
        break;
      case 'copy':
        // eslint-disable-next-line no-restricted-syntax -- SSR-safe: browser-only user-interaction handler (context-menu click), never invoked during SSR render
        void navigator.clipboard?.writeText(epr).catch(() => {
          // Clipboard write can reject (permissions / insecure context); no-op.
        });
        break;
      case 'download':
        // Rung 3: acquisition disposition — peer path pins to /api/v1/pins;
        // browser path warms the SW cache lane.
        void this.acquisition.download(epr).catch(() => {
          console.warn('[EprLink] download failed for', epr);
        });
        break;
      case 'pin-as-peer':
        // Rung 4 (provide): pin AND offer to serve to peers. Peer-only — the
        // service rejects on a browser node; the action isn't offered there.
        this.startPeerPin(epr);
        break;
      case 'network':
        // Resolve to the resource route, then land directly on the Network /
        // resilience tab via the route fragment (content-viewer honors it).
        this.eprResolver.resolve(epr).subscribe(resolved => {
          this.navigateResolved(resolved, { fragment: 'network' });
        });
        break;
      case 'relationships':
        // Relationships render in the content tab of the resource view.
        this.navigateToResource(epr);
        break;
      case 'steward':
        // No pledge-by-EPR service exists — REA event creation belongs in the
        // zome/doorway, never the client. Emit upward; default to the cluster.
        this.steward.emit(epr);
        void this.router.navigate(['/shefa/cluster']);
        break;
      case 'flag':
      case 'challenge':
      case 'feedback':
        this.governance.emit({
          entityType: 'epr',
          entityId: epr,
          action: id === 'feedback' ? 'open-feedback' : id,
        });
        break;
    }
  }

  /** Route in-bundle when claimed; cross-bundle full-load via the universal href.
   *  NOTE: navExtras (e.g. fragment) apply only to in-bundle navigation — a
   *  cross-bundle full-load drops them (Slice 3 fragment-preserving redirect). */
  private navigateResolved(resolved: ResolvedContent | null, extras?: NavigationExtras): void {
    if (resolved?.route) {
      void (extras
        ? this.router.navigate(resolved.route, extras)
        : this.router.navigate(resolved.route));
    } else if (resolved) {
      this.eprNav.navigate(resolved.href);
    }
  }

  private navigateToResource(epr: string): void {
    this.eprResolver.resolve(epr).subscribe(resolved => {
      this.navigateResolved(resolved);
    });
  }
}
