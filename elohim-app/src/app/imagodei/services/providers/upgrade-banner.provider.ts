/**
 * UpgradeBannerProvider - Maps session upgrade prompts to BannerNotices.
 *
 * Subscribes to SessionHumanService.upgradePrompts$ and converts
 * HolochainUpgradePrompt instances into generic BannerNotice objects.
 * Self-registers with BannerService on construction.
 */

import { Injectable, OnDestroy } from '@angular/core';

import { map, takeUntil } from 'rxjs/operators';

import { BehaviorSubject, Observable, Subject } from 'rxjs';

import {
  BannerNotice,
  BannerNoticeProvider,
  BannerPriority,
} from '@app/elohim/models/banner-notice.model';
import { BannerService } from '@app/elohim/services/banner.service';

import { AlertSeverity } from '../../../shared/components/alert-banner';
import { HolochainUpgradePrompt } from '../../models/session-human.model';
import { SessionHumanService } from '../session-human.service';

const PROVIDER_ID = 'upgrade-banner';

@Injectable({ providedIn: 'root' })
export class UpgradeBannerProvider implements BannerNoticeProvider, OnDestroy {
  readonly providerId = PROVIDER_ID;

  private readonly destroy$ = new Subject<void>();
  private readonly noticesSubject = new BehaviorSubject<BannerNotice[]>([]);
  readonly notices$: Observable<BannerNotice[]> = this.noticesSubject.asObservable();

  /** Emits when "Learn More" action is clicked, so the navigator can open the modal */
  private readonly upgradeModalRequestedSubject = new Subject<void>();
  readonly upgradeModalRequested$ = this.upgradeModalRequestedSubject.asObservable();

  constructor(
    private readonly sessionHumanService: SessionHumanService,
    private readonly bannerService: BannerService
  ) {
    // Subscribe to upgrade prompts and map to BannerNotices
    this.sessionHumanService.upgradePrompts$
      .pipe(
        map(prompts => prompts.filter(p => !p.dismissed).map(p => this.mapPromptToNotice(p))),
        takeUntil(this.destroy$)
      )
      .subscribe(notices => this.noticesSubject.next(notices));

    // Self-register with the banner service
    this.bannerService.registerProvider(this);
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.bannerService.unregisterProvider(this.providerId);
  }

  dismissNotice(noticeId: string): void {
    this.sessionHumanService.dismissUpgradePrompt(noticeId);
  }

  handleAction(_noticeId: string, actionId: string): void {
    if (actionId === 'learn-more') {
      this.upgradeModalRequestedSubject.next();
    }
  }

  private mapPromptToNotice(prompt: HolochainUpgradePrompt): BannerNotice {
    const isUrgent = prompt.trigger === 'progress-at-risk';

    const severity: AlertSeverity = isUrgent ? 'warning' : 'info';
    const priority: BannerPriority = isUrgent ? 'system' : 'agent';

    return {
      id: prompt.id,
      providerId: PROVIDER_ID,
      severity,
      priority,
      contexts: ['global'],
      title: prompt.title,
      message: prompt.message,
      actions: [
        {
          id: 'learn-more',
          label: 'Learn More',
          variant: 'primary',
        },
      ],
      dismissible: true,
      createdAt: new Date(),
      metadata: {
        trigger: prompt.trigger,
        benefits: prompt.benefits,
      },
    };
  }
}
