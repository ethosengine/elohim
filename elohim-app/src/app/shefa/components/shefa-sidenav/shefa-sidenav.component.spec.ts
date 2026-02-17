/**
 * Shefa Sidenav Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { ShefaSidenavComponent } from './shefa-sidenav.component';

describe('ShefaSidenavComponent', () => {
  let component: ShefaSidenavComponent;
  let fixture: ComponentFixture<ShefaSidenavComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ShefaSidenavComponent],
      providers: [provideRouter([])],
    }).compileComponents();

    fixture = TestBed.createComponent(ShefaSidenavComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('sidebar header', () => {
    it('should render sidebar context label', () => {
      const context = fixture.nativeElement.querySelector('.sidebar-context');
      expect(context).toBeTruthy();
      expect(context.textContent.trim()).toBe('Shefa');
    });

    it('should render sidebar title', () => {
      const title = fixture.nativeElement.querySelector('.sidebar-title');
      expect(title).toBeTruthy();
      expect(title.textContent.trim()).toBe('Economy');
    });

    it('should render header with border-bottom separator', () => {
      const header = fixture.nativeElement.querySelector('.sidebar-header');
      expect(header).toBeTruthy();
    });
  });

  describe('navigation groups', () => {
    it('should render 4 nav groups', () => {
      const groups = fixture.nativeElement.querySelectorAll('.nav-group');
      expect(groups.length).toBe(4);
    });

    it('should render group titles', () => {
      const titles: HTMLElement[] = fixture.nativeElement.querySelectorAll('.nav-group-title');
      const titleTexts = Array.from(titles).map(t => t.textContent?.trim());
      expect(titleTexts).toEqual(['Primary', 'Resources', 'Community', 'Management']);
    });

    it('should render 13 nav items total', () => {
      const items = fixture.nativeElement.querySelectorAll('.nav-item');
      expect(items.length).toBe(13);
    });
  });

  describe('nav items', () => {
    it('should render Material icons for each nav item', () => {
      const icons = fixture.nativeElement.querySelectorAll('.nav-icon');
      expect(icons.length).toBe(13);
      expect(icons[0].textContent?.trim()).toBe('home');
    });

    it('should render labels for each nav item', () => {
      const labels: HTMLElement[] = fixture.nativeElement.querySelectorAll('.nav-label');
      expect(labels[0].textContent?.trim()).toBe('Overview');
      expect(labels[3].textContent?.trim()).toBe('Devices');
    });

    it('should have correct routerLink on Overview', () => {
      const items: HTMLAnchorElement[] = fixture.nativeElement.querySelectorAll('.nav-item');
      expect(items[0].getAttribute('href')).toBe('/shefa');
    });

    it('should have correct routerLink on Accounts', () => {
      const items: HTMLAnchorElement[] = fixture.nativeElement.querySelectorAll('.nav-item');
      expect(items[1].getAttribute('href')).toBe('/shefa/accounts');
    });

    it('should have correct routerLink on Dashboard', () => {
      const items: HTMLAnchorElement[] = fixture.nativeElement.querySelectorAll('.nav-item');
      // Dashboard is the 11th item (index 10) in Management group
      expect(items[10].getAttribute('href')).toBe('/shefa/dashboard');
    });
  });

  describe('user interactions', () => {
    it('should emit navItemClicked when a nav item is clicked', () => {
      const spy = jasmine.createSpy('navItemClicked');
      component.navItemClicked.subscribe(spy);

      const items: HTMLAnchorElement[] = fixture.nativeElement.querySelectorAll('.nav-item');
      items[0].click();

      expect(spy).toHaveBeenCalled();
    });

    it('should emit collapseClicked when collapse button is clicked', () => {
      const spy = jasmine.createSpy('collapseClicked');
      component.collapseClicked.subscribe(spy);

      const btn: HTMLButtonElement =
        fixture.nativeElement.querySelector('.sidebar-collapse-btn');
      btn.click();

      expect(spy).toHaveBeenCalled();
    });
  });

  describe('accessibility', () => {
    it('should have navigation role on aside element', () => {
      const aside = fixture.nativeElement.querySelector('aside');
      expect(aside.getAttribute('role')).toBe('navigation');
    });

    it('should have aria-label on aside element', () => {
      const aside = fixture.nativeElement.querySelector('aside');
      expect(aside.getAttribute('aria-label')).toBe('Shefa navigation');
    });

    it('should render collapse button with aria-label', () => {
      const btn = fixture.nativeElement.querySelector('.sidebar-collapse-btn');
      expect(btn).toBeTruthy();
      expect(btn.getAttribute('aria-label')).toBe('Collapse sidebar');
    });
  });
});
