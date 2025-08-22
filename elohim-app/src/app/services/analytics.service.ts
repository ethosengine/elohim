import { Injectable, inject } from '@angular/core';
import { DOCUMENT } from '@angular/common';
import { Observable, filter, switchMap, EMPTY, of, tap, catchError } from 'rxjs';
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

interface GtagFunction {
  (...args: any[]): void;
}

const GA_TRACKING_ID = 'G-NSL7PVP55B' as const;

@Injectable({
  providedIn: 'root'
})
export class AnalyticsService {
  private readonly configService = inject(ConfigService);
  private readonly document = inject(DOCUMENT) as Document;

  private getGtagStream(): Observable<GtagFunction> {
    return this.configService.getConfig().pipe(
      filter(config => config.environment === 'production'),
      switchMap(() => this.initializeGoogleAnalytics()),
      catchError(error => {
        console.error('Analytics initialization failed:', error);
        return EMPTY;
      })
    );
  }

  trackEvent(event: AnalyticsEvent): Observable<void> {
    return this.getGtagStream().pipe(
      tap(gtag => gtag('event', event.action, {
        event_category: event.category,
        event_label: event.label,
        value: event.value
      })),
      switchMap(() => of(void 0))
    );
  }

  trackPageView(pageView: PageView): Observable<void> {
    return this.getGtagStream().pipe(
      tap(gtag => gtag('config', GA_TRACKING_ID, {
        page_path: pageView.path,
        page_title: pageView.title
      })),
      switchMap(() => of(void 0))
    );
  }

  private initializeGoogleAnalytics(): Observable<GtagFunction> {
    return new Observable<GtagFunction>(subscriber => {
      try {
        const window = this.document.defaultView;
        if (!window) {
          subscriber.error(new Error('Window not available'));
          return;
        }

        // Initialize dataLayer and gtag
        (window as any).dataLayer = (window as any).dataLayer || [];
        const gtag: GtagFunction = (...args: any[]) => {
          (window as any).dataLayer.push(args);
        };
        (window as any).gtag = gtag;

        // Configure GA
        gtag('js', new Date());
        gtag('config', GA_TRACKING_ID);

        // Load script
        const script = this.document.createElement('script');
        script.async = true;
        script.src = `https://www.googletagmanager.com/gtag/js?id=${GA_TRACKING_ID}`;
        
        script.onload = () => {
          subscriber.next(gtag);
          subscriber.complete();
        };
        
        script.onerror = () => {
          subscriber.error(new Error('Failed to load Google Analytics'));
        };

        this.document.head.appendChild(script);
      } catch (error) {
        subscriber.error(error);
      }
    });
  }
}