import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ThemeToggleComponent } from './theme-toggle.component';
import { ThemeService } from '../../services/theme.service';

describe('ThemeToggleComponent', () => {
  let component: ThemeToggleComponent;
  let fixture: ComponentFixture<ThemeToggleComponent>;
  let themeService: ThemeService;

  beforeEach(async () => {
    localStorage.clear();
    await TestBed.configureTestingModule({
      imports: [ThemeToggleComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ThemeToggleComponent);
    component = fixture.componentInstance;
    themeService = TestBed.inject(ThemeService);
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should display auto mode by default', () => {
    expect(component.isAutoMode()).toBe(true);
  });

  it('should cycle theme on toggle', () => {
    component.toggleTheme();
    expect(themeService.getCurrentTheme()).toBe('light');

    component.toggleTheme();
    expect(themeService.getCurrentTheme()).toBe('dark');

    component.toggleTheme();
    expect(themeService.getCurrentTheme()).toBe('device');
  });

  it('should return appropriate tooltip', () => {
    themeService.setTheme('device');
    fixture.detectChanges();
    expect(component.getTooltip()).toContain('Auto mode');

    themeService.setTheme('light');
    fixture.detectChanges();
    expect(component.getTooltip()).toContain('Light mode');

    themeService.setTheme('dark');
    fixture.detectChanges();
    expect(component.getTooltip()).toContain('Dark mode');
  });
});
