import { Component, OnInit, ElementRef, ViewEncapsulation } from '@angular/core';
import { DomInteractionService } from '../../services/dom-interaction.service';

@Component({
  selector: 'app-hero',
  imports: [],
  templateUrl: './hero.component.html',
  styleUrl: './hero.component.css',
  encapsulation: ViewEncapsulation.None
})
export class HeroComponent implements OnInit {
  isVideoVisible: boolean = false;
  isDeepDiveVisible: boolean = false;

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
