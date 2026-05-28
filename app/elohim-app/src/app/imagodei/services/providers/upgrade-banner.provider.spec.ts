import { TestBed } from '@angular/core/testing';

import { firstValueFrom } from 'rxjs';

import { BannerService } from '@app/elohim/services/banner.service';
import { SessionHumanService } from '../session-human.service';
import { UpgradeBannerProvider } from './upgrade-banner.provider';
import { vi, Mock } from 'vitest';

/**
 * M-AGGR-1: UpgradeBannerProvider now derives prompts from the substrate
 * projection at GET /api/v1/identity/{agentId}/upgrade-prompts (UpgradePromptView).
 * The local upgradePrompts$ subject has been removed from SessionHumanService.
 *
 * Until the substrate Observable is wired, the provider emits empty notices.
 * These tests verify structure, registration, and dismissal behaviour.
 */
describe('UpgradeBannerProvider', () => {
  let provider: UpgradeBannerProvider;
  let mockBannerService: any;

  beforeEach(() => {
    const mockSessionHumanService = {
      dismissUpgradePrompt: vi.fn(),
      getDismissedPromptIds: vi.fn().mockReturnValue([]),
    };

    mockBannerService = {
      registerProvider: vi.fn(),
      unregisterProvider: vi.fn(),
    };

    TestBed.configureTestingModule({
      providers: [
        UpgradeBannerProvider,
        { provide: SessionHumanService, useValue: mockSessionHumanService },
        { provide: BannerService, useValue: mockBannerService },
      ],
    });

    provider = TestBed.inject(UpgradeBannerProvider);
  });

  it('should be created', () => {
    expect(provider).toBeTruthy();
  });

  it('should self-register with BannerService', () => {
    expect(mockBannerService.registerProvider).toHaveBeenCalledWith(provider);
  });

  it('should have providerId "upgrade-banner"', () => {
    expect(provider.providerId).toBe('upgrade-banner');
  });

  it('should emit empty notices until substrate observable is wired (M-AGGR-1)', async () => {
    const notices = await firstValueFrom(provider.notices$);
    expect(notices).toEqual([]);
  });

  it('should delegate dismissNotice to SessionHumanService', () => {
    const mockService = TestBed.inject(SessionHumanService) as {
      [K in keyof SessionHumanService]?: Mock;
    };
    provider.dismissNotice('prompt-1');
    expect(mockService.dismissUpgradePrompt).toHaveBeenCalledWith('prompt-1');
  });

  it('should emit on upgradeModalRequested$ when learn-more action is handled', () =>
    new Promise<void>(done => {
      provider.upgradeModalRequested$.subscribe(() => {
        done();
      });

      provider.handleAction('prompt-1', 'learn-more');
    }));

  it('should not emit on upgradeModalRequested$ for unknown actions', () => {
    let emitted = false;
    const sub = provider.upgradeModalRequested$.subscribe(() => {
      emitted = true;
    });

    provider.handleAction('prompt-1', 'unknown-action');

    expect(emitted).toBe(false);
    sub.unsubscribe();
  });
});
