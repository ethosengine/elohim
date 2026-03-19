import { vi } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NO_ERRORS_SCHEMA } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { of, throwError } from 'rxjs';

import { SpatialMapComponent } from './spatial-map.component';
import { PlacesApiService } from '../../services/places-api.service';
import { SpatialDashboardApiService } from '../../services/spatial-dashboard-api.service';
import type { SpatialDashboardView, PlaceDashboardEntry } from '@elohim/storage-client';

// ---------------------------------------------------------------------------
// MapLibre GL mock — prevents real canvas/WebGL operations in jsdom
// ---------------------------------------------------------------------------
vi.mock('maplibre-gl', () => {
  const mockMap = {
    addControl: vi.fn(),
    on: vi.fn(),
    getCanvas: vi.fn(() => ({ style: {} })),
    loaded: vi.fn(() => true),
    addSource: vi.fn(),
    addLayer: vi.fn(),
    getSource: vi.fn(() => null),
    getLayer: vi.fn(() => null),
    removeLayer: vi.fn(),
    removeSource: vi.fn(),
    flyTo: vi.fn(),
    remove: vi.fn(),
  };
  return {
    default: {
      Map: vi.fn(() => mockMap),
      NavigationControl: vi.fn(() => ({})),
    },
  };
});

// ---------------------------------------------------------------------------
// Test data
// ---------------------------------------------------------------------------

function makeDashboardEntry(overrides: Partial<PlaceDashboardEntry> = {}): PlaceDashboardEntry {
  return {
    id: 'place-1',
    name: 'Test Place',
    placeType: 'city',
    constitutionalLayer: 'community',
    h3Index: '8928308280fffff',
    centroidLat: 45.0,
    centroidLng: -75.0,
    geometryJson: null,
    status: 'active',
    parentPlaceId: null,
    ...overrides,
  };
}

function makeDashboardView(entries: PlaceDashboardEntry[] = []): SpatialDashboardView {
  return {
    places: entries,
    summary: {
      totalPlaces: entries.length,
      placesByRiskTier: { low: 0, moderate: 0, elevated: 0, critical: 0, unknown: entries.length },
      totalActiveHazards: 0,
      totalActiveAlerts: 0,
      totalUnacknowledgedAlerts: 0,
      worstOverallRisk: 'unknown',
    },
    queriedAt: new Date().toISOString(),
  };
}

// ---------------------------------------------------------------------------
// Suite
// ---------------------------------------------------------------------------

