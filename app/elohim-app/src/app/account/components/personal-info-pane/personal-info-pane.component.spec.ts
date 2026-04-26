import { TestBed } from '@angular/core/testing';

import { PersonalInfoPaneComponent } from './personal-info-pane.component';

describe('PersonalInfoPaneComponent', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PersonalInfoPaneComponent],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(PersonalInfoPaneComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render pane title with data-testid', () => {
    const fixture = TestBed.createComponent(PersonalInfoPaneComponent);
    fixture.detectChanges();
    const title = (fixture.nativeElement as HTMLElement).querySelector('[data-testid="pane-title-personal-info"]');
    expect(title).toBeTruthy();
    expect(title?.textContent?.trim()).toBe('Personal info');
  });

  it('should render M5 scaffold placeholder text', () => {
    const fixture = TestBed.createComponent(PersonalInfoPaneComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.textContent).toContain('M5 scaffold');
  });
});
