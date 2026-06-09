import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import { PinProgressComponent } from './pin-progress.component';

describe('PinProgressComponent', () => {
  let fixture: ComponentFixture<PinProgressComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PinProgressComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(PinProgressComponent);
  });

  it('renders the fetched/total fraction and percent', () => {
    fixture.componentInstance.total = 4;
    fixture.componentInstance.fetched = 1;
    fixture.componentInstance.pending = 3;
    fixture.componentInstance.failed = 0;
    fixture.componentInstance.caughtUp = false;
    fixture.detectChanges();

    const frac = fixture.nativeElement.querySelector('[data-testid="pin-progress-fraction"]');
    expect(frac?.textContent).toContain('1');
    expect(frac?.textContent).toContain('4');

    const pct = fixture.nativeElement.querySelector('[data-testid="pin-progress-percent"]');
    expect(pct?.textContent).toMatch(/25/);
  });

  it('shows the caught-up badge when caughtUp is true', () => {
    fixture.componentInstance.total = 2;
    fixture.componentInstance.fetched = 2;
    fixture.componentInstance.pending = 0;
    fixture.componentInstance.failed = 0;
    fixture.componentInstance.caughtUp = true;
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector('[data-testid="pin-progress-caught-up"]')
    ).toBeTruthy();
  });

  it('surfaces a failed count when failures occur', () => {
    fixture.componentInstance.total = 3;
    fixture.componentInstance.fetched = 1;
    fixture.componentInstance.pending = 1;
    fixture.componentInstance.failed = 1;
    fixture.componentInstance.caughtUp = false;
    fixture.detectChanges();

    const failed = fixture.nativeElement.querySelector('[data-testid="pin-progress-failed"]');
    expect(failed).toBeTruthy();
    expect(failed?.textContent).toContain('1');
  });

  it('renders a waiting state (no percent, no caught-up) when total is null', () => {
    fixture.componentInstance.total = null;
    fixture.componentInstance.fetched = 0;
    fixture.componentInstance.pending = 0;
    fixture.componentInstance.failed = 0;
    fixture.componentInstance.caughtUp = null;
    fixture.detectChanges();

    // Null total = "state not computable yet" — never claim caught up.
    expect(
      fixture.nativeElement.querySelector('[data-testid="pin-progress-caught-up"]')
    ).toBeFalsy();
    expect(
      fixture.nativeElement.querySelector('[data-testid="pin-progress-waiting"]')
    ).toBeTruthy();
  });

  it('never renders the caught-up badge while total is null even if caughtUp is true', () => {
    // The null≠done contract: a stale/contradictory caughtUp must not surface
    // "serving" when the desired set is not yet computable.
    fixture.componentInstance.total = null;
    fixture.componentInstance.fetched = 0;
    fixture.componentInstance.pending = 0;
    fixture.componentInstance.failed = 0;
    fixture.componentInstance.caughtUp = true;
    fixture.detectChanges();

    expect(
      fixture.nativeElement.querySelector('[data-testid="pin-progress-caught-up"]')
    ).toBeFalsy();
    expect(
      fixture.nativeElement.querySelector('[data-testid="pin-progress-waiting"]')
    ).toBeTruthy();
  });

  it('emits cancel and retry events from the host controls', () => {
    fixture.componentInstance.total = 3;
    fixture.componentInstance.fetched = 1;
    fixture.componentInstance.pending = 1;
    fixture.componentInstance.failed = 1;
    fixture.componentInstance.caughtUp = false;
    fixture.detectChanges();

    let cancelled = 0;
    let retried = 0;
    fixture.componentInstance.cancel.subscribe(() => (cancelled += 1));
    fixture.componentInstance.retry.subscribe(() => (retried += 1));

    fixture.nativeElement.querySelector('[data-testid="pin-progress-retry"]')?.click();
    fixture.nativeElement.querySelector('[data-testid="pin-progress-cancel"]')?.click();

    expect(retried).toBe(1);
    expect(cancelled).toBe(1);
  });
});
