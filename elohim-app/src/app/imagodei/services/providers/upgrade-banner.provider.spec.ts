import { TestBed } from '@angular/core/testing';

import { BehaviorSubject, firstValueFrom } from 'rxjs';

import { BannerService } from '@app/elohim/services/banner.service';
import { HolochainUpgradePrompt } from '../../models/session-human.model';
import { SessionHumanService } from '../session-human.service';
import { UpgradeBannerProvider } from './upgrade-banner.provider';

describe('UpgradeBannerProvider', () => {
  let provider: UpgradeBannerProvider;
  let mockBannerService: jasmine.SpyObj<BannerService>;
  let upgradePromptsSubject: BehaviorSubject<HolochainUpgradePrompt[]>;

  beforeEach(() => {
    upgradePromptsSubject = new BehaviorSubject<HolochainUpgradePrompt[]>([]);

    const mockSessionHumanService = jasmine.createSpyObj(
      'SessionHumanService',
      ['dismissUpgradePrompt'],
      {
        upgradePrompts$: upgradePromptsSubject.asObservable(),
      }
    );

    mockBannerService = jasmine.createSpyObj('BannerService', [
      'registerProvider',
      'unregisterProvider',
    ]);

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

  it('should map upgrade prompts to banner notices', async () => {
    const prompt: HolochainUpgradePrompt = {
      id: 'prompt-1',
      trigger: 'first-affinity',
      title: 'Save Your Progress',
      message: 'Join the network to keep your data',
      benefits: ['Permanent storage'],
      dismissed: false,
    };

    upgradePromptsSubject.next([prompt]);

    const notices = await firstValueFrom(provider.notices$);
    expect(notices.length).toBe(1);
    expect(notices[0].id).toBe('prompt-1');
    expect(notices[0].title).toBe('Save Your Progress');
    expect(notices[0].severity).toBe('info');
    expect(notices[0].priority).toBe('agent');
    expect(notices[0].contexts).toEqual(['global']);
    expect(notices[0].actions?.length).toBe(1);
    expect(notices[0].actions?.[0].id).toBe('learn-more');
  });

  it('should map progress-at-risk prompts with higher urgency', async () => {
    const prompt: HolochainUpgradePrompt = {
      id: 'urgent-1',
      trigger: 'progress-at-risk',
      title: 'Storage Almost Full',
      message: 'Your progress may be lost',
      benefits: [],
      dismissed: false,
    };

    upgradePromptsSubject.next([prompt]);

    const notices = await firstValueFrom(provider.notices$);
    expect(notices[0].severity).toBe('warning');
    expect(notices[0].priority).toBe('system');
  });

  it('should filter out dismissed prompts', async () => {
    const prompts: HolochainUpgradePrompt[] = [
      {
        id: 'active',
        trigger: 'first-affinity',
        title: 'Active',
        message: '',
        benefits: [],
        dismissed: false,
      },
      {
        id: 'dismissed',
        trigger: 'path-started',
        title: 'Dismissed',
        message: '',
        benefits: [],
        dismissed: true,
      },
    ];

    upgradePromptsSubject.next(prompts);

    const notices = await firstValueFrom(provider.notices$);
    expect(notices.length).toBe(1);
    expect(notices[0].id).toBe('active');
  });

  it('should delegate dismissNotice to SessionHumanService', () => {
    const mockService = TestBed.inject(SessionHumanService) as jasmine.SpyObj<SessionHumanService>;
    provider.dismissNotice('prompt-1');
    expect(mockService.dismissUpgradePrompt).toHaveBeenCalledWith('prompt-1');
  });

  it('should emit on upgradeModalRequested$ when learn-more action is handled', done => {
    provider.upgradeModalRequested$.subscribe(() => {
      done();
    });

    provider.handleAction('prompt-1', 'learn-more');
  });

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
