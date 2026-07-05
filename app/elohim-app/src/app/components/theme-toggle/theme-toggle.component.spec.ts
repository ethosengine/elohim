import { ComponentFixture, TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

import { ThemeService } from '../../services/theme.service';

import { ThemeToggleComponent } from './theme-toggle.component';
import { vi } from 'vitest';

describe('ThemeToggleComponent', () => {
  let component: ThemeToggleComponent;
  let fixture: ComponentFixture<ThemeToggleComponent>;
  let mockThemeService: any;

  beforeEach(async () => {
    mockThemeService = {
      getTheme: vi.fn(),
      cycleTheme: vi.fn(),
    };
    mockThemeService.getTheme.mockReturnValue(of('device'));

    await TestBed.configureTestingModule({
      imports: [ThemeToggleComponent],
      providers: [{ provide: ThemeService, useValue: mockThemeService }],
    }).compileComponents();

    fixture = TestBed.createComponent(ThemeToggleComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should display sun or moon icon based on effective theme', () => {
    component.currentTheme = 'light';
    expect(component.getIcon()).toBe('☀️');

    component.currentTheme = 'dark';
    expect(component.getIcon()).toBe('🌙');
  });

  it('getIcon does not throw and defaults to moon when matchMedia is unavailable (SSR)', () => {
    // The V8 SSR runtime has no `matchMedia`; getIcon() is reached synchronously
    // via the template during the server render. Simulate its absence and prove
    // the guard returns a default instead of throwing (the empty-render class).
    const original = globalThis.matchMedia;
    (globalThis as unknown as { matchMedia?: unknown }).matchMedia = undefined;
    try {
      component.currentTheme = 'device';
      expect(() => component.getIcon()).not.toThrow();
      expect(component.getIcon()).toBe('🌙');
    } finally {
      globalThis.matchMedia = original;
    }
  });

  it('should show auto mode indicator when in device mode', () => {
    component.currentTheme = 'device';
    expect(component.isAutoMode()).toBe(true);

    component.currentTheme = 'light';
    expect(component.isAutoMode()).toBe(false);
  });

  it('should call cycleTheme when toggled', () => {
    component.toggleTheme();
    expect(mockThemeService.cycleTheme).toHaveBeenCalled();
  });
});
