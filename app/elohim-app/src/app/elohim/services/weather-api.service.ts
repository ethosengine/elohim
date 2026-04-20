import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

import { Observable } from 'rxjs';

import type { WeatherForecastView } from '@elohim/storage-client';

@Injectable({ providedIn: 'root' })
export class WeatherApiService {
  private readonly http = inject(HttpClient);
  private readonly baseUrl = '';

  getForecast(placeId: string): Observable<WeatherForecastView> {
    return this.http.get<WeatherForecastView>(`${this.baseUrl}/api/v1/weather/${placeId}`);
  }
}
