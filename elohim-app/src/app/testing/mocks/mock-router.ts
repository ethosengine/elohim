/**
 * Angular Router mock factories for unit testing
 */

import { ActivatedRoute, ActivatedRouteSnapshot, ParamMap, Router } from '@angular/router';

import { BehaviorSubject, of } from 'rxjs';

// ============================================================================
// Router Mock
// ============================================================================

export interface MockRouter {
  navigate: jasmine.Spy;
  navigateByUrl: jasmine.Spy;
  events: BehaviorSubject<unknown>;
  url: string;
}

export function createMockRouter(): MockRouter {
  const mock = jasmine.createSpyObj('Router', ['navigate', 'navigateByUrl']) as MockRouter;

  mock.navigate.and.returnValue(Promise.resolve(true));
  mock.navigateByUrl.and.returnValue(Promise.resolve(true));
  mock.events = new BehaviorSubject<unknown>(null);
  mock.url = '/';

  return mock;
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

// ============================================================================
// Usage Example
// ============================================================================

/*
  TestBed.configureTestingModule({
    providers: [
      { provide: Router, useValue: createMockRouter() },
      { provide: ActivatedRoute, useValue: createMockActivatedRoute(
        { id: '123' },           // route params
        { filter: 'active' },   // query params
        { title: 'Page Title' } // route data
      )}
    ]
  });
*/
