import { InjectionToken } from '@angular/core';

import type {
  ContentAttestationView,
  CreateAttestationInputView,
} from '@elohim/storage-client/generated';

export interface IContentAttestation {
  createAttestation(input: CreateAttestationInputView): Promise<ContentAttestationView>;
  revokeAttestation(id: string, revocation?: Record<string, unknown>): Promise<void>;
  getAttestation(id: string): Promise<ContentAttestationView | null>;
  queryAttestationsForContent(contentId: string): Promise<ContentAttestationView[]>;
  queryAttestationsByAttestor(attestorId: string): Promise<ContentAttestationView[]>;
}

export const CONTENT_ATTESTATION = new InjectionToken<IContentAttestation>('ContentAttestation');
