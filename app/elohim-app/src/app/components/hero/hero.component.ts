import {
  Component,
  OnInit,
  ElementRef,
  ViewEncapsulation,
} from '@angular/core';

// @coverage: 100.0% (2026-02-24)

import { eprToUniversalHref } from '@elohim/service';

import { DomInteractionService } from '../../services/dom-interaction.service';

@Component({
  selector: 'app-hero',
  imports: [],
  templateUrl: './hero.component.html',
  styleUrl: './hero.component.css',
  encapsulation: ViewEncapsulation.None,
})
export class HeroComponent implements OnInit {
  isVideoVisible = false;
  isDeepDiveVisible = false;

  /** Minted universal EPR address for the flagship protocol content (→ /epr/elohim-protocol). */
  readonly protocolHref = eprToUniversalHref({ id: 'elohim-protocol', tier: 'head' });

  constructor(
    private readonly el: ElementRef,
    private readonly domInteractionService: DomInteractionService
  ) {}

  ngOnInit() {
    this.domInteractionService.setupScrollIndicator(this.el);
    this.domInteractionService.setupHeroTitleAnimation(this.el);
  }

  toggleVideo() {
    this.isVideoVisible = true;
    this.isDeepDiveVisible = false;
  }

  toggleDeepDive() {
    this.isDeepDiveVisible = true;
    this.isVideoVisible = false;
  }

  hideVideo() {
    this.isVideoVisible = false;
    this.isDeepDiveVisible = false;
  }
}
