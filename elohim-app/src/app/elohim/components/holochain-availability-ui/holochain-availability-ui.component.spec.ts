import { ComponentFixture, TestBed } from '@angular/core/testing';

import { HolochainAvailabilityUiComponent } from './holochain-availability-ui.component';
import { HolochainClientService } from '../../services/holochain-client.service';
import { vi } from 'vitest';

describe('HolochainAvailabilityUiComponent', () => {
  let component: HolochainAvailabilityUiComponent;
  let fixture: ComponentFixture<HolochainAvailabilityUiComponent>;
  let mockHolochainClient: any;

  beforeEach(async () => {
    // Create mock services with signal properties
    mockHolochainClient = {
      connect: vi.fn(),
      state: vi.fn().mockReturnValue('disconnected'),
      isConnected: vi.fn().mockReturnValue(false),
      error: vi.fn().mockReturnValue(null),
    };

    await TestBed.configureTestingModule({
      imports: [HolochainAvailabilityUiComponent],
      providers: [
        { provide: HolochainClientService, useValue: mockHolochainClient },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(HolochainAvailabilityUiComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
