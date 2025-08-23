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
import { DebugBarComponent } from './components/debug-bar/debug-bar.component';
import { ConfigService } from './services/config.service';
import { AnalyticsService } from './services/analytics.service';

@Component({
  selector: 'app-root',
  imports: [
    DebugBarComponent,
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
  private rafId?: number;
  private isScrolling = false;

  constructor(
    private readonly el: ElementRef, 
    private readonly renderer: Renderer2, 
    private readonly configService: ConfigService,
    private readonly analyticsService: AnalyticsService
  ) {}

  ngOnInit() {
    this.configService.getConfig().subscribe(() => {
      this.setupParallaxScrolling();
      this.setupIntersectionObserver();
      this.setupScrollIndicator();
      this.setupHeroTitleInteraction();
    });
  }

  ngOnDestroy() {
    if (this.scrollListener) {
      window.removeEventListener('scroll', this.scrollListener);
    }
    if (this.intersectionObserver) {
      this.intersectionObserver.disconnect();
    }
    if (this.rafId) {
      cancelAnimationFrame(this.rafId);
    }
  }

  private setupParallaxScrolling() {
    this.scrollListener = () => {
      if (!this.isScrolling) {
        this.isScrolling = true;
        this.rafId = requestAnimationFrame(() => {
          this.updateParallaxElements();
          this.isScrolling = false;
        });
      }
    };

    window.addEventListener('scroll', this.scrollListener, { passive: true });
  }

  private updateParallaxElements() {
    const scrolled = window.pageYOffset;
    
    // Update CSS custom property for parallax background
    this.renderer.setStyle(
      this.el.nativeElement.querySelector('.parallax-bg'), 
      '--scroll-y', 
      `${scrolled}px`
    );
    
    const parallaxLayers = this.el.nativeElement.querySelectorAll('.parallax-layer');
    parallaxLayers.forEach((layer: HTMLElement) => {
      const speed = parseFloat(layer.dataset['speed'] ?? '0.5');
      const yPos = -(scrolled * speed);
      this.renderer.setStyle(layer, 'transform', `translate3d(0, ${yPos}px, 0)`);
    });

    // Move orbs based on scroll
    const orbs = this.el.nativeElement.querySelectorAll('.orb');
    orbs.forEach((orb: HTMLElement, index: number) => {
      const speed = 0.1 * (index + 1);
      this.renderer.setStyle(orb, 'transform', `translate3d(0, ${scrolled * speed}px, 0)`);
    });
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
          
          // Add visible class to all cards in card grid (CSS handles staggering)
          const cardGrid = entry.target.querySelector('.card-grid');
          if (cardGrid) {
            const cards = cardGrid.querySelectorAll('.card');
            cards.forEach((card: Element) => {
              this.renderer.addClass(card, 'visible');
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