describe('SpatialMapComponent', () => {
  let component: SpatialMapComponent;
  let fixture: ComponentFixture<SpatialMapComponent>;
  let placesApiMock: { list: ReturnType<typeof vi.fn>; getById: ReturnType<typeof vi.fn> };
  let dashboardApiMock: { getDashboard: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    placesApiMock = {
      list: vi.fn(() => of([])),
      getById: vi.fn(() => of(null)),
    };

    dashboardApiMock = {
      getDashboard: vi.fn(() => of(makeDashboardView())),
    };

    await TestBed.configureTestingModule({
      imports: [SpatialMapComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: PlacesApiService, useValue: placesApiMock },
        { provide: SpatialDashboardApiService, useValue: dashboardApiMock },
      ],
      schemas: [NO_ERRORS_SCHEMA],
    }).compileComponents();

    fixture = TestBed.createComponent(SpatialMapComponent);
    component = fixture.componentInstance;
    // Do not call detectChanges here — ngAfterViewInit would run initMap() which
    // depends on a real DOM canvas element not available in jsdom.
  });

  // -------------------------------------------------------------------------
  // Creation
  // -------------------------------------------------------------------------

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // -------------------------------------------------------------------------
  // Initial signal state
  // -------------------------------------------------------------------------

  it('starts with dashboardMode off', () => {
    expect(component.dashboardMode()).toBe(false);
  });

  it('starts with no dashboardData', () => {
    expect(component.dashboardData()).toBeNull();
  });

  it('starts with empty selectedLayer', () => {
    expect(component.selectedLayer()).toBe('');
  });

  it('starts with loading false', () => {
    expect(component.loading()).toBe(false);
  });

  it('starts with no selectedPlace', () => {
    expect(component.selectedPlace()).toBeNull();
  });

  // -------------------------------------------------------------------------
  // sortedPlaces computed
  // -------------------------------------------------------------------------

  it('sortedPlaces returns empty array when dashboardData is null', () => {
    expect(component.sortedPlaces()).toEqual([]);
  });

  it('sortedPlaces orders critical before low', () => {
    const critical = makeDashboardEntry({
      id: 'c',
      name: 'Critical Place',
      vulnerability: { overallScore: 0.9, riskTier: 'critical', preparationStatus: 'unprepared' },
    });
    const low = makeDashboardEntry({
      id: 'l',
      name: 'Low Place',
      vulnerability: { overallScore: 0.1, riskTier: 'low', preparationStatus: 'prepared' },
    });
    component.dashboardData.set(makeDashboardView([low, critical]));

    const sorted = component.sortedPlaces();
    expect(sorted[0].id).toBe('c');
    expect(sorted[1].id).toBe('l');
  });

  it('sortedPlaces puts entries with no vulnerability at the end', () => {
    const withRisk = makeDashboardEntry({
      id: 'r',
      vulnerability: { overallScore: 0.5, riskTier: 'moderate', preparationStatus: 'partial' },
    });
    const noRisk = makeDashboardEntry({ id: 'n' });
    component.dashboardData.set(makeDashboardView([noRisk, withRisk]));

    const sorted = component.sortedPlaces();
    expect(sorted[0].id).toBe('r');
    expect(sorted[1].id).toBe('n');
  });

  it('sortedPlaces does not mutate the original places array', () => {
    const entry1 = makeDashboardEntry({ id: 'a' });
    const entry2 = makeDashboardEntry({ id: 'b' });
    const data = makeDashboardView([entry1, entry2]);
    component.dashboardData.set(data);
    component.sortedPlaces();

    // Original order should be unchanged
    expect(data.places[0].id).toBe('a');
    expect(data.places[1].id).toBe('b');
  });

  // -------------------------------------------------------------------------
  // toggleDashboardMode
  // -------------------------------------------------------------------------

  it('toggleDashboardMode flips dashboardMode from false to true', () => {
    // Prevent map operations by stubbing the private methods
    vi.spyOn(component as any, 'loadDashboard').mockImplementation(() => {});
    component.toggleDashboardMode();
    expect(component.dashboardMode()).toBe(true);
  });

  it('toggleDashboardMode calls loadDashboard when turning on', () => {
    const spy = vi.spyOn(component as any, 'loadDashboard').mockImplementation(() => {});
    component.toggleDashboardMode();
    expect(spy).toHaveBeenCalledOnce();
  });

  it('toggleDashboardMode calls clearDashboardLayers and loadPlaces when turning off', () => {
    component.dashboardMode.set(true);
    const clearSpy = vi.spyOn(component as any, 'clearDashboardLayers').mockImplementation(() => {});
    const loadSpy = vi.spyOn(component as any, 'loadPlaces').mockImplementation(() => {});
    component.toggleDashboardMode();
    expect(component.dashboardMode()).toBe(false);
    expect(clearSpy).toHaveBeenCalledOnce();
    expect(loadSpy).toHaveBeenCalledOnce();
  });

  // -------------------------------------------------------------------------
  // onLayerChange
  // -------------------------------------------------------------------------

  it('onLayerChange updates selectedLayer signal', () => {
    vi.spyOn(component as any, 'loadDashboard').mockImplementation(() => {});
    const event = { target: { value: 'bioregional' } } as unknown as Event;
    component.onLayerChange(event);
    expect(component.selectedLayer()).toBe('bioregional');
  });

  it('onLayerChange triggers loadDashboard when dashboardMode is on', () => {
    component.dashboardMode.set(true);
    const spy = vi.spyOn(component as any, 'loadDashboard').mockImplementation(() => {});
    const event = { target: { value: 'global' } } as unknown as Event;
    component.onLayerChange(event);
    expect(spy).toHaveBeenCalledOnce();
  });

  it('onLayerChange does not trigger loadDashboard when dashboardMode is off', () => {
    component.dashboardMode.set(false);
    const spy = vi.spyOn(component as any, 'loadDashboard').mockImplementation(() => {});
    const event = { target: { value: 'global' } } as unknown as Event;
    component.onLayerChange(event);
    expect(spy).not.toHaveBeenCalled();
  });

  // -------------------------------------------------------------------------
  // loadDashboard
  // -------------------------------------------------------------------------

  it('loadDashboard passes constitutionalLayer when selectedLayer is set', () => {
    component.selectedLayer.set('bioregional');
    vi.spyOn(component as any, 'addDashboardLayers').mockImplementation(() => {});
    component.loadDashboard();
    expect(dashboardApiMock.getDashboard).toHaveBeenCalledWith({ constitutionalLayer: 'bioregional' });
  });

  it('loadDashboard passes undefined query when selectedLayer is empty', () => {
    component.selectedLayer.set('');
    vi.spyOn(component as any, 'addDashboardLayers').mockImplementation(() => {});
    component.loadDashboard();
    expect(dashboardApiMock.getDashboard).toHaveBeenCalledWith(undefined);
  });

  it('loadDashboard sets dashboardData on success', () => {
    const data = makeDashboardView([makeDashboardEntry()]);
    dashboardApiMock.getDashboard.mockReturnValue(of(data));
    vi.spyOn(component as any, 'addDashboardLayers').mockImplementation(() => {});

    component.loadDashboard();

    expect(component.dashboardData()).toEqual(data);
    expect(component.loading()).toBe(false);
  });

  it('loadDashboard clears loading on API error', () => {
    dashboardApiMock.getDashboard.mockReturnValue(throwError(() => new Error('network error')));

    component.loadDashboard();

    expect(component.loading()).toBe(false);
    expect(component.dashboardData()).toBeNull();
  });

  // -------------------------------------------------------------------------
  // asArray helper
  // -------------------------------------------------------------------------

  it('asArray returns the value when it is an array', () => {
    const arr = [{ resourceCategory: 'water', currentUtilization: 0.5 }];
    expect(component.asArray(arr)).toBe(arr);
  });

  it('asArray returns empty array for non-array values', () => {
    expect(component.asArray(null)).toEqual([]);
    expect(component.asArray(undefined)).toEqual([]);
    expect(component.asArray({ foo: 'bar' })).toEqual([]);
  });
});
