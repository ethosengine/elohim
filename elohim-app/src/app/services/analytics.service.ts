import { Injectable, inject } from '@angular/core';
import { DOCUMENT } from '@angular/common';
import { ConfigService } from './config.service';

const GA_TRACKING_ID = 'G-NSL7PVP55B' as const;

@Injectable({
  providedIn: 'root'
})
export class AnalyticsService {
  private readonly configService = inject(ConfigService);
  private readonly document = inject(DOCUMENT);
  private initialized = false;

  constructor() {
    this.initializeIfProduction();
  }

  private initializeIfProduction(): void {
    this.configService.getConfig().subscribe(config => {
      if (config.environment === 'production' && !this.initialized) {
        this.initializeGoogleAnalytics();
        this.initialized = true;
      } else if (config.environment !== 'production') {
        this.addNoIndexingMeta();
      }
    });
  }

  private addNoIndexingMeta(): void {
    const robotsMeta = this.document.createElement('meta');
    robotsMeta.name = 'robots';
    robotsMeta.content = 'noindex, nofollow, noarchive, nosnippet';
    this.document.head.appendChild(robotsMeta);
  }

  private initializeGoogleAnalytics(): void {
    const window = this.document.defaultView;
    if (!window) return;

    // Initialize dataLayer and gtag
    (window as any).dataLayer = (window as any).dataLayer ?? [];
    (window as any).gtag = function() {
      (window as any).dataLayer.push(arguments);
    };


    // Load script
    const script = this.document.createElement('script');
    script.async = true;
    script.src = `https://www.googletagmanager.com/gtag/js?id=${GA_TRACKING_ID}`;
    
    // Configure GA
    script.onload = () => {
      (window as any).gtag('js', new Date());
      (window as any).gtag('config', GA_TRACKING_ID);
    };
    
    this.document.head.appendChild(script);


  }
}