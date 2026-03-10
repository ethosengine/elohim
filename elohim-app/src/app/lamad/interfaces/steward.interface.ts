import { InjectionToken } from '@angular/core';

import type {
  StewardCredentialView,
  PremiumGateView,
  AccessGrantView,
  StewardRevenueSummaryView,
  CreateCredentialInputView,
  CreateGateInputView,
  CreateGrantInputView,
} from '@elohim/storage-client/generated';

export interface ISteward {
  createCredential(input: CreateCredentialInputView): Promise<StewardCredentialView>;
  getCredential(id: string): Promise<StewardCredentialView | null>;
  queryCredentialsForPresence(presenceId: string): Promise<StewardCredentialView[]>;
  queryCredentialsForContent(contentId: string): Promise<StewardCredentialView[]>;
  createGate(input: CreateGateInputView): Promise<PremiumGateView>;
  getGate(id: string): Promise<PremiumGateView | null>;
  queryGatesForResource(resourceType: string): Promise<PremiumGateView[]>;
  createGrant(input: CreateGrantInputView): Promise<AccessGrantView>;
  getGrant(id: string): Promise<AccessGrantView | null>;
  queryGrantsForGate(gateId: string): Promise<AccessGrantView[]>;
  checkAccess(gateId: string, granteeId: string): Promise<boolean>;
  getRevenueSummary(presenceId: string): Promise<StewardRevenueSummaryView>;
}

export const STEWARD_OPERATIONS = new InjectionToken<ISteward>('StewardOperations');
