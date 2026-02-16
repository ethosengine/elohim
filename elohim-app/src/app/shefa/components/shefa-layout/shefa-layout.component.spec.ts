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

  describe('sidebar layout', () => {
    it('should render the workspace container', () => {
      const workspace = fixture.nativeElement.querySelector('.shefa-workspace');
      expect(workspace).toBeTruthy();
    });

    it('should render the sidenav component', () => {
      const sidenav = fixture.nativeElement.querySelector('app-shefa-sidenav');
      expect(sidenav).toBeTruthy();
    });

    it('should render the main content area with router-outlet', () => {
      const main = fixture.nativeElement.querySelector('main.shefa-content');
      expect(main).toBeTruthy();
      const outlet = main.querySelector('router-outlet');
      expect(outlet).toBeTruthy();
    });

    it('should render a content scroll container', () => {
      const scroll = fixture.nativeElement.querySelector('.content-scroll-container');
      expect(scroll).toBeTruthy();
    });

    it('should render sidebar toggle button', () => {
      const btn = fixture.nativeElement.querySelector('.sidebar-toggle');
      expect(btn).toBeTruthy();
      expect(btn.getAttribute('aria-label')).toBe('Toggle navigation menu');
    });

    it('should render backdrop always in DOM', () => {
      const backdrop = fixture.nativeElement.querySelector('.sidebar-backdrop');
      expect(backdrop).toBeTruthy();
    });

    it('should render expand button always in DOM', () => {
      const expandBtn = fixture.nativeElement.querySelector('.sidebar-expand-btn');
      expect(expandBtn).toBeTruthy();
      expect(expandBtn.getAttribute('aria-label')).toBe('Expand sidebar');
    });
  });

  describe('sidebar state', () => {
    it('should start with sidebar open', () => {
      expect(component.sidebarOpen()).toBe(true);
    });

    it('should not have sidebar-collapsed class when sidebar is open', () => {
      const workspace = fixture.nativeElement.querySelector('.shefa-workspace');
      expect(workspace.classList.contains('sidebar-collapsed')).toBe(false);
    });

    it('should add sidebar-collapsed class when sidebar is closed', () => {
      component.sidebarOpen.set(false);
      fixture.detectChanges();

      const workspace = fixture.nativeElement.querySelector('.shefa-workspace');
      expect(workspace.classList.contains('sidebar-collapsed')).toBe(true);
    });

    it('should toggle sidebarOpen on toggleSidebar()', () => {
      expect(component.sidebarOpen()).toBe(true);

      component.toggleSidebar();
      expect(component.sidebarOpen()).toBe(false);

      component.toggleSidebar();
      expect(component.sidebarOpen()).toBe(true);
    });

    it('should toggle sidebar when toggle button is clicked', () => {
      const btn: HTMLButtonElement = fixture.nativeElement.querySelector('.sidebar-toggle');
      btn.click();
      expect(component.sidebarOpen()).toBe(false);

      btn.click();
      expect(component.sidebarOpen()).toBe(true);
    });

    it('should toggle sidebar when backdrop is clicked', () => {
      const backdrop: HTMLElement = fixture.nativeElement.querySelector('.sidebar-backdrop');
      backdrop.click();
      expect(component.sidebarOpen()).toBe(false);
    });

    it('should toggle sidebar when expand button is clicked', () => {
      component.sidebarOpen.set(false);
      fixture.detectChanges();

      const expandBtn: HTMLButtonElement =
        fixture.nativeElement.querySelector('.sidebar-expand-btn');
      expandBtn.click();
      expect(component.sidebarOpen()).toBe(true);
    });
  });
});
