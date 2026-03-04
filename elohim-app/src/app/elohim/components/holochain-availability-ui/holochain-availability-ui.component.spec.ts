import { ComponentFixture, TestBed } from '@angular/core/testing';

import { HolochainAvailabilityUiComponent } from './holochain-availability-ui.component';
import { HolochainClientService } from '../../services/holochain-client.service';
import { HolochainContentService } from '../../services/holochain-content.service';
import { OfflineOperationQueueService } from '../../services/offline-operation-queue.service';
import { vi } from 'vitest';

describe('HolochainAvailabilityUiComponent', () => {
  let component: HolochainAvailabilityUiComponent;
  let fixture: ComponentFixture<HolochainAvailabilityUiComponent>;
  let mockHolochainClient: any;
  let mockHolochainContent: any;
  let mockOperationQueue: any;

  beforeEach(async () => {
    // Create mock services with signal properties
    mockHolochainClient = {
      connect: vi.fn(),
      state: vi.fn().mockReturnValue('disconnected'),
      isConnected: vi.fn().mockReturnValue(false),
      error: vi.fn().mockReturnValue(null),
    };

    mockHolochainContent = { available: vi.fn().mockReturnValue(false) };

    mockOperationQueue = { getQueueSize: vi.fn(), syncAll: vi.fn() };
    mockOperationQueue.getQueueSize.mockReturnValue(0);

    await TestBed.configureTestingModule({
      imports: [HolochainAvailabilityUiComponent],
      providers: [
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: HolochainContentService, useValue: mockHolochainContent },
        { provide: OfflineOperationQueueService, useValue: mockOperationQueue },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(HolochainAvailabilityUiComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
