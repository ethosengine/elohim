import { TestBed } from '@angular/core/testing';

import { HolochainContentService } from './holochain-content.service';
import { HolochainClientService } from './holochain-client.service';

describe('HolochainContentService', () => {
  let service: HolochainContentService;
  let clientMock: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    const clientSpy = jasmine.createSpyObj('HolochainClientService', ['callZome']);

    TestBed.configureTestingModule({
      providers: [
        HolochainContentService,
        { provide: HolochainClientService, useValue: clientSpy },
      ],
    });

    service = TestBed.inject(HolochainContentService);
    clientMock = TestBed.inject(HolochainClientService) as jasmine.SpyObj<HolochainClientService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have isAvailable method', () => {
    expect(service.isAvailable).toBeDefined();
    expect(typeof service.isAvailable).toBe('function');
  });

  it('should have clearCache method', () => {
    expect(service.clearCache).toBeDefined();
    expect(typeof service.clearCache).toBe('function');
  });
});
