import { TestBed } from '@angular/core/testing';
import {
  ActivatedRouteSnapshot,
  Router,
  RouterStateSnapshot,
  UrlTree,
  provideRouter,
} from '@angular/router';
import { describe, expect, it } from 'vitest';

import { AuthStateService } from '../../services/auth-state.service';

import { authGuard, safeReturnUrl } from './auth.guard';

describe('authGuard', () => {
  function run(signedIn: boolean, url: string): unknown {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [
        provideRouter([]),
        {
          provide: AuthStateService,
          useValue: { isLoading: () => false, isAuthenticated: () => signedIn },
        },
      ],
    });
    return TestBed.runInInjectionContext(() =>
      authGuard({} as ActivatedRouteSnapshot, { url } as RouterStateSnapshot)
    );
  }

  it('lets a signed-in person through', async () => {
    await expect(run(true, '/account')).resolves.toBe(true);
  });

  it('sends a signed-out person to sign in, remembering the page they asked for', async () => {
    const url = '/consent/device?request=eyJhIjoxfQ';
    const result = (await run(false, url)) as UrlTree;
    const router = TestBed.inject(Router);

    expect(result).toBeInstanceOf(UrlTree);
    expect(router.serializeUrl(result)).toBe(`/login?returnUrl=${encodeURIComponent(url)}`);
    expect(result.queryParams['returnUrl']).toBe(url);
  });
});

describe('safeReturnUrl', () => {
  it('keeps an in-app path', () => {
    expect(safeReturnUrl('/consent/device?request=abc')).toBe('/consent/device?request=abc');
  });

  it.each([
    ['an absolute URL', 'https://evil.example/'],
    ['a protocol-relative URL', '//evil.example/'],
    ['a backslash trick', '/\\evil.example'],
    ['a relative path', 'dashboard'],
    ['nothing', undefined],
  ])('refuses %s', (_label, value) => {
    expect(safeReturnUrl(value)).toBeNull();
  });
});
