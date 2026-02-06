import { ComponentFixture, TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

import { ThemeService } from '../../services/theme.service';

import { ThemeToggleComponent } from './theme-toggle.component';

describe('ThemeToggleComponent', () => {
  let component: ThemeToggleComponent;
  let fixture: ComponentFixture<ThemeToggleComponent>;
  let mockThemeService: jasmine.SpyObj<ThemeService>;

  beforeEach(async () => {
    mockThemeService = jasmine.createSpyObj('ThemeService', ['getTheme', 'cycleTheme']);
    mockThemeService.getTheme.and.returnValue(of('device'));

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
