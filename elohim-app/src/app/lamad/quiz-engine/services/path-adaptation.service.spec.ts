import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { PathAdaptationService } from './path-adaptation.service';
import { ELOHIM_CLIENT } from '@app/elohim/providers/elohim-client.provider';

describe('PathAdaptationService', () => {
  let service: PathAdaptationService;

  beforeEach(() => {
    const mockElohimClient = jasmine.createSpyObj('ElohimClient', ['getContent', 'listContent']);
    mockElohimClient.getContent.and.returnValue(Promise.resolve(null));
    mockElohimClient.listContent.and.returnValue(Promise.resolve([]));

    TestBed.configureTestingModule({
      providers: [
        PathAdaptationService,
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient },
      ],
    });
    service = TestBed.inject(PathAdaptationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
