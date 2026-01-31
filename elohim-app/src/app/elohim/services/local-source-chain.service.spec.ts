import { TestBed } from '@angular/core/testing';

import { LocalSourceChainService } from './local-source-chain.service';

describe('LocalSourceChainService', () => {
  let service: LocalSourceChainService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [LocalSourceChainService],
    });

    service = TestBed.inject(LocalSourceChainService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
