import { Injectable, OnDestroy } from '@angular/core';

import { filter, map, takeUntil } from 'rxjs/operators';

import { Observable, Subject } from 'rxjs';

interface SseEvent {
  type: string;
  data: unknown;
}

@Injectable({ providedIn: 'root' })
export class EventStreamService implements OnDestroy {
  private eventSource: EventSource | null = null;
  private readonly events$ = new Subject<SseEvent>();
  private readonly destroy$ = new Subject<void>();
  private activeListeners: { type: string; listener: (e: MessageEvent) => void }[] = [];

  connect(url: string): void {
    if (this.eventSource) return;
    this.eventSource = new EventSource(url);
  }

  disconnect(): void {
    if (!this.eventSource) return;

    // Remove all registered listeners
    for (const { type, listener } of this.activeListeners) {
      this.eventSource.removeEventListener(type, listener);
    }
    this.activeListeners = [];

    this.eventSource.close();
    this.eventSource = null;
    this.destroy$.next();
  }

  on<T = unknown>(eventType: string): Observable<T> {
    if (!this.eventSource) {
      return new Observable<T>(subscriber => subscriber.complete());
    }

    const listener = (event: MessageEvent) => {
      try {
        const data = JSON.parse(event.data);
        this.events$.next({ type: eventType, data });
      } catch {
        // Ignore malformed JSON
      }
    };

    this.eventSource.addEventListener(eventType, listener);
    this.activeListeners.push({ type: eventType, listener });

    return this.events$.pipe(
      takeUntil(this.destroy$),
      filter(e => e.type === eventType),
      map(e => e.data as T)
    );
  }

  ngOnDestroy(): void {
    this.disconnect();
    this.events$.complete();
    this.destroy$.complete();
  }
}
