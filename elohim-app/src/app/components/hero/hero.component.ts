import { Component, OnInit, ElementRef, Renderer2 } from '@angular/core';

@Component({
  selector: 'app-hero',
  imports: [],
  templateUrl: './hero.component.html',
  styleUrl: './hero.component.css'
})
export class HeroComponent implements OnInit {

  constructor(private readonly el: ElementRef, private readonly renderer: Renderer2) {}

  ngOnInit() {
    this.setupScrollIndicator();
    this.setupHeroTitleInteraction();
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
