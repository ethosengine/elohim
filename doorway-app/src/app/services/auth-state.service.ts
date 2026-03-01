import { Injectable, inject, signal, computed } from '@angular/core';
import { Router } from '@angular/router';
import { firstValueFrom } from 'rxjs';

import { DoorwayAdminService } from './doorway-admin.service';
import { AccountResponse } from '../models/doorway.model';

@Injectable({ providedIn: 'root' })
export class AuthStateService {
  private readonly adminService = inject(DoorwayAdminService);
  private readonly router = inject(Router);

  private readonly _account = signal<AccountResponse | null>(null);
  private readonly _isLoading = signal(true);

  readonly account = this._account.asReadonly();
  readonly isLoading = this._isLoading.asReadonly();
  readonly isAuthenticated = computed(() => this._account() !== null);
  readonly isSteward = computed(() => this._account()?.isSteward ?? false);
  readonly modeLabel = computed<'Steward' | 'Hosted'>(() =>
    this._account()?.isSteward ? 'Steward' : 'Hosted'
  );
  readonly initials = computed(() => {
    const id = this._account()?.identifier;
    return id ? id.charAt(0).toUpperCase() : '?';
  });

  async init(): Promise<void> {
    this._isLoading.set(true);
    try {
      const account = await firstValueFrom(this.adminService.getAccount());
      this._account.set(account);
    } finally {
      this._isLoading.set(false);
    }
  }

  async refresh(): Promise<void> {
    const account = await firstValueFrom(this.adminService.getAccount());
    this._account.set(account);
  }

  async logout(): Promise<void> {
    await firstValueFrom(this.adminService.logout());
    this._account.set(null);
    this.router.navigate(['/']);
  }
}
