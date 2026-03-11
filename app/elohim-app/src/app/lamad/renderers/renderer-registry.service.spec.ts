import { TestBed } from '@angular/core/testing';
import { RendererRegistryService } from './renderer-registry.service';

describe('RendererRegistryService', () => {
  let service: RendererRegistryService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [RendererRegistryService],
    });
    service = TestBed.inject(RendererRegistryService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
