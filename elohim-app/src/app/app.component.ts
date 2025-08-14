import { Component, OnInit, OnDestroy, ElementRef, Renderer2 } from '@angular/core';
import { HeroComponent } from './components/hero/hero.component';
import { CrisisComponent } from './components/crisis/crisis.component';
import { VisionComponent } from './components/vision/vision.component';
import { ElohimHostComponent } from './components/elohim-host/elohim-host.component';
import { DesignPrinciplesComponent } from './components/design-principles/design-principles.component';
import { LearningSuccessComponent } from './components/learning-success/learning-success.component';
import { PathForwardComponent } from './components/path-forward/path-forward.component';
import { CallToActionComponent } from './components/call-to-action/call-to-action.component';
import { FooterComponent } from './components/footer/footer.component';

@Component({
  selector: 'app-root',
  imports: [
    HeroComponent,
    CrisisComponent,
    VisionComponent,
    ElohimHostComponent,
    DesignPrinciplesComponent,
    LearningSuccessComponent,
    PathForwardComponent,
    CallToActionComponent,
    FooterComponent
  ],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit, OnDestroy {
  title = 'elohim-app';
  private scrollListener?: () => void;
  private intersectionObserver?: IntersectionObserver;

  constructor(private el: ElementRef, private renderer: Renderer2) {}

  ngOnInit() {
    this.setupParallaxScrolling();
    this.setupIntersectionObserver();
    this.setupScrollIndicator();
    this.setupHeroTitleInteraction();
  }

  ngOnDestroy() {
    if (this.scrollListener) {
      window.removeEventListener('scroll', this.scrollListener);
    }
    if (this.intersectionObserver) {
      this.intersectionObserver.disconnect();
    }
  }

  private setupParallaxScrolling() {
    this.scrollListener = () => {
      const scrolled = window.pageYOffset;
      const parallaxLayers = this.el.nativeElement.querySelectorAll('.parallax-layer');
      
      parallaxLayers.forEach((layer: HTMLElement) => {
        const speed = parseFloat(layer.dataset['speed'] || '0.5');
        const yPos = -(scrolled * speed);
        this.renderer.setStyle(layer, 'transform', `translateY(${yPos}px)`);
      });

      // Move orbs based on scroll
      const orbs = this.el.nativeElement.querySelectorAll('.orb');
      orbs.forEach((orb: HTMLElement, index: number) => {
        const speed = 0.1 * (index + 1);
        this.renderer.setStyle(orb, 'transform', `translateY(${scrolled * speed}px)`);
      });
    };

    window.addEventListener('scroll', this.scrollListener);
  }

  private setupIntersectionObserver() {
    const observerOptions = {
      threshold: 0.1,
      rootMargin: '0px 0px -50px 0px'
    };

    this.intersectionObserver = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          this.renderer.addClass(entry.target, 'visible');
          
          // Stagger card animations
          const cardGrid = entry.target.querySelector('.card-grid');
          if (cardGrid) {
            const cards = cardGrid.querySelectorAll('.card');
            cards.forEach((card: Element, index: number) => {
              setTimeout(() => {
                this.renderer.addClass(card, 'visible');
              }, index * 100);
            });
          }
        }
      });
    }, observerOptions);

    // Observe all sections and cards after view init
    setTimeout(() => {
      const sections = this.el.nativeElement.querySelectorAll('.section-content');
      sections.forEach((section: HTMLElement) => {
        this.intersectionObserver?.observe(section);
      });
    }, 0);
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
