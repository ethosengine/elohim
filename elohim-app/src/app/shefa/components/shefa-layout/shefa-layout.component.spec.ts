/**
 * Shefa Layout Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideRouter } from '@angular/router';

import { ShefaLayoutComponent } from './shefa-layout.component';

describe('ShefaLayoutComponent', () => {
  let component: ShefaLayoutComponent;
  let fixture: ComponentFixture<ShefaLayoutComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ShefaLayoutComponent],
      providers: [provideHttpClient(), provideHttpClientTesting(), provideRouter([])],
    }).compileComponents();

    fixture = TestBed.createComponent(ShefaLayoutComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('tab navigation', () => {
    it('should render a nav element with aria-label', () => {
      const nav = fixture.nativeElement.querySelector('nav.shefa-tab-bar');
      expect(nav).toBeTruthy();
      expect(nav.getAttribute('aria-label')).toBe('Shefa sections');
    });

    it('should render 3 tabs with correct labels', () => {
      const tabs: HTMLAnchorElement[] = fixture.nativeElement.querySelectorAll('.shefa-tab');
      expect(tabs.length).toBe(3);
      expect(tabs[0].textContent?.trim()).toBe('Overview');
      expect(tabs[1].textContent?.trim()).toBe('Dashboard');
      expect(tabs[2].textContent?.trim()).toBe('Devices');
    });

    it('should have correct routerLink values', () => {
      const tabs: HTMLAnchorElement[] = fixture.nativeElement.querySelectorAll('.shefa-tab');
      expect(tabs[0].getAttribute('href')).toBe('/shefa');
      expect(tabs[1].getAttribute('href')).toBe('/shefa/dashboard');
      expect(tabs[2].getAttribute('href')).toBe('/shefa/devices');
    });
  });
});
