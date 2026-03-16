// event-stream.service.spec.ts
import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

import { EventStreamService } from './event-stream.service';

// Mock EventSource
class MockEventSource {
  static instances: MockEventSource[] = [];
  listeners: Record<string, ((event: MessageEvent) => void)[]> = {};
  url: string;
  readyState = 0;
  close = vi.fn();

  constructor(url: string) {
    this.url = url;
    this.readyState = 1; // OPEN
    MockEventSource.instances.push(this);
  }

  addEventListener(type: string, listener: (event: MessageEvent) => void): void {
    if (!this.listeners[type]) this.listeners[type] = [];
    this.listeners[type].push(listener);
  }

  removeEventListener(type: string, listener: (event: MessageEvent) => void): void {
    if (this.listeners[type]) {
      this.listeners[type] = this.listeners[type].filter((l) => l !== listener);
    }
  }

  // Test helper: simulate an event
  emit(type: string, data: string): void {
    const event = new MessageEvent(type, { data });
    this.listeners[type]?.forEach((l) => l(event));
  }
}

describe('EventStreamService', () => {
  let service: EventStreamService;
  let originalEventSource: typeof EventSource;

  beforeEach(() => {
    MockEventSource.instances = [];
    originalEventSource = globalThis.EventSource;
    (globalThis as unknown as Record<string, unknown>).EventSource = MockEventSource;

    TestBed.configureTestingModule({
      providers: [EventStreamService],
    });
    service = TestBed.inject(EventStreamService);
  });

  afterEach(() => {
    service.disconnect();
    (globalThis as unknown as Record<string, unknown>).EventSource = originalEventSource;
  });

  it('should create EventSource on connect', () => {
    service.connect('/api/v1/events');
    expect(MockEventSource.instances.length).toBe(1);
    expect(MockEventSource.instances[0].url).toBe('/api/v1/events');
  });

  it('should close EventSource on disconnect', () => {
    service.connect('/api/v1/events');
    const es = MockEventSource.instances[0];
    service.disconnect();
    expect(es.close).toHaveBeenCalled();
  });

  it('should emit matching events via on()', () => {
    service.connect('/api/v1/events');
    const values: unknown[] = [];
    service.on<{ id: string }>('content.created').subscribe((v) => values.push(v));

    MockEventSource.instances[0].emit('content.created', '{"id":"abc"}');

    expect(values.length).toBe(1);
    expect(values[0]).toEqual({ id: 'abc' });
  });

  it('should not emit events after disconnect', () => {
    service.connect('/api/v1/events');
    const values: unknown[] = [];
    service.on('content.created').subscribe((v) => values.push(v));
    service.disconnect();

    // The subscription should complete on disconnect
    expect(values.length).toBe(0);
  });

  it('should handle multiple event types independently', () => {
    service.connect('/api/v1/events');
    const created: unknown[] = [];
    const updated: unknown[] = [];

    service.on('content.created').subscribe((v) => created.push(v));
    service.on('content.updated').subscribe((v) => updated.push(v));

    MockEventSource.instances[0].emit('content.created', '{"id":"1"}');
    MockEventSource.instances[0].emit('content.updated', '{"id":"2"}');

    expect(created.length).toBe(1);
    expect(updated.length).toBe(1);
  });

  it('should not create duplicate connections', () => {
    service.connect('/api/v1/events');
    service.connect('/api/v1/events');
    expect(MockEventSource.instances.length).toBe(1);
  });
});
