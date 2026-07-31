/** @vitest-environment jsdom */

import '@analogjs/vitest-angular/setup-zone';
import '@angular/compiler';

import {
  ChangeDetectionStrategy,
  Component,
  NgZone,
  provideZoneChangeDetection,
} from '@angular/core';
import { getTestBed, TestBed } from '@angular/core/testing';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';
import { Subject } from 'rxjs';

import { AppComponent } from './app.component.js';

getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting());

const productionRootChangeDetection = (AppComponent as unknown as { ɵcmp: { onPush: boolean } })
  .ɵcmp.onPush
  ? ChangeDetectionStrategy.OnPush
  : ChangeDetectionStrategy.Eager;

@Component({
  selector: 'imagodei-root-cd-probe',
  standalone: true,
  template: `<span data-testid="probe">{{ value }}</span>`,
  // Keep the child Eager so this regression isolates the bundle ROOT strategy.
  changeDetection: ChangeDetectionStrategy.Eager,
})
class RootCdProbeComponent {
  value = 'stale';
}

describe('AppComponent (imagodei portal bundle root)', () => {
  it('propagates an out-of-band child update on a zone-driven tick', async () => {
    const pulse$ = new Subject<string>();

    globalThis.fetch = async () => new Response('not found', { status: 404 });

    await TestBed.configureTestingModule({
      imports: [AppComponent, RootCdProbeComponent],
      providers: [provideZoneChangeDetection()],
    })
      .overrideComponent(AppComponent, {
        set: {
          // Template overrides otherwise fall back to Eager. Carry the real
          // root's compiled strategy into the isolated descendant harness.
          changeDetection: productionRootChangeDetection,
          imports: [RootCdProbeComponent],
          template: `<imagodei-root-cd-probe />`,
        },
      })
      .compileComponents();

    const fixture = TestBed.createComponent(AppComponent);
    fixture.autoDetectChanges(true);
    await fixture.whenStable();

    const host = fixture.nativeElement as HTMLElement;
    const probe = fixture.debugElement.children[0].componentInstance as RootCdProbeComponent;
    expect(host.querySelector('[data-testid="probe"]')?.textContent).toContain('stale');

    TestBed.inject(NgZone).run(() => {
      pulse$.subscribe((value) => (probe.value = value));
      pulse$.next('fresh');
    });
    await fixture.whenStable();

    expect(host.querySelector('[data-testid="probe"]')?.textContent).toContain('fresh');
  });
});
