import {
  ChangeDetectionStrategy,
  Component,
  NgZone,
  provideZoneChangeDetection,
} from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { provideRouter, Router } from '@angular/router';
import { Subject } from 'rxjs';

import { AppComponent } from './app.component';

/**
 * Root-view change-detection regression (@regression).
 *
 * Angular 22 made OnPush the implicit change-detection default. The lamad
 * bundle's ROOT component is the only view attached to `ApplicationRef`, so its
 * strategy governs the whole tree: a zone-driven global tick refreshes an
 * `Eager` (CheckAlways) root and traverses into its children, but SKIPS an
 * OnPush root that nothing has marked dirty — and with it every descendant.
 *
 * When the root lost its `Eager` stamp in the v22 Eager-removal wave, the lamad
 * surface froze on "Loading content..." forever on the deployed alpha: the
 * doorway query returned, subscribers ran, state flipped, and no view was ever
 * checked again. Only signal-backed components (which mark themselves dirty and
 * ride the targeted refresh path) kept updating.
 *
 * This spec drives the browser's actual sequence — zone change detection as the
 * bundle provides it, a mutation delivered inside `NgZone.run()` (how a resolved
 * fetch re-enters the zone), and the assertion taken after the zone-driven tick
 * settles. It deliberately never calls `fixture.detectChanges()` after the
 * mutation: that forces an unconditional check and erases the exact distinction
 * OnPush enforces (the structural blindness recorded in
 * backlog-onpush-eager-debt-inventory).
 */
@Component({
  selector: 'lamad-cd-probe',
  standalone: true,
  template: `
    <span data-testid="probe">{{ value }}</span>
  `,
  // The probe itself must be Eager so the test isolates the ROOT's strategy.
  changeDetection: ChangeDetectionStrategy.Eager,
})
class CdProbeComponent {
  value = 'stale';
}

describe('AppComponent (lamad bundle root)', () => {
  it('propagates an out-of-band state change to the routed child on a zone-driven tick', async () => {
    const pulse$ = new Subject<string>();

    TestBed.configureTestingModule({
      imports: [AppComponent, CdProbeComponent],
      providers: [
        // Mirror the real bundle (app.config.ts) — TestBed defaults to zoneless,
        // which would mask a zone-driven change-detection regression entirely.
        provideZoneChangeDetection(),
        provideRouter([{ path: '**', component: CdProbeComponent }]),
      ],
    });

    const fixture = TestBed.createComponent(AppComponent);
    fixture.autoDetectChanges(true);
    await TestBed.inject(Router).navigateByUrl('/');
    await fixture.whenStable();

    const host = fixture.nativeElement as HTMLElement;
    const probe = fixture.debugElement.query(By.directive(CdProbeComponent))
      ?.componentInstance as CdProbeComponent;
    expect(probe).toBeTruthy();
    expect(host.querySelector('[data-testid="probe"]')?.textContent).toContain('stale');

    // A subscribe-style mutation: no markForCheck, no signal — exactly the shape
    // the whole bundle uses for doorway reads.
    TestBed.inject(NgZone).run(() => {
      pulse$.subscribe(v => (probe.value = v));
      pulse$.next('fresh');
    });
    await fixture.whenStable();

    expect(host.querySelector('[data-testid="probe"]')?.textContent).toContain('fresh');

    fixture.destroy();
  });
});
