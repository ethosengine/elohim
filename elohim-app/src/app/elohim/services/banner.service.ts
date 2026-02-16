/**
 * BannerService - Aggregation service for banner notifications.
 *
 * Merges notices from multiple providers, deduplicates by id,
 * sorts by priority then severity, and delegates dismiss/action
 * events back to the originating provider.
 */

import { Injectable } from '@angular/core';

import { map, switchMap } from 'rxjs/operators';

import { BehaviorSubject, combineLatest, Observable, of } from 'rxjs';

import {
  BannerContext,
  BannerNotice,
  BannerNoticeProvider,
  BANNER_PRIORITY_ORDER,
  BANNER_SEVERITY_ORDER,
} from '../models/banner-notice.model';

@Injectable({ providedIn: 'root' })
export class BannerService {
  private readonly providers = new Map<string, BannerNoticeProvider>();
  private readonly providers$ = new BehaviorSubject<Map<string, BannerNoticeProvider>>(
    this.providers
  );

  /**
   * All notices from all providers, deduplicated and sorted.
   */
  readonly allNotices$: Observable<BannerNotice[]> = this.providers$.pipe(
    switchMap(providerMap => {
      const streams = Array.from(providerMap.values()).map(p => p.notices$);
      if (streams.length === 0) {
        return of([]);
      }
      return combineLatest(streams).pipe(map(arrays => this.mergeAndSort(arrays)));
    })
  );

  /**
   * Register a notice provider.
   */
  registerProvider(provider: BannerNoticeProvider): void {
    this.providers.set(provider.providerId, provider);
    this.providers$.next(this.providers);
  }

  /**
   * Unregister a notice provider.
   */
  unregisterProvider(providerId: string): void {
    this.providers.delete(providerId);
    this.providers$.next(this.providers);
  }

  /**
   * Get notices filtered to a specific context (includes 'global' notices).
   */
  noticesForContext$(context: BannerContext): Observable<BannerNotice[]> {
    return this.allNotices$.pipe(
      map(notices =>
        notices.filter(n => n.contexts.includes('global') || n.contexts.includes(context))
      )
    );
  }

  /**
   * Dismiss a notice by delegating to its provider.
   */
  dismissNotice(notice: BannerNotice): void {
    const provider = this.providers.get(notice.providerId);
    provider?.dismissNotice(notice.id);
  }

  /**
   * Handle an action click by delegating to the notice's provider.
   */
  handleAction(notice: BannerNotice, actionId: string): void {
    const provider = this.providers.get(notice.providerId);
    provider?.handleAction(notice.id, actionId);
  }

  /**
   * Merge arrays from all providers, deduplicate by id, sort by priority then severity.
   */
  private mergeAndSort(arrays: BannerNotice[][]): BannerNotice[] {
    const seen = new Set<string>();
    const merged: BannerNotice[] = [];

    for (const arr of arrays) {
      for (const notice of arr) {
        if (!seen.has(notice.id)) {
          seen.add(notice.id);
          merged.push(notice);
        }
      }
    }

    return merged.sort((a, b) => {
      const priorityDiff = BANNER_PRIORITY_ORDER[a.priority] - BANNER_PRIORITY_ORDER[b.priority];
      if (priorityDiff !== 0) return priorityDiff;
      return BANNER_SEVERITY_ORDER[a.severity] - BANNER_SEVERITY_ORDER[b.severity];
    });
  }
}
