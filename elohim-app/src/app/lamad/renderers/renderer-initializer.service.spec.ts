import { TestBed } from '@angular/core/testing';
import { RendererInitializerService } from './renderer-initializer.service';

describe('RendererInitializerService', () => {
  let service: RendererInitializerService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [RendererInitializerService],
    });
    service = TestBed.inject(RendererInitializerService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
