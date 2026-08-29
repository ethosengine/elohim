import { ComponentFixture, TestBed } from '@angular/core/testing';

import { GovernanceApiService } from '@elohim/service';

import { FeedbackAggregateComponent } from './feedback-aggregate.component';
import { vi } from 'vitest';

describe('FeedbackAggregateComponent', () => {
  let component: FeedbackAggregateComponent;
  let fixture: ComponentFixture<FeedbackAggregateComponent>;
  let mockGovernanceApi: any;

  beforeEach(async () => {
    mockGovernanceApi = {
      getSignalAggregate: vi.fn().mockResolvedValue(null),
    };

    await TestBed.configureTestingModule({
      imports: [FeedbackAggregateComponent],
      providers: [{ provide: GovernanceApiService, useValue: mockGovernanceApi }],
    }).compileComponents();

    fixture = TestBed.createComponent(FeedbackAggregateComponent);
    component = fixture.componentInstance;
    fixture.componentRef.setInput('entityType', 'content');
    fixture.componentRef.setInput('entityId', 'content-1');
  });

  afterEach(() => {
    fixture.destroy();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  // The live service pipes catchError, so this guard is latent today — it exists so a
  // caller that does let the rejection through cannot become an unhandled rejection.
  it('should leave the aggregate null when the load rejects', async () => {
    mockGovernanceApi.getSignalAggregate.mockRejectedValue(new Error('aggregate unavailable'));

    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();

    expect(mockGovernanceApi.getSignalAggregate).toHaveBeenCalledWith('content', 'content-1');
    expect(component.aggregate()).toBeNull();
    expect(fixture.nativeElement.querySelector('.feedback-aggregate')).toBeNull();
  });
});
