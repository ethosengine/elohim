/**
 * RegisterComponent — spec
 *
 * The route is a REDIRECTOR to the doorway's own registration page. There is no
 * form here, so these tests assert the hand-off (`prompt=create` via
 * `initiateRegistration`), the fallback to doorway discovery, and — the
 * invariant this consolidation exists for — that nothing in this surface
 * collects or posts a credential.
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';
import { ActivatedRoute, Router, provideRouter } from '@angular/router';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { RegisterComponent } from './register.component';
import { DoorwayRegistryService } from '../../services/doorway-registry.service';
import { OAuthAuthProvider } from '../../services/providers/oauth-auth.provider';

describe('RegisterComponent (portal redirector)', () => {
  let fixture: ComponentFixture<RegisterComponent>;
  let mockOAuthProvider: {
    initiateRegistration: ReturnType<typeof vi.fn>;
    storeReturnUrl: ReturnType<typeof vi.fn>;
  };
  let mockDoorwayRegistry: { selectedUrl: ReturnType<typeof signal<string | null>> };
  let mockActivatedRoute: { queryParams: ReturnType<typeof of> };
  let router: Router;

  const build = (): ComponentFixture<RegisterComponent> => {
    const created = TestBed.createComponent(RegisterComponent);
    created.detectChanges();
    return created;
  };

  beforeEach(async () => {
    mockOAuthProvider = {
      initiateRegistration: vi.fn(),
      storeReturnUrl: vi.fn(),
    };
    mockDoorwayRegistry = { selectedUrl: signal<string | null>('https://alpha.elohim.host') };
    mockActivatedRoute = { queryParams: of({}) };

    await TestBed.configureTestingModule({
      imports: [RegisterComponent],
      providers: [
        provideRouter([]),
        { provide: OAuthAuthProvider, useValue: mockOAuthProvider },
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
      ],
    }).compileComponents();

    router = TestBed.inject(Router);
    vi.spyOn(router, 'navigate').mockResolvedValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('sends the human to the doorway portal to create the account', () => {
    fixture = build();

    expect(mockOAuthProvider.initiateRegistration).toHaveBeenCalledWith(
      'https://alpha.elohim.host',
      expect.stringContaining('/auth/callback'),
      undefined
    );
  });

  it('stores the return URL so the callback brings them back', () => {
    mockActivatedRoute.queryParams = of({ returnUrl: '/lamad/path/abc' });

    fixture = build();

    expect(mockOAuthProvider.storeReturnUrl).toHaveBeenCalledWith('/lamad/path/abc');
  });

  it('offers a remembered identifier to the portal as the login hint', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockReturnValue('ruth@alpha.elohim.host');

    fixture = build();

    expect(mockOAuthProvider.initiateRegistration).toHaveBeenCalledWith(
      'https://alpha.elohim.host',
      expect.stringContaining('/auth/callback'),
      'ruth@alpha.elohim.host'
    );
  });

  it('falls back to doorway discovery when no doorway has proved itself', () => {
    mockDoorwayRegistry.selectedUrl = signal<string | null>(null);

    fixture = build();

    expect(mockOAuthProvider.initiateRegistration).not.toHaveBeenCalled();
    expect(router.navigate).toHaveBeenCalledWith(['/identity/login'], {
      queryParams: { returnUrl: '/' },
    });
  });

  it('says so, rather than failing silently, when the hand-off throws', () => {
    mockOAuthProvider.initiateRegistration.mockImplementation(() => {
      throw new Error('sessionStorage unavailable');
    });

    fixture = build();

    expect(fixture.componentInstance.message()).toContain('sessionStorage unavailable');
  });

  it('renders no form, no password field, and no submit control', () => {
    fixture = build();
    const el = fixture.nativeElement as HTMLElement;

    expect(el.querySelector('form')).toBeNull();
    expect(el.querySelector('input')).toBeNull();
    expect(el.querySelector('input[type="password"]')).toBeNull();
    expect(el.querySelector('button')).toBeNull();
  });
});
