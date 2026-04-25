import { TestBed } from '@angular/core/testing';

import { ThirdPartyAppsPaneComponent } from './third-party-apps-pane.component';

describe('ThirdPartyAppsPaneComponent', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ThirdPartyAppsPaneComponent],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(ThirdPartyAppsPaneComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render pane title with data-testid', () => {
    const fixture = TestBed.createComponent(ThirdPartyAppsPaneComponent);
    fixture.detectChanges();
    const title = (fixture.nativeElement as HTMLElement).querySelector('[data-testid="pane-title-third-party-apps"]');
    expect(title).toBeTruthy();
    expect(title?.textContent?.trim()).toBe('Third-party apps');
  });

  it('should render M5 scaffold placeholder text', () => {
    const fixture = TestBed.createComponent(ThirdPartyAppsPaneComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.textContent).toContain('M5 scaffold');
  });
});
