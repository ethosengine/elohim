import { Injectable } from '@angular/core';
import { ConfigService } from './config.service';

declare global {
  interface Window {
    dataLayer: any[];
    gtag: (...args: any[]) => void;
  }
}

@Injectable({
  providedIn: 'root'
})
export class AnalyticsService {
  private readonly GA_TRACKING_ID = 'G-NSL7PVP55B';
  private isInitialized = false;

  constructor(private readonly configService: ConfigService) {}

  async initialize(): Promise<void> {
    try {
      const config = this.configService.getConfig();
      
      // Only load Google Analytics in production environment
      if (config.environment === 'production' && !this.isInitialized) {
        await this.loadGoogleAnalytics();
        this.isInitialized = true;
      }
    } catch (error) {
      console.error('Failed to initialize analytics:', error);
    }
  }

  private loadGoogleAnalytics(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        // Initialize dataLayer
        window.dataLayer = window.dataLayer || [];
        
        // Define gtag function
        window.gtag = function() {
          window.dataLayer.push(arguments);
        };
        
        // Set initial values
        window.gtag('js', new Date());
        window.gtag('config', this.GA_TRACKING_ID);
        
        // Create and append script tag
        const script = document.createElement('script');
        script.async = true;
        script.src = `https://www.googletagmanager.com/gtag/js?id=${this.GA_TRACKING_ID}`;
        script.onload = () => resolve();
        script.onerror = () => reject(new Error('Failed to load Google Analytics script'));
        
        document.head.appendChild(script);
      } catch (error) {
        reject(error instanceof Error ? error : new Error(String(error)));
      }
    });
  }

  trackEvent(action: string, category: string, label?: string, value?: number): void {
    if (this.isInitialized && window.gtag) {
      window.gtag('event', action, {
        event_category: category,
        event_label: label,
        value: value
      });
    }
  }

  trackPageView(pagePath: string, pageTitle?: string): void {
    if (this.isInitialized && window.gtag) {
      window.gtag('config', this.GA_TRACKING_ID, {
        page_path: pagePath,
        page_title: pageTitle
      });
    }
  }
}