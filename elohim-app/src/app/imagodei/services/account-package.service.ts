/**
 * AccountPackageService - Import/export account packages for human worlds.
 *
 * An account package is a portable, self-contained representation of a human's world:
 * - Identity (who they are)
 * - Content assignments (what they see, at what reach)
 * - Relationships (who they know)
 * - Stewardship (what they care for)
 *
 * This service supports:
 * - Genesis seeding (initial conditions for test humans)
 * - Account recovery (restoring from backup)
 * - Migration (moving between conductors)
 *
 * Uses the /account/* endpoints on elohim-storage (proxied via doorway).
 */

import { HttpClient } from '@angular/common/http';
import { Injectable } from '@angular/core';

import { Observable, catchError, throwError } from 'rxjs';

import { environment } from '../../../environments/environment';

import type {
  AccountImportResultView,
  AccountPackageInputView,
  AccountPackageView,
} from '@elohim/storage-client/generated';

// Re-export generated types for consumer convenience
export type {
  AccountImportResultView,
  AccountPackageInputView,
  AccountPackageView,
  AccountIdentityView,
  ContentAssignmentView,
  RelationshipSeedView,
  StewardshipSeedView,
  PackageManifestView,
} from '@elohim/storage-client/generated';

@Injectable({
  providedIn: 'root',
})
export class AccountPackageService {
  /** Base URL for storage API (doorway proxies to elohim-storage) */
  private readonly baseUrl: string;

  constructor(private readonly http: HttpClient) {
    this.baseUrl =
      environment.holochain?.storageUrl ??
      (environment as unknown as { client?: { doorwayUrl?: string } }).client?.doorwayUrl ??
      '';
  }

  /**
   * Import an account package into elohim-storage.
   *
   * Orchestrates content reach updates, relationship creation,
   * and stewardship allocation creation for the given human.
   */
  importPackage(pkg: AccountPackageInputView): Observable<AccountImportResultView> {
    return this.http.post<AccountImportResultView>(`${this.baseUrl}/account/import`, pkg).pipe(
      catchError((error: unknown) => {
        console.error('[AccountPackageService] Import failed:', error);
        return throwError(() => error);
      })
    );
  }

  /**
   * Export an account package for a human.
   *
   * Assembles the human's current world state into a portable package:
   * content assignments, relationships, stewardship allocations.
   */
  exportPackage(humanId: string): Observable<AccountPackageView> {
    return this.http
      .get<AccountPackageView>(`${this.baseUrl}/account/export/${encodeURIComponent(humanId)}`)
      .pipe(
        catchError((error: unknown) => {
          console.error('[AccountPackageService] Export failed:', error);
          return throwError(() => error);
        })
      );
  }
}
