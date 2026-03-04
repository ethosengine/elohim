/**
 * Transaction Review Component Tests
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';

import { TransactionReviewComponent } from './transaction-review.component';
import { TransactionImportService } from '../../services/transaction-import.service';
import { AICategorizationService } from '../../services/ai-categorization.service';
import { BudgetReconciliationService } from '../../services/budget-reconciliation.service';
import { vi } from 'vitest';

describe('TransactionReviewComponent', () => {
  let component: TransactionReviewComponent;
  let fixture: ComponentFixture<TransactionReviewComponent>;
  let mockImportService: any;
  let mockAIService: any;
  let mockBudgetService: any;
  let mockActivatedRoute: any;

  beforeEach(async () => {
    mockImportService = {
      getBatch: vi.fn(),
      getStagedTransactionsForBatch: vi.fn(),
      approveTransaction: vi.fn(),
      rejectTransaction: vi.fn(),
      approveBatch: vi.fn(),
    };
    mockImportService.getBatch.mockReturnValue(undefined);
    mockImportService.getStagedTransactionsForBatch.mockReturnValue([]);

    mockAIService = {
      categorize: vi.fn(),
    };

    mockBudgetService = {
      reconcile: vi.fn(),
    };

    mockActivatedRoute = {
      snapshot: {
        paramMap: {
          get: vi.fn().mockReturnValue('test-batch-id'),
        },
      },
    };

    await TestBed.configureTestingModule({
      imports: [TransactionReviewComponent],
      providers: [
        { provide: TransactionImportService, useValue: mockImportService },
        { provide: AICategorizationService, useValue: mockAIService },
        { provide: BudgetReconciliationService, useValue: mockBudgetService },
        { provide: ActivatedRoute, useValue: mockActivatedRoute },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(TransactionReviewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
