import { TestBed } from '@angular/core/testing';
import { provideHttpClientTesting } from '@angular/common/http/testing';

import { StewardshipAllocationService } from './stewardship-allocation.service';
import { provideHttpClient } from '@angular/common/http';

describe('StewardshipAllocationService', () => {
  let service: StewardshipAllocationService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), StewardshipAllocationService],
    });
    service = TestBed.inject(StewardshipAllocationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
