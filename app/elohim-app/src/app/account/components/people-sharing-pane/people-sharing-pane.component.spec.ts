import { TestBed } from '@angular/core/testing';

import { PeopleSharingPaneComponent } from './people-sharing-pane.component';

describe('PeopleSharingPaneComponent', () => {
  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PeopleSharingPaneComponent],
    }).compileComponents();
  });

  it('should create', () => {
    const fixture = TestBed.createComponent(PeopleSharingPaneComponent);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('should render pane title with data-testid', () => {
    const fixture = TestBed.createComponent(PeopleSharingPaneComponent);
    fixture.detectChanges();
    const title = (fixture.nativeElement as HTMLElement).querySelector('[data-testid="pane-title-people-sharing"]');
    expect(title).toBeTruthy();
    expect(title?.textContent?.trim()).toBe('People & sharing');
  });

  it('should render M5 scaffold placeholder text', () => {
    const fixture = TestBed.createComponent(PeopleSharingPaneComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.textContent).toContain('M5 scaffold');
  });
});
