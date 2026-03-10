/**
 * ContentAttestationApiService — Thin HTTP client for attestation endpoints.
 *
 * Calls doorway `/api/v1/attestations/*` endpoints, implementing IContentAttestation.
 */

import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';
import { firstValueFrom, of } from 'rxjs';

import type {
  ContentAttestationView,
  CreateAttestationInputView,
} from '@elohim/storage-client/generated';

import type { IContentAttestation } from '../interfaces/content-attestation.interface';

@Injectable({ providedIn: 'root' })
export class ContentAttestationApiService implements IContentAttestation {
  private readonly http = inject(HttpClient);

  async createAttestation(input: CreateAttestationInputView): Promise<ContentAttestationView> {
    return firstValueFrom(
      this.http.post<ContentAttestationView>('/api/v1/attestations', input)
    );
  }

  async revokeAttestation(id: string, revocation?: Record<string, unknown>): Promise<void> {
    return firstValueFrom(
      this.http.post<void>(`/api/v1/attestations/${encodeURIComponent(id)}/revoke`, {
        revocation,
      })
    );
  }

  async getAttestation(id: string): Promise<ContentAttestationView | null> {
    return firstValueFrom(
      this.http
        .get<ContentAttestationView>(`/api/v1/attestations/${encodeURIComponent(id)}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async queryAttestationsForContent(contentId: string): Promise<ContentAttestationView[]> {
    return firstValueFrom(
      this.http
        .get<ContentAttestationView[]>('/api/v1/attestations', {
          params: { contentId: encodeURIComponent(contentId) },
        })
        .pipe(catchError(() => of([])))
    );
  }

  async queryAttestationsByAttestor(attestorId: string): Promise<ContentAttestationView[]> {
    return firstValueFrom(
      this.http
        .get<ContentAttestationView[]>('/api/v1/attestations', {
          params: { attestorId: encodeURIComponent(attestorId) },
        })
        .pipe(catchError(() => of([])))
    );
  }
}
