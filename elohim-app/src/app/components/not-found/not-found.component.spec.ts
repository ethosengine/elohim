import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';

import { SeoService } from '../../services/seo.service';

import { NotFoundComponent } from './not-found.component';
import { vi, Mock } from 'vitest';

describe('NotFoundComponent', () => {
  let component: NotFoundComponent;
  let fixture: ComponentFixture<NotFoundComponent>;
  let seoServiceSpy: any;
  let router: Router;

  beforeEach(async () => {
    seoServiceSpy = {
      updateSeo: vi.fn(),
    };

    await TestBed.configureTestingModule({
      imports: [NotFoundComponent],
      providers: [provideRouter([]), { provide: SeoService, useValue: seoServiceSpy }],
    }).compileComponents();

    router = TestBed.inject(Router);
    vi.spyOn(router, 'navigate');
    // Mock the url property
    Object.defineProperty(router, 'url', { value: '/some/invalid/path', writable: true });

    fixture = TestBed.createComponent(NotFoundComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should set attempted URL on init', () => {
    fixture.detectChanges();
    expect(component.attemptedUrl).toBe('/some/invalid/path');
  });

  it('should update SEO with noIndex on init', () => {
    fixture.detectChanges();

    expect(seoServiceSpy.updateSeo).toHaveBeenCalledWith(
      expect.objectContaining({
        title: 'Page Not Found',
        noIndex: true,
      })
    );
  });

  it('should navigate to home', () => {
    component.goHome();
    expect(router.navigate).toHaveBeenCalledWith(['/']);
  });

  it('should navigate to lamad', () => {
    component.goToLamad();
    expect(router.navigate).toHaveBeenCalledWith(['/lamad']);
  });

  it('should go back', () => {
    vi.spyOn(window.history, 'back');
    component.goBack();
    expect(window.history.back).toHaveBeenCalled();
  });
});
