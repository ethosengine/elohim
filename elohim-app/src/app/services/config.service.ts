import { Injectable } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import { environment } from '../../environments/environment';

export interface AppConfig {
  logLevel: 'debug' | 'info' | 'error';
  environment: string;
}

@Injectable({
  providedIn: 'root'
})
export class ConfigService {
  private config: AppConfig | null = null;

  constructor(private http: HttpClient) {}

  loadConfig(): Promise<AppConfig> {
    // For local development, use environment file configuration
    if (!environment.production) {
      this.config = {
        logLevel: environment.logLevel,
        environment: environment.environment
      };
      return Promise.resolve(this.config);
    }
    
    // For production deployments, load from configmap-mounted config.json
    return this.http.get<AppConfig>('/assets/config.json').toPromise()
      .then(config => {
        this.config = config || { logLevel: 'error', environment: 'production' };
        return this.config!;
      });
  }

  getConfig(): AppConfig {
    if (!this.config) {
      throw new Error('Config not loaded. Call loadConfig() first.');
    }
    return this.config;
  }
}