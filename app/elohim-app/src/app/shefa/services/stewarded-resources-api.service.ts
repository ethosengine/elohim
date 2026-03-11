/**
 * StewardedResourcesApiService — Thin HTTP client for resource stewardship.
 *
 * Calls doorway `/api/v1/resources/*` endpoints, implementing IStewardedResources.
 * Replaces the fat StewardedResourceService (Holochain zome calls) when the
 * business logic lives behind the Rust API boundary.
 */

import { HttpClient, HttpParams } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { catchError } from 'rxjs/operators';

import { firstValueFrom, of } from 'rxjs';

import type { IStewardedResources } from '../interfaces/stewarded-resources.interface';
import type {
  AllocationBlock,
  CategorySummary,
  ConstitutionalLimit,
  ResourceCategory,
  ResourceMeasure,
  StewardedResource,
  StewardedResourceDashboard,
  UsageRecord,
} from '@app/shefa/models/stewarded-resources.model';

const RESOURCES_URL = '/api/v1/resources';

@Injectable({ providedIn: 'root' })
export class StewardedResourcesApiService implements IStewardedResources {
  private readonly http = inject(HttpClient);

  async createResource(
    stewardId: string,
    category: ResourceCategory,
    subcategory: string,
    name: string,
    totalCapacity: ResourceMeasure,
    options?: {
      description?: string;
      governanceLevel?: string;
      observerEnabled?: boolean;
      permanentReserve?: ResourceMeasure;
    }
  ): Promise<StewardedResource> {
    return firstValueFrom(
      this.http.post<StewardedResource>(RESOURCES_URL, {
        stewardId,
        category,
        subcategory,
        name,
        totalCapacity,
        ...options,
      })
    );
  }

  async getResource(resourceId: string): Promise<StewardedResource | null> {
    return firstValueFrom(
      this.http
        .get<StewardedResource>(`/api/v1/resources/${resourceId}`)
        .pipe(catchError(() => of(null)))
    );
  }

  async getStewardResources(stewardId: string): Promise<StewardedResource[]> {
    return firstValueFrom(
      this.http
        .get<StewardedResource[]>(RESOURCES_URL, { params: { stewardId } })
        .pipe(catchError(() => of([])))
    );
  }

  async getResourcesByCategory(
    stewardId: string,
    category: ResourceCategory
  ): Promise<StewardedResource[]> {
    return firstValueFrom(
      this.http
        .get<StewardedResource[]>(RESOURCES_URL, { params: { stewardId, category } })
        .pipe(catchError(() => of([])))
    );
  }

  async createAllocation(
    resourceId: string,
    label: string,
    allocatedAmount: ResourceMeasure,
    options?: {
      description?: string;
      governanceLevel?: string;
      priority?: number;
    }
  ): Promise<AllocationBlock> {
    return firstValueFrom(
      this.http.post<AllocationBlock>(`/api/v1/resources/${resourceId}/allocate`, {
        label,
        allocatedAmount,
        ...options,
      })
    );
  }

  async recordUsage(
    resourceId: string,
    allocationBlockId: string,
    amount: ResourceMeasure,
    options?: {
      action?: string;
      observerAttestationId?: string;
      note?: string;
    }
  ): Promise<UsageRecord> {
    return firstValueFrom(
      this.http.post<UsageRecord>(`/api/v1/resources/${resourceId}/usage`, {
        allocationBlockId,
        amount,
        ...options,
      })
    );
  }

  async getCategorySummary(
    stewardId: string,
    category: ResourceCategory
  ): Promise<CategorySummary> {
    const params = new HttpParams().set('stewardId', stewardId).set('category', category);
    return firstValueFrom(
      this.http.get<CategorySummary>('/api/v1/resources/categories', { params })
    );
  }

  async buildDashboard(stewardId: string): Promise<StewardedResourceDashboard> {
    return firstValueFrom(
      this.http.get<StewardedResourceDashboard>(`/api/v1/resources/dashboard/${stewardId}`)
    );
  }

  async getConstitutionalLimits(category: ResourceCategory): Promise<ConstitutionalLimit | null> {
    return firstValueFrom(
      this.http
        .get<ConstitutionalLimit>(`/api/v1/resources/limits/${category}`)
        .pipe(catchError(() => of(null)))
    );
  }
}
