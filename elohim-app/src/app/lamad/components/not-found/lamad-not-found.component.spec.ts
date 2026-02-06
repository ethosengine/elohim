import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, provideRouter } from '@angular/router';
import { LamadNotFoundComponent } from './lamad-not-found.component';
import { SeoService } from '../../../services/seo.service';

describe('LamadNotFoundComponent', () => {
  let component: LamadNotFoundComponent;
  let fixture: ComponentFixture<LamadNotFoundComponent>;
  let seoServiceSpy: jasmine.SpyObj<SeoService>;
  let router: Router;

  beforeEach(async () => {
    seoServiceSpy = jasmine.createSpyObj('SeoService', ['updateSeo']);

    await TestBed.configureTestingModule({
      imports: [LamadNotFoundComponent],
      providers: [provideRouter([]), { provide: SeoService, useValue: seoServiceSpy }],
    }).compileComponents();

    router = TestBed.inject(Router);
    spyOn(router, 'navigate');
    // Mock the url property
    Object.defineProperty(router, 'url', { value: '/lamad/some/invalid/path', writable: true });

    fixture = TestBed.createComponent(LamadNotFoundComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should set attempted URL on init', () => {
    fixture.detectChanges();
    expect(component.attemptedUrl).toBe('/lamad/some/invalid/path');
  });

  it('should update SEO with noIndex on init', () => {
    fixture.detectChanges();

    expect(seoServiceSpy.updateSeo).toHaveBeenCalledWith(
      jasmine.objectContaining({
        title: 'Content Not Found - Lamad',
        noIndex: true,
      })
    );
  });

  it('should detect path resource type', () => {
    Object.defineProperty(router, 'url', { value: '/lamad/path/invalid-id', writable: true });
    fixture.detectChanges();

    expect(component.resourceType).toBe('path');
  });

  it('should detect resource type for /resource/ URLs', () => {
    Object.defineProperty(router, 'url', { value: '/lamad/resource/invalid-id', writable: true });
    fixture.detectChanges();

    expect(component.resourceType).toBe('resource');
  });

  it('should get appropriate message for path type', () => {
    component.resourceType = 'path';
    expect(component.getMessage()).toContain('learning path');
  });

  it('should get appropriate message for resource type', () => {
    component.resourceType = 'resource';
    expect(component.getMessage()).toContain('content resource');
  });

  it('should get default message for unknown type', () => {
    component.resourceType = 'unknown';
    expect(component.getMessage()).toContain('curriculum');
  });

  it('should navigate to lamad home', () => {
    component.goToLamadHome();
    expect(router.navigate).toHaveBeenCalledWith(['/lamad']);
  });

  it('should navigate to search', () => {
    component.goToSearch();
    expect(router.navigate).toHaveBeenCalledWith(['/lamad/search']);
  });

  it('should navigate to explore', () => {
    component.goToExplore();
    expect(router.navigate).toHaveBeenCalledWith(['/lamad/explore']);
  });

  it('should go back', () => {
    spyOn(window.history, 'back');
    component.goBack();
    expect(window.history.back).toHaveBeenCalled();
  });
});
