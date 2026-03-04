/**
 * Angular Router mock factories for unit testing
 */
import { ActivatedRouteSnapshot, ParamMap } from '@angular/router';

import { BehaviorSubject } from 'rxjs';
import { type Mock, vi } from 'vitest';

// ============================================================================
// Router Mock
// ============================================================================

export interface MockRouter {
  navigate: Mock;
  navigateByUrl: Mock;
  events: BehaviorSubject<unknown>;
  url: string;
}

export function createMockRouter(): MockRouter {
  return {
    navigate: vi.fn().mockReturnValue(Promise.resolve(true)),
    navigateByUrl: vi.fn().mockReturnValue(Promise.resolve(true)),
    events: new BehaviorSubject<unknown>(null),
    url: '/',
  };
}

// ============================================================================
// ActivatedRoute Mock
// ============================================================================

export interface MockActivatedRoute {
  params: BehaviorSubject<Record<string, string>>;
  queryParams: BehaviorSubject<Record<string, string>>;
  snapshot: Partial<ActivatedRouteSnapshot>;
  paramMap: BehaviorSubject<ParamMap>;
  data: BehaviorSubject<Record<string, unknown>>;
}

export function createMockActivatedRoute(
  params: Record<string, string> = {},
  queryParams: Record<string, string> = {},
  data: Record<string, unknown> = {}
): MockActivatedRoute {
  const paramMap = createParamMap(params);

  return {
    params: new BehaviorSubject(params),
    queryParams: new BehaviorSubject(queryParams),
    paramMap: new BehaviorSubject(paramMap),
    data: new BehaviorSubject(data),
    snapshot: {
      params,
      queryParams,
      paramMap,
      data,
    } as Partial<ActivatedRouteSnapshot>,
  };
}

// ============================================================================
// ParamMap Helper
// ============================================================================

function createParamMap(params: Record<string, string>): ParamMap {
  return {
    has: (key: string) => key in params,
    get: (key: string) => params[key] || null,
    getAll: (key: string) => (params[key] ? [params[key]] : []),
    keys: Object.keys(params),
  };
}
