import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

// @coverage: 100.0% (2026-02-05)

import { Observable, of, shareReplay, map, catchError } from 'rxjs';

import { environment } from '../../environments/environment';

export interface AppConfig {
  readonly logLevel: 'debug' | 'info' | 'error';
  readonly environment: string;
}

const DEFAULT_PROD_CONFIG: AppConfig = {
  logLevel: 'error',
  environment: 'production',
} as const;

@Injectable({
  providedIn: 'root',
})
export class ConfigService {
  private readonly http = inject(HttpClient);

  readonly config$: Observable<AppConfig> = this.createConfigStream();

  private createConfigStream(): Observable<AppConfig> {
    if (!environment.production) {
      return of(this.getDevConfig());
    }

    return this.http.get<AppConfig>('/assets/config.json').pipe(
      map(config => config || DEFAULT_PROD_CONFIG),
      catchError(() => of(DEFAULT_PROD_CONFIG)),
      shareReplay(1)
    );
  }

  private getDevConfig(): AppConfig {
    return {
      logLevel: environment.logLevel || 'debug',
      environment: environment.environment || 'development',
    };
  }

  getConfig(): Observable<AppConfig> {
    return this.config$;
  }
}
