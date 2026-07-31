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
import { AuthStateService } from './services/auth-state.service';

@Component({
  selector: 'app-root-cd-probe',
  standalone: true,
  template: `
    <span data-testid="probe">{{ value }}</span>
  `,
  // Keep the child Eager so this regression isolates the bundle ROOT strategy.
  changeDetection: ChangeDetectionStrategy.Eager,
})
class RootCdProbeComponent {
  value = 'stale';
}

describe('AppComponent (doorway bundle root)', () => {
  it('propagates an out-of-band child update on a zone-driven tick', async () => {
    const pulse$ = new Subject<string>();

    await TestBed.configureTestingModule({
      imports: [AppComponent, RootCdProbeComponent],
      providers: [
        // Mirror app.config.ts; zoneless TestBed would mask this regression.
        provideZoneChangeDetection(),
        provideRouter([{ path: '**', component: RootCdProbeComponent }]),
        {
          provide: AuthStateService,
          useValue: {
            init: () => undefined,
            logout: () => Promise.resolve(),
            isLoading: () => false,
            isAuthenticated: () => false,
            isSteward: () => false,
            account: () => null,
            initials: () => '?',
            modeLabel: () => 'Hosted',
          },
        },
      ],
    }).compileComponents();

    const fixture = TestBed.createComponent(AppComponent);
    fixture.autoDetectChanges(true);
    await TestBed.inject(Router).navigateByUrl('/probe');
    await fixture.whenStable();

    const host = fixture.nativeElement as HTMLElement;
    const probe = fixture.debugElement.query(By.directive(RootCdProbeComponent))
      .componentInstance as RootCdProbeComponent;
    expect(host.querySelector('[data-testid="probe"]')?.textContent).toContain('stale');

    TestBed.inject(NgZone).run(() => {
      pulse$.subscribe(value => (probe.value = value));
      pulse$.next('fresh');
    });
    await fixture.whenStable();

    expect(host.querySelector('[data-testid="probe"]')?.textContent).toContain('fresh');
  });
});
