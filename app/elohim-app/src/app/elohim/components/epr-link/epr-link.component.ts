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
} from '@angular/core';
import { Router } from '@angular/router';

import { firstValueFrom } from 'rxjs';

import 'elohim-core/register';

import { type ContextMenuAction } from '@app/qahal';

import { EprResolverService } from '../../services/epr-resolver.service';
import { EprNavService } from '../../services/epr-nav.service';

import type { ContextMenuItem, ElohimEprLink, EprLinkDisplay } from 'elohim-core';

export type { EprLinkDisplay } from 'elohim-core';

@Component({
  selector: 'app-epr-link',
  standalone: true,
  imports: [],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  template: `
    <elohim-epr-link [attr.epr]="epr" [attr.display]="display"></elohim-epr-link>
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
  private readonly elRef = inject<ElementRef<HTMLElement>>(ElementRef);

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
      if (resolved?.route) {
        void this.router.navigate(resolved.route);
      } else if (resolved) {
        this.eprNav.navigate(resolved.href);
      }
    });
  };

  private readonly eprMenuSelectListener = (e: Event): void => {
    const { id, epr } = (e as CustomEvent<{ id: string; epr: string }>).detail;
    this.handleMenuSelect(id, epr);
  };

  ngOnInit(): void {
    const host = this.elRef.nativeElement;

    // Wire the Lit element's resolver property (not an attribute — must be set
    // as a DOM property after the element upgrades).
    const litEl = host.querySelector('elohim-epr-link') as ElohimEprLink;
    litEl.resolver = async (eprRef: string) => {
      const resolved = await firstValueFrom(this.eprResolver.resolve(eprRef));
      if (!resolved) return null;
      return {
        title: resolved.content.title,
        description: resolved.content.description || undefined,
        reach: resolved.content.reach,
      };
    };

    // Inject the full Epic E action set as a DOM property (same as resolver —
    // a property, not an attribute).
    litEl.contextMenuItems = this.fullActionList;

    // Translate the Lit 'navigate' event into Angular Router navigation.
    host.addEventListener('navigate', this.navigateListener);
    // Handle context-menu selections the element re-emits.
    host.addEventListener('epr-menu-select', this.eprMenuSelectListener);
  }

  ngOnDestroy(): void {
    const host = this.elRef.nativeElement;
    host.removeEventListener('navigate', this.navigateListener);
    host.removeEventListener('epr-menu-select', this.eprMenuSelectListener);
  }

  /**
   * Owns all app-specific dispositions of a context-menu selection. The Lit
   * element decides nothing about routing/governance/stewardship — it only
   * surfaces the id (blank-slate discipline, app/elohim-elements/CLAUDE.md).
   */
  private handleMenuSelect(id: string, epr: string): void {
    switch (id) {
      case 'open':
        this.navigateToResource(epr);
        break;
      case 'about':
        this.about.emit(epr);
        break;
      case 'copy':
        void navigator.clipboard?.writeText(epr).catch(() => {
          // Clipboard write can reject (permissions / insecure context); no-op.
        });
        break;
      case 'network':
        // Resolve to the resource route, then land directly on the Network /
        // resilience tab via the route fragment (content-viewer honors it).
        this.eprResolver.resolve(epr).subscribe(resolved => {
          if (resolved?.route) {
            void this.router.navigate(resolved.route, { fragment: 'network' });
          } else if (resolved) {
            this.eprNav.navigate(resolved.href);
          }
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

  private navigateToResource(epr: string): void {
    this.eprResolver.resolve(epr).subscribe(resolved => {
      if (resolved?.route) {
        void this.router.navigate(resolved.route);
      } else if (resolved) {
        this.eprNav.navigate(resolved.href);
      }
    });
  }
}
