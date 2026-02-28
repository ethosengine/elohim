/**
 * Vitest spike — component test with TestBed.createComponent.
 *
 * Proves: fixture.detectChanges(), component interaction,
 * vi.fn() replacing jasmine.createSpyObj for service mocks.
 */
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { vi, describe, it, expect, beforeEach } from 'vitest';
import { of } from 'rxjs';

import { ThemeService } from '../../services/theme.service';
import { ThemeToggleComponent } from './theme-toggle.component';

describe('ThemeToggleComponent (vitest spike)', () => {
  let component: ThemeToggleComponent;
  let fixture: ComponentFixture<ThemeToggleComponent>;

  const mockThemeService = {
    getTheme: vi.fn(),
    cycleTheme: vi.fn(),
  };

  beforeEach(async () => {
    mockThemeService.getTheme.mockReturnValue(of('device'));
    mockThemeService.cycleTheme.mockClear();

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
    expect(component.getIcon()).toBe('\u2600\uFE0F');

    component.currentTheme = 'dark';
    expect(component.getIcon()).toBe('\uD83C\uDF19');
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
