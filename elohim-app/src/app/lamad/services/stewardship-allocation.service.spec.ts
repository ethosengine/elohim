import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule } from '@angular/common/http/testing';

import { StewardshipAllocationService } from './stewardship-allocation.service';

describe('StewardshipAllocationService', () => {
  let service: StewardshipAllocationService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [StewardshipAllocationService],
    });
    service = TestBed.inject(StewardshipAllocationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
