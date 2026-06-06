import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { RouterTestingHarness } from '@angular/router/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { LAMAD_EPR_NAV } from '../../interfaces/cross-pillar.interface';
import { LegacyResourceRedirectComponent } from './legacy-resource-redirect.component';

describe('LegacyResourceRedirectComponent', () => {
  const eprNav = { navigate: vi.fn(), ownsPath: vi.fn(() => false), recordHandoff: vi.fn() };

  beforeEach(() => {
    eprNav.navigate.mockClear();
    TestBed.configureTestingModule({
      providers: [
        provideRouter([
          { path: 'resource/:resourceId', component: LegacyResourceRedirectComponent },
        ]),
        { provide: LAMAD_EPR_NAV, useValue: eprNav },
      ],
    });
  });

  it('bridges the legacy /lamad/resource URL to the universal address', async () => {
    const harness = await RouterTestingHarness.create();
    await harness.navigateByUrl('/resource/fct-module-01-church-dilemma');
    expect(eprNav.navigate).toHaveBeenCalledWith('/epr/fct-module-01-church-dilemma');
  });
});
