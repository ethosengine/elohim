import { TestBed } from '@angular/core/testing';
import { ThemeService } from './theme.service';

describe('ThemeService', () => {
  let service: ThemeService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(ThemeService);
    localStorage.clear();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should default to device theme', () => {
    expect(service.getCurrentTheme()).toBe('device');
  });

  it('should cycle through themes', () => {
    service.setTheme('device');
    expect(service.getCurrentTheme()).toBe('device');

    service.cycleTheme();
    expect(service.getCurrentTheme()).toBe('light');

    service.cycleTheme();
    expect(service.getCurrentTheme()).toBe('dark');

    service.cycleTheme();
    expect(service.getCurrentTheme()).toBe('device');
  });

  it('should save theme to localStorage', () => {
    service.setTheme('dark');
    expect(localStorage.getItem('elohim-theme')).toBe('dark');
  });

  it('should apply theme class to body', () => {
    service.setTheme('light');
    expect(document.body.classList.contains('theme-light')).toBe(true);
    expect(document.body.getAttribute('data-theme')).toBe('light');
  });
});
