import { TestBed } from '@angular/core/testing';
import { DiscoveryAttestationService } from './discovery-attestation.service';

describe('DiscoveryAttestationService', () => {
  let service: DiscoveryAttestationService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [DiscoveryAttestationService],
    });
    service = TestBed.inject(DiscoveryAttestationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
