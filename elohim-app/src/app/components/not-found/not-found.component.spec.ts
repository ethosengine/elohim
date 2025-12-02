import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';
import { NotFoundComponent } from './not-found.component';
import { SeoService } from '../../services/seo.service';

describe('NotFoundComponent', () => {
  let component: NotFoundComponent;
  let fixture: ComponentFixture<NotFoundComponent>;
  let seoServiceSpy: jasmine.SpyObj<SeoService>;
  let router: Router;

  beforeEach(async () => {
    seoServiceSpy = jasmine.createSpyObj('SeoService', ['updateSeo']);

    await TestBed.configureTestingModule({
      imports: [NotFoundComponent],
      providers: [
        provideRouter([]),
        { provide: SeoService, useValue: seoServiceSpy }
      ]
    }).compileComponents();

    router = TestBed.inject(Router);
    spyOn(router, 'navigate');
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

    expect(seoServiceSpy.updateSeo).toHaveBeenCalledWith(jasmine.objectContaining({
      title: 'Page Not Found',
      noIndex: true
    }));
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
    spyOn(window.history, 'back');
    component.goBack();
    expect(window.history.back).toHaveBeenCalled();
  });
});
