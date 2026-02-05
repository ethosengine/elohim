import { ComponentFixture, TestBed } from '@angular/core/testing';

import { HolochainAvailabilityUiComponent } from './holochain-availability-ui.component';
import { HolochainClientService } from '../../services/holochain-client.service';
import { HolochainContentService } from '../../services/holochain-content.service';
import { OfflineOperationQueueService } from '../../services/offline-operation-queue.service';

describe('HolochainAvailabilityUiComponent', () => {
  let component: HolochainAvailabilityUiComponent;
  let fixture: ComponentFixture<HolochainAvailabilityUiComponent>;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockHolochainContent: jasmine.SpyObj<HolochainContentService>;
  let mockOperationQueue: jasmine.SpyObj<OfflineOperationQueueService>;

  beforeEach(async () => {
    // Create mock services with signal properties
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', ['connect'], {
      state: jasmine.createSpy('state').and.returnValue('disconnected'),
      isConnected: jasmine.createSpy('isConnected').and.returnValue(false),
      error: jasmine.createSpy('error').and.returnValue(null),
    });

    mockHolochainContent = jasmine.createSpyObj('HolochainContentService', [], {
      available: jasmine.createSpy('available').and.returnValue(false),
    });

    mockOperationQueue = jasmine.createSpyObj(
      'OfflineOperationQueueService',
      ['getQueueSize', 'syncAll'],
      {}
    );
    mockOperationQueue.getQueueSize.and.returnValue(0);

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
