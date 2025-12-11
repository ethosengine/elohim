import { Component, OnInit, ElementRef, Renderer2, ViewEncapsulation } from '@angular/core';

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

  constructor(private readonly el: ElementRef, private readonly renderer: Renderer2) {}

  ngOnInit() {
    this.setupScrollIndicator();
    this.setupHeroTitleInteraction();
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

  private setupScrollIndicator() {
    setTimeout(() => {
      const scrollIndicator = this.el.nativeElement.querySelector('.scroll-indicator');
      if (scrollIndicator) {
        this.renderer.listen(scrollIndicator, 'click', () => {
          window.scrollTo({
            top: window.innerHeight,
            behavior: 'smooth'
          });
        });
      }
    }, 0);
  }

  private setupHeroTitleInteraction() {
    setTimeout(() => {
      const heroTitle = this.el.nativeElement.querySelector('.hero h1');
      if (heroTitle) {
        this.renderer.setStyle(heroTitle, 'cursor', 'pointer');
        this.renderer.listen(heroTitle, 'click', () => {
          this.renderer.setStyle(heroTitle, 'animation', 'none');
          setTimeout(() => {
            this.renderer.setStyle(heroTitle, 'animation', 'float 6s ease-in-out infinite');
          }, 10);
        });
      }
    }, 0);
  }
}
