/**
 * AI Categorization Service Tests
 */

import { TestBed } from '@angular/core/testing';

import { AICategorizationService } from './ai-categorization.service';

describe('AICategorizationService', () => {
  let service: AICategorizationService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [AICategorizationService],
    });
    service = TestBed.inject(AICategorizationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });
});
