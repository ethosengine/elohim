import { TestBed } from '@angular/core/testing';

import { DataPrivacyPaneComponent } from './data-privacy-pane.component';

describe('DataPrivacyPaneComponent', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DataPrivacyPaneComponent],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(DataPrivacyPaneComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render pane title with data-testid', () => {
    const fixture = TestBed.createComponent(DataPrivacyPaneComponent);
    fixture.detectChanges();
    const title = (fixture.nativeElement as HTMLElement).querySelector('[data-testid="pane-title-data-privacy"]');
    expect(title).toBeTruthy();
    expect(title?.textContent?.trim()).toBe('Data & privacy');
  });

  it('should render M5 scaffold placeholder text', () => {
    const fixture = TestBed.createComponent(DataPrivacyPaneComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.textContent).toContain('M5 scaffold');
  });
});
