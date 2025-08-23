import { Injectable, inject } from '@angular/core';
import { DOCUMENT } from '@angular/common';
import { ConfigService } from './config.service';

export interface AnalyticsEvent {
  readonly action: string;
  readonly category: string;
  readonly label?: string;
  readonly value?: number;
}

export interface PageView {
  readonly path: string;
  readonly title?: string;
}

const GA_TRACKING_ID = 'G-NSL7PVP55B' as const;

@Injectable({
  providedIn: 'root'
})
export class AnalyticsService {
  private readonly configService = inject(ConfigService);
  private readonly document = inject(DOCUMENT) as Document;
  private initialized = false;

  constructor() {
    this.initializeIfProduction();
  }

  trackEvent(event: AnalyticsEvent): void {
    if (!this.isGoogleAnalyticsAvailable()) return;

    (window as any).gtag('event', event.action, {
      event_category: event.category,
      event_label: event.label,
      value: event.value
    });
  }

  trackPageView(pageView: PageView): void {
    if (!this.isGoogleAnalyticsAvailable()) return;

    (window as any).gtag('config', GA_TRACKING_ID, {
      page_path: pageView.path,
      page_title: pageView.title
    });
  }

  private initializeIfProduction(): void {
    this.configService.getConfig().subscribe(config => {
      if (config.environment === 'production' && !this.initialized) {
        this.initializeGoogleAnalytics();
        this.initialized = true;
      }
    });
  }

  private initializeGoogleAnalytics(): void {
    const window = this.document.defaultView;
    if (!window) return;

    // Initialize dataLayer and gtag
    (window as any).dataLayer = (window as any).dataLayer || [];
    (window as any).gtag = (...args: any[]) => {
      (window as any).dataLayer.push(args);
    };

    // Configure GA
    (window as any).gtag('js', new Date());
    (window as any).gtag('config', GA_TRACKING_ID);

    // Load script
    const script = this.document.createElement('script');
    script.async = true;
    script.src = `https://www.googletagmanager.com/gtag/js?id=${GA_TRACKING_ID}`;
    this.document.head.appendChild(script);
  }

  private isGoogleAnalyticsAvailable(): boolean {
    return typeof (window as any)?.gtag === 'function';
  }
}