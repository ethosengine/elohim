import { InjectionToken } from '@angular/core';

import type {
  ContributorDashboardView,
  ContributorImpactView,
  ContributorRecognitionView,
} from '@elohim/storage-client/generated';

export interface IContributor {
  getDashboard(presenceId: string): Promise<ContributorDashboardView | null>;
  getMyDashboard(): Promise<ContributorDashboardView | null>;
  getImpact(presenceId: string): Promise<ContributorImpactView>;
  getRecognitionHistory(presenceId: string): Promise<ContributorRecognitionView>;
}

export const CONTRIBUTOR = new InjectionToken<IContributor>('Contributor');
