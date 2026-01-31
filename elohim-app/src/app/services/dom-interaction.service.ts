import { Injectable, ElementRef, Renderer2, RendererFactory2 } from '@angular/core';

// @coverage: 100.0% (2026-02-04)

/**
 * Service for common DOM interaction patterns
 */
@Injectable({
  providedIn: 'root',
})
export class DomInteractionService {
  private readonly renderer: Renderer2;

  constructor(rendererFactory: RendererFactory2) {
    this.renderer = rendererFactory.createRenderer(null, null);
  }

  /**
   * Setup scroll indicator that scrolls to one viewport height
   * @param elementRef - Element reference containing the scroll indicator
   */
  setupScrollIndicator(elementRef: ElementRef): void {
    setTimeout(() => {
      const scrollIndicator = elementRef.nativeElement.querySelector('.scroll-indicator');
      if (scrollIndicator) {
        this.renderer.listen(scrollIndicator, 'click', () => {
          window.scrollTo({
            top: window.innerHeight,
            behavior: 'smooth',
          });
        });
      }
    }, 0);
  }

  /**
   * Setup hero title animation interaction
   * @param elementRef - Element reference containing the hero title
   */
  setupHeroTitleAnimation(elementRef: ElementRef): void {
    setTimeout(() => {
      const heroTitle = elementRef.nativeElement.querySelector('.hero h1');
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
