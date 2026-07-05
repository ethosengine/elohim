import { ElementRef, Injectable, Renderer2, RendererFactory2, inject } from '@angular/core';

// @coverage: 100.0% (2026-02-24)

/**
 * Service for common DOM interaction patterns
 */
@Injectable({
  providedIn: 'root',
})
export class DomInteractionService {
  private readonly renderer: Renderer2;

  private readonly rendererFactory = inject(RendererFactory2);

  constructor() {
    this.renderer = this.rendererFactory.createRenderer(null, null);
  }

  /**
   * Setup scroll indicator that scrolls to one viewport height
   * @param elementRef - Element reference containing the scroll indicator
   */
  setupScrollIndicator(elementRef: ElementRef<HTMLElement>): void {
    setTimeout(() => {
      const scrollIndicator = elementRef.nativeElement.querySelector('.scroll-indicator');
      if (scrollIndicator) {
        this.renderer.listen(scrollIndicator, 'click', () => {
          // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside a renderer.listen('click', ...) callback body, only invoked on a genuine click event — no DOM event dispatch occurs during SSR, so this never executes server-side
          window.scrollTo({
            // eslint-disable-next-line no-restricted-syntax -- SSR-safe: same click-callback body as above, never invoked during SSR
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
  setupHeroTitleAnimation(elementRef: ElementRef<HTMLElement>): void {
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
