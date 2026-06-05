import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';

// @coverage: 100.0% (2026-02-24)

import { Observable, of, shareReplay, map, catchError } from 'rxjs';

import { environment } from '../../environments/environment';

export interface AppConfig {
  readonly logLevel: 'debug' | 'info' | 'warn' | 'error';
  readonly environment: string;
  /** CI-substituted git commit hash (ServingContext.buildId source). */
  readonly gitHash: string;
}

const DEFAULT_PROD_CONFIG: AppConfig = {
  logLevel: 'error',
  environment: 'production',
  gitHash: environment.gitHash,
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

    return this.http.get<Partial<AppConfig>>('/assets/config.json').pipe(
      map(config => ({ ...DEFAULT_PROD_CONFIG, ...(config ?? {}) })),
      catchError(() => of(DEFAULT_PROD_CONFIG)),
      shareReplay(1)
    );
  }

  private getDevConfig(): AppConfig {
    return {
      logLevel: environment.logLevel || 'debug',
      environment: environment.environment || 'development',
      gitHash: environment.gitHash,
    };
  }

  getConfig(): Observable<AppConfig> {
    return this.config$;
  }
}
