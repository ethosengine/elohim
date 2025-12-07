import { TestBed } from '@angular/core/testing';
import { ElementRef, RendererFactory2, Renderer2 } from '@angular/core';
import { DomInteractionService } from './dom-interaction.service';

describe('DomInteractionService', () => {
  let service: DomInteractionService;
  let mockRenderer: jasmine.SpyObj<Renderer2>;
  let mockRendererFactory: jasmine.SpyObj<RendererFactory2>;

  beforeEach(() => {
    mockRenderer = jasmine.createSpyObj('Renderer2', ['listen', 'setStyle']);
    mockRendererFactory = jasmine.createSpyObj('RendererFactory2', ['createRenderer']);
    mockRendererFactory.createRenderer.and.returnValue(mockRenderer);

    TestBed.configureTestingModule({
      providers: [
        DomInteractionService,
        { provide: RendererFactory2, useValue: mockRendererFactory }
      ]
    });

    service = TestBed.inject(DomInteractionService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('setupScrollIndicator', () => {
    it('should setup scroll indicator click listener', (done) => {
      const mockElement = document.createElement('div');
      const scrollIndicator = document.createElement('div');
      scrollIndicator.className = 'scroll-indicator';
      mockElement.appendChild(scrollIndicator);

      const elementRef = new ElementRef(mockElement);
      const scrollSpy = spyOn(window, 'scrollTo');
      let clickHandler: (event: any) => void;

      mockRenderer.listen.and.callFake((element: any, event: string, handler: (event: any) => void) => {
        if (event === 'click') {
          clickHandler = handler;
        }
        return () => {};
      });

      service.setupScrollIndicator(elementRef);

      setTimeout(() => {
        expect(mockRenderer.listen).toHaveBeenCalledWith(scrollIndicator, 'click', jasmine.any(Function));

        // Trigger the click handler
        if (clickHandler!) {
          clickHandler({});
        }

        expect(scrollSpy).toHaveBeenCalled();
        const callArgs = scrollSpy.calls.mostRecent().args[0] as ScrollToOptions;
        expect(callArgs.top).toBe(window.innerHeight);
        expect(callArgs.behavior).toBe('smooth');

        done();
      }, 10);
    });

    it('should handle missing scroll indicator gracefully', (done) => {
      const mockElement = document.createElement('div');
      const elementRef = new ElementRef(mockElement);

      service.setupScrollIndicator(elementRef);

      setTimeout(() => {
        expect(mockRenderer.listen).not.toHaveBeenCalled();
        done();
      }, 10);
    });
  });

  describe('setupHeroTitleAnimation', () => {
    it('should setup hero title click animation', (done) => {
      const mockElement = document.createElement('div');
      const heroSection = document.createElement('div');
      heroSection.className = 'hero';
      const heroTitle = document.createElement('h1');
      heroSection.appendChild(heroTitle);
      mockElement.appendChild(heroSection);

      const elementRef = new ElementRef(mockElement);
      let clickHandler: (event: any) => void;

      mockRenderer.listen.and.callFake((element: any, event: string, handler: (event: any) => void) => {
        if (event === 'click') {
          clickHandler = handler;
        }
        return () => {};
      });

      service.setupHeroTitleAnimation(elementRef);

      setTimeout(() => {
        expect(mockRenderer.setStyle).toHaveBeenCalledWith(heroTitle, 'cursor', 'pointer');
        expect(mockRenderer.listen).toHaveBeenCalledWith(heroTitle, 'click', jasmine.any(Function));

        // Reset the spy to check calls within click handler
        mockRenderer.setStyle.calls.reset();

        // Trigger the click handler
        if (clickHandler!) {
          clickHandler({});
        }

        expect(mockRenderer.setStyle).toHaveBeenCalledWith(heroTitle, 'animation', 'none');

        // Wait for the nested setTimeout
        setTimeout(() => {
          expect(mockRenderer.setStyle).toHaveBeenCalledWith(
            heroTitle,
            'animation',
            'float 6s ease-in-out infinite'
          );
          done();
        }, 20);
      }, 10);
    });

    it('should handle missing hero title gracefully', (done) => {
      const mockElement = document.createElement('div');
      const elementRef = new ElementRef(mockElement);

      service.setupHeroTitleAnimation(elementRef);

      setTimeout(() => {
        expect(mockRenderer.setStyle).not.toHaveBeenCalled();
        expect(mockRenderer.listen).not.toHaveBeenCalled();
        done();
      }, 10);
    });

    it('should handle hero section without h1', (done) => {
      const mockElement = document.createElement('div');
      const heroSection = document.createElement('div');
      heroSection.className = 'hero';
      mockElement.appendChild(heroSection);

      const elementRef = new ElementRef(mockElement);

      service.setupHeroTitleAnimation(elementRef);

      setTimeout(() => {
        expect(mockRenderer.setStyle).not.toHaveBeenCalled();
        expect(mockRenderer.listen).not.toHaveBeenCalled();
        done();
      }, 10);
    });
  });
});
