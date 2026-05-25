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
  Input,
  OnDestroy,
  OnInit,
  inject,
} from '@angular/core';
import { Router } from '@angular/router';

import { firstValueFrom } from 'rxjs';

import 'elohim-core/register';

import { EprResolverService } from '../../services/epr-resolver.service';

import type { ElohimEprLink, EprLinkDisplay } from 'elohim-core';

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

  private readonly eprResolver = inject(EprResolverService);
  private readonly router = inject(Router);
  private readonly elRef = inject<ElementRef<HTMLElement>>(ElementRef);

  private readonly navigateListener = (e: Event): void => {
    const epr = (e as CustomEvent<{ epr: string }>).detail.epr;
    this.eprResolver.resolve(epr).subscribe(resolved => {
      if (resolved?.route) {
        void this.router.navigate(resolved.route);
      }
    });
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

    // Translate the Lit 'navigate' event into Angular Router navigation.
    host.addEventListener('navigate', this.navigateListener);
  }

  ngOnDestroy(): void {
    this.elRef.nativeElement.removeEventListener('navigate', this.navigateListener);
  }
}
