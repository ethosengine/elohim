import {
  Component,
  ChangeDetectionStrategy,
  ElementRef,
  ViewChild,
  AfterViewInit,
  OnDestroy,
  signal,
  computed,
  inject,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { Subject, takeUntil } from 'rxjs';
import maplibregl from 'maplibre-gl';

import type { PlaceView, SpatialDashboardView, PlaceDashboardEntry } from '@elohim/storage-client';
import { PlacesApiService } from '../../services/places-api.service';
import { SpatialDashboardApiService } from '../../services/spatial-dashboard-api.service';

/** Color mapping for constitutional layers */
const LAYER_COLORS: Record<string, string> = {
  global: '#1a237e',
  bioregional: '#1b5e20',
  'nation-state': '#4a148c',
  provincial: '#e65100',
  community: '#00838f',
  family: '#bf360c',
  individual: '#455a64',
};

/** Risk tier sort order — lower index = more critical */
const RISK_TIER_ORDER: Record<string, number> = {
  critical: 0,
  elevated: 1,
  moderate: 2,
  low: 3,
};

/** Fill colors per risk tier for data-driven map styling */
const RISK_TIER_COLORS: Record<string, string> = {
  low: '#4CAF50',
  moderate: '#FFC107',
  elevated: '#FF9800',
  critical: '#F44336',
};

/** Hazard severity circle colors */
const HAZARD_SEVERITY_COLORS: Record<string, string> = {
  emergency: '#D32F2F',
  warning: '#FF9800',
  advisory: '#FFC107',
  watch: '#9E9E9E',
};

@Component({
  selector: 'app-spatial-map',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="map-wrapper">
      <div #mapContainer class="map"></div>

      @if (selectedPlace()) {
        <div class="place-panel">
          <button class="close-btn" data-testid="close-place-panel" aria-label="Close place panel" (click)="selectedPlace.set(null)">&times;</button>
          <h3>{{ selectedPlace()!.name }}</h3>
          <div class="badges">
            <span class="badge type">{{ selectedPlace()!.placeType }}</span>
            <span class="badge layer">{{ selectedPlace()!.constitutionalLayer }}</span>
            <span class="badge status">{{ selectedPlace()!.status }}</span>
          </div>
          @if (selectedPlace()!.carryingCapacity; as capacity) {
            <div class="capacity-section">
              <h4>Carrying Capacity</h4>
              @for (cap of asArray(capacity); track cap.resourceCategory) {
                <div class="capacity-row">
                  <span class="cap-label">{{ cap.resourceCategory }}</span>
                  <div class="cap-bar">
                    <div
                      class="cap-fill"
                      [style.width.%]="cap.currentUtilization * 100"
                      [class.warning]="cap.currentUtilization > 0.8"
                      [class.critical]="cap.currentUtilization > 1.0"
                    ></div>
                  </div>
                  <span class="cap-value">{{ (cap.currentUtilization * 100).toFixed(0) }}%</span>
                </div>
              }
            </div>
          }
        </div>
      }

      <!-- Dashboard sidebar panel -->
      @if (dashboardMode() && dashboardData(); as data) {
        <div class="dashboard-panel">
          <h3>Governance Dashboard</h3>

          <!-- Layer selector -->
          <div class="layer-selector">
            <label for="layer-select">Scope</label>
            <select id="layer-select" data-testid="layer-select" [value]="selectedLayer()" (change)="onLayerChange($event)">
              <option value="">All layers</option>
              <option value="global">Global</option>
              <option value="bioregional">Bioregional</option>
              <option value="nation-state">Nation-state</option>
              <option value="provincial">Provincial</option>
              <option value="community">Community</option>
              <option value="family">Family</option>
            </select>
          </div>

          <!-- Summary stats -->
          <div class="summary-stats">
            <div class="stat">
              <span class="stat-value">{{ data.summary.totalPlaces }}</span>
              <span class="stat-label">Places</span>
            </div>
            <div class="stat">
              <span class="stat-value">{{ data.summary.totalActiveHazards }}</span>
              <span class="stat-label">Hazards</span>
            </div>
            <div class="stat">
              <span class="stat-value">{{ data.summary.totalUnacknowledgedAlerts }}</span>
              <span class="stat-label">Alerts</span>
            </div>
          </div>

          <!-- Risk distribution bar -->
          <div class="risk-bar">
            <div class="risk-segment low" [style.flex]="data.summary.placesByRiskTier.low"></div>
            <div class="risk-segment moderate" [style.flex]="data.summary.placesByRiskTier.moderate"></div>
            <div class="risk-segment elevated" [style.flex]="data.summary.placesByRiskTier.elevated"></div>
            <div class="risk-segment critical" [style.flex]="data.summary.placesByRiskTier.critical"></div>
          </div>
          <div class="risk-legend">
            <span class="legend-item"><span class="dot low"></span> Low</span>
            <span class="legend-item"><span class="dot moderate"></span> Moderate</span>
            <span class="legend-item"><span class="dot elevated"></span> Elevated</span>
            <span class="legend-item"><span class="dot critical"></span> Critical</span>
          </div>

          <!-- Place list sorted by risk -->
          <div class="place-list">
            @for (entry of sortedPlaces(); track entry.id) {
              <button class="place-list-item" (click)="flyToPlace(entry)" data-testid="place-list-item">
                <span class="place-name">{{ entry.name }}</span>
                <div class="place-indicators">
                  @if (entry.vulnerability) {
                    <span class="risk-badge" [attr.data-tier]="entry.vulnerability.riskTier">
                      {{ entry.vulnerability.riskTier }}
                    </span>
                  }
                  @if (entry.hazards?.activeCount) {
                    <span class="hazard-count">{{ entry.hazards!.activeCount }}</span>
                  }
                  @if (entry.alerts?.unacknowledgedCount) {
                    <span class="alert-count">{{ entry.alerts!.unacknowledgedCount }}</span>
                  }
                </div>
              </button>
            }
          </div>
        </div>
      }

      <!-- Dashboard toggle button -->
      <button
        class="dashboard-toggle-btn"
        [class.active]="dashboardMode()"
        (click)="toggleDashboardMode()"
        data-testid="dashboard-toggle"
        aria-label="Toggle dashboard mode"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M3 13h8V3H3v10zm0 8h8v-6H3v6zm10 0h8V11h-8v10zm0-18v6h8V3h-8z"/>
        </svg>
      </button>

      <button class="locate-btn" data-testid="geolocate-btn" aria-label="Show my location" (click)="geolocate()">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
          <path d="M12 8c-2.21 0-4 1.79-4 4s1.79 4 4 4 4-1.79 4-4-1.79-4-4-4zm8.94 3A8.994 8.994 0 0013 3.06V1h-2v2.06A8.994 8.994 0 003.06 11H1v2h2.06A8.994 8.994 0 0011 20.94V23h2v-2.06A8.994 8.994 0 0020.94 13H23v-2h-2.06zM12 19c-3.87 0-7-3.13-7-7s3.13-7 7-7 7 3.13 7 7-3.13 7-7 7z"/>
        </svg>
      </button>
    </div>
  `,
  styleUrl: './spatial-map.component.css',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SpatialMapComponent implements AfterViewInit, OnDestroy {
  @ViewChild('mapContainer', { static: true }) mapContainer!: ElementRef<HTMLDivElement>;

  private readonly placesApi = inject(PlacesApiService);
  private readonly dashboardApi = inject(SpatialDashboardApiService);
  private readonly destroy$ = new Subject<void>();
  private map!: maplibregl.Map;

  selectedPlace = signal<PlaceView | null>(null);
  dashboardMode = signal(false);
  dashboardData = signal<SpatialDashboardView | null>(null);
  /** Constitutional layer filter — empty string means all layers */
  selectedLayer = signal<string>('');
  loading = signal(false);

  /** Places sorted by risk tier (critical first), for dashboard list */
  sortedPlaces = computed(() => {
    const data = this.dashboardData();
    if (!data) return [];
    return [...data.places].sort((a, b) => {
      const aT = RISK_TIER_ORDER[a.vulnerability?.riskTier ?? ''] ?? 4;
      const bT = RISK_TIER_ORDER[b.vulnerability?.riskTier ?? ''] ?? 4;
      return aT - bT;
    });
  });

  ngAfterViewInit(): void {
    this.initMap();
    this.loadPlaces();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.map?.remove();
  }

  toggleDashboardMode(): void {
    this.dashboardMode.update((v) => !v);
    if (this.dashboardMode()) {
      this.loadDashboard();
    } else {
      this.clearDashboardLayers();
      this.loadPlaces();
    }
  }

  onLayerChange(event: Event): void {
    this.selectedLayer.set((event.target as HTMLSelectElement).value);
    if (this.dashboardMode()) {
      this.loadDashboard();
    }
  }

  flyToPlace(entry: PlaceDashboardEntry): void {
    this.map.flyTo({ center: [entry.centroidLng, entry.centroidLat], zoom: 10 });
  }

  geolocate(): void {
    if (!navigator.geolocation) return;

    navigator.geolocation.getCurrentPosition(
      (pos) => {
        const { latitude, longitude } = pos.coords;
        this.map.flyTo({ center: [longitude, latitude], zoom: 14 });

        // Add/update user location marker
        const markerId = 'user-location';
        const existing = this.map.getSource(markerId);
        const point: GeoJSON.Point = { type: 'Point', coordinates: [longitude, latitude] };

        if (existing && 'setData' in existing) {
          (existing as maplibregl.GeoJSONSource).setData(point);
        } else {
          this.map.addSource(markerId, { type: 'geojson', data: point });
          this.map.addLayer({
            id: 'user-location-dot',
            type: 'circle',
            source: markerId,
            paint: {
              'circle-radius': 8,
              'circle-color': '#2196f3',
              'circle-stroke-width': 3,
              'circle-stroke-color': '#fff',
            },
          });
        }
      },
      (err) => console.warn('Geolocation failed:', err.message),
      { enableHighAccuracy: true },
    );
  }

  /** Helper to safely cast carrying capacity JSON to array */
  asArray(val: unknown): Array<{ resourceCategory: string; currentUtilization: number }> {
    return Array.isArray(val) ? val : [];
  }

  // ---------------------------------------------------------------------------
  // Private helpers
  // ---------------------------------------------------------------------------

  private initMap(): void {
    this.map = new maplibregl.Map({
      container: this.mapContainer.nativeElement,
      style: {
        version: 8,
        sources: {
          'osm-tiles': {
            type: 'raster',
            tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
            tileSize: 256,
            attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>',
          },
        },
        layers: [
          {
            id: 'osm-tiles',
            type: 'raster',
            source: 'osm-tiles',
            minzoom: 0,
            maxzoom: 19,
          },
        ],
      },
      center: [0, 20],
      zoom: 2,
    });

    this.map.addControl(new maplibregl.NavigationControl(), 'top-right');

    // Click handler for place boundaries — works in both normal and dashboard mode
    this.map.on('click', 'place-fills', (e: maplibregl.MapLayerMouseEvent) => {
      if (e.features?.length) {
        const props = e.features[0].properties;
        this.placesApi
          .getById(props?.['id'] as string)
          .pipe(takeUntil(this.destroy$))
          .subscribe((place) => this.selectedPlace.set(place));
      }
    });

    // Cursor change on hover
    this.map.on('mouseenter', 'place-fills', () => {
      this.map.getCanvas().style.cursor = 'pointer';
    });
    this.map.on('mouseleave', 'place-fills', () => {
      this.map.getCanvas().style.cursor = '';
    });
  }

  private loadPlaces(): void {
    this.placesApi
      .list({ status: 'active' })
      .pipe(takeUntil(this.destroy$))
      .subscribe((places) => {
        if (!places.length) return;
        this.addPlaceLayers(places);
      });
  }

  private addPlaceLayers(places: PlaceView[]): void {
    const features = places
      .filter((p) => p.geometryJson)
      .map((p) => ({
        type: 'Feature' as const,
        properties: {
          id: p.id,
          name: p.name,
          placeType: p.placeType,
          constitutionalLayer: p.constitutionalLayer,
          status: p.status,
          color: LAYER_COLORS[p.constitutionalLayer] ?? '#607d8b',
          riskTier: 'unknown',
        },
        geometry: p.geometryJson,
      }));

    const geojson = { type: 'FeatureCollection' as const, features };

    const addLayers = () => {
      if (this.map.getSource('places')) return;

      this.map.addSource('places', { type: 'geojson', data: geojson as unknown as GeoJSON.FeatureCollection });

      this.map.addLayer({
        id: 'place-fills',
        type: 'fill',
        source: 'places',
        paint: {
          'fill-color': ['get', 'color'],
          'fill-opacity': 0.15,
        },
      });

      this.map.addLayer({
        id: 'place-borders',
        type: 'line',
        source: 'places',
        paint: {
          'line-color': ['get', 'color'],
          'line-width': 2,
          'line-opacity': 0.7,
        },
      });

      this.map.addLayer({
        id: 'place-labels',
        type: 'symbol',
        source: 'places',
        layout: {
          'text-field': ['get', 'name'],
          'text-size': 12,
          'text-anchor': 'center',
        },
        paint: {
          'text-color': '#333',
          'text-halo-color': '#fff',
          'text-halo-width': 1,
        },
      });
    };

    if (this.map.loaded()) {
      addLayers();
    } else {
      this.map.on('load', addLayers);
    }
  }

  loadDashboard(): void {
    this.loading.set(true);
    const layer = this.selectedLayer();
    const query = layer ? { constitutionalLayer: layer } : undefined;

    this.dashboardApi
      .getDashboard(query)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: (data) => {
          this.dashboardData.set(data);
          this.addDashboardLayers(data);
          this.loading.set(false);
        },
        error: (err) => {
          console.error('[SpatialMapComponent] Dashboard load failed:', err);
          this.loading.set(false);
        },
      });
  }

  private addDashboardLayers(data: SpatialDashboardView): void {
    const applyLayers = () => {
      // Remove existing place layers and source to replace with risk-colored versions
      this.removePlaceLayers();

      // Build GeoJSON features with risk tier in properties
      const features = data.places
        .filter((p) => p.geometryJson)
        .map((p) => ({
          type: 'Feature' as const,
          properties: {
            id: p.id,
            name: p.name,
            placeType: p.placeType,
            constitutionalLayer: p.constitutionalLayer,
            status: p.status,
            color: LAYER_COLORS[p.constitutionalLayer] ?? '#607d8b',
            riskTier: p.vulnerability?.riskTier ?? 'unknown',
            hazardCount: p.hazards?.activeCount ?? 0,
            alertCount: p.alerts?.unacknowledgedCount ?? 0,
          },
          geometry: p.geometryJson,
        }));

      const geojson = { type: 'FeatureCollection' as const, features };

      this.map.addSource('places', { type: 'geojson', data: geojson as unknown as GeoJSON.FeatureCollection });

      // Fill layer with data-driven risk tier colors
      this.map.addLayer({
        id: 'place-fills',
        type: 'fill',
        source: 'places',
        paint: {
          'fill-color': [
            'match',
            ['get', 'riskTier'],
            'low', RISK_TIER_COLORS['low'],
            'moderate', RISK_TIER_COLORS['moderate'],
            'elevated', RISK_TIER_COLORS['elevated'],
            'critical', RISK_TIER_COLORS['critical'],
            '#9E9E9E', // unknown/default
          ],
          'fill-opacity': 0.25,
        },
      });

      this.map.addLayer({
        id: 'place-borders',
        type: 'line',
        source: 'places',
        paint: {
          'line-color': ['get', 'color'],
          'line-width': 2,
          'line-opacity': 0.7,
        },
      });

      this.map.addLayer({
        id: 'place-labels',
        type: 'symbol',
        source: 'places',
        layout: {
          'text-field': ['get', 'name'],
          'text-size': 12,
          'text-anchor': 'center',
        },
        paint: {
          'text-color': '#333',
          'text-halo-color': '#fff',
          'text-halo-width': 1,
        },
      });

      // Hazard markers — circle layer for places with active hazards
      const hazardFeatures = data.places
        .filter((p) => (p.hazards?.activeCount ?? 0) > 0)
        .map((p) => ({
          type: 'Feature' as const,
          properties: {
            id: p.id,
            activeCount: p.hazards!.activeCount,
            worstSeverity: p.hazards!.worstSeverity,
          },
          geometry: {
            type: 'Point' as const,
            coordinates: [p.centroidLng, p.centroidLat],
          },
        }));

      if (hazardFeatures.length > 0) {
        const hazardGeoJson = {
          type: 'FeatureCollection' as const,
          features: hazardFeatures,
        };

        this.map.addSource('hazard-markers', {
          type: 'geojson',
          data: hazardGeoJson as unknown as GeoJSON.FeatureCollection,
        });

        this.map.addLayer({
          id: 'hazard-markers',
          type: 'circle',
          source: 'hazard-markers',
          paint: {
            // Radius scales from 8 to 14 based on active count, capped at 20
            'circle-radius': [
              'interpolate',
              ['linear'],
              ['min', ['get', 'activeCount'], 20],
              1, 8,
              20, 14,
            ],
            'circle-color': [
              'match',
              ['get', 'worstSeverity'],
              'emergency', HAZARD_SEVERITY_COLORS['emergency'],
              'warning', HAZARD_SEVERITY_COLORS['warning'],
              'advisory', HAZARD_SEVERITY_COLORS['advisory'],
              'watch', HAZARD_SEVERITY_COLORS['watch'],
              '#9E9E9E',
            ],
            'circle-stroke-width': 2,
            'circle-stroke-color': '#fff',
            'circle-opacity': 0.85,
          },
        });
      }
    };

    if (this.map.loaded()) {
      applyLayers();
    } else {
      this.map.on('load', applyLayers);
    }
  }

  private clearDashboardLayers(): void {
    if (this.map.getLayer('hazard-markers')) this.map.removeLayer('hazard-markers');
    if (this.map.getSource('hazard-markers')) this.map.removeSource('hazard-markers');
    this.removePlaceLayers();
  }

  /** Remove the three place layers and their shared source */
  private removePlaceLayers(): void {
    if (this.map.getLayer('place-labels')) this.map.removeLayer('place-labels');
    if (this.map.getLayer('place-borders')) this.map.removeLayer('place-borders');
    if (this.map.getLayer('place-fills')) this.map.removeLayer('place-fills');
    if (this.map.getSource('places')) this.map.removeSource('places');
  }
}
