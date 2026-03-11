/**
 * DataProtectionApiService — Thin HTTP client for data protection monitoring.
 *
 * Calls doorway `/api/v1/custodians/protection/*` endpoints, implementing
 * IDataProtection. Maintains a local cache of protection status for
 * synchronous accessor methods.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import {
  BehaviorSubject,
  Observable,
  Subscription,
  firstValueFrom,
  switchMap,
  tap,
  timer,
} from 'rxjs';

import type {
  CustodianNode,
  CustodianType,
  FamilyCommunityProtectionStatus,
  IDataProtection,
  RegionalPresence,
} from '../interfaces/data-protection.interface';

@Injectable({ providedIn: 'root' })
export class DataProtectionApiService implements IDataProtection {
  private readonly http = inject(HttpClient);
  private readonly status$ = new BehaviorSubject<FamilyCommunityProtectionStatus | null>(
    null
  );
  private pollingSubscription: Subscription | null = null;

  /**
   * Fetch protection status once from the API and cache it.
   */
  async fetchProtectionStatus(
    operatorId: string
  ): Promise<FamilyCommunityProtectionStatus> {
    const status = await firstValueFrom(
      this.http.get<FamilyCommunityProtectionStatus>(
        `/api/v1/custodians/protection/${operatorId}/summary`
      )
    );
    this.status$.next(status);
    return status;
  }

  initializeProtectionMonitoring(
    operatorId: string,
    refreshInterval = 60000
  ): Observable<FamilyCommunityProtectionStatus> {
    this.pollingSubscription?.unsubscribe();

    const poll$ = timer(0, refreshInterval).pipe(
      switchMap(() =>
        this.http.get<FamilyCommunityProtectionStatus>(
          `/api/v1/custodians/protection/${operatorId}/summary`
        )
      ),
      tap((status) => this.status$.next(status))
    );

    this.pollingSubscription = poll$.subscribe();

    return poll$;
  }

  getProtectionStatus(): FamilyCommunityProtectionStatus | null {
    return this.status$.getValue();
  }

  getProtectionStatus$(): Observable<FamilyCommunityProtectionStatus | null> {
    return this.status$.asObservable();
  }

  getCustodiansByType(type: CustodianType): CustodianNode[] {
    const status = this.status$.getValue();
    if (!status) return [];
    return status.custodians.filter((c) => c.type === type);
  }

  getHighRiskRegions(): RegionalPresence[] {
    const status = this.status$.getValue();
    if (!status) return [];
    return status.geographicDistribution.regions.filter(
      (r) => r.riskFactors.length > 0 || r.custodianCount <= 1
    );
  }

  isCustodianHealthy(custodianId: string): boolean {
    const status = this.status$.getValue();
    if (!status) return false;
    const custodian = status.custodians.find((c) => c.id === custodianId);
    return custodian ? custodian.health.upPercent >= 95 : false;
  }

  getAverageUptime(): number {
    const status = this.status$.getValue();
    if (!status || status.custodians.length === 0) return 0;
    const total = status.custodians.reduce((sum, c) => sum + c.health.upPercent, 0);
    return total / status.custodians.length;
  }

  getProtectionAlerts(): string[] {
    const status = this.status$.getValue();
    if (!status) return [];
    const alerts: string[] = [];

    if (status.protectionLevel === 'vulnerable') {
      alerts.push('Protection level is vulnerable — add more custodians');
    }

    const unhealthy = status.custodians.filter((c) => c.health.upPercent < 95);
    for (const c of unhealthy) {
      alerts.push(`Custodian ${c.name} uptime is ${c.health.upPercent}%`);
    }

    const riskyRegions = this.getHighRiskRegions();
    for (const r of riskyRegions) {
      alerts.push(`Region ${r.region} has elevated risk: ${r.riskFactors.join(', ')}`);
    }

    return alerts;
  }
}
