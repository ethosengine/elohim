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

describe('TransactionReviewComponent', () => {
  let component: TransactionReviewComponent;
  let fixture: ComponentFixture<TransactionReviewComponent>;
  let mockImportService: jasmine.SpyObj<TransactionImportService>;
  let mockAIService: jasmine.SpyObj<AICategorizationService>;
  let mockBudgetService: jasmine.SpyObj<BudgetReconciliationService>;
  let mockActivatedRoute: any;

  beforeEach(async () => {
    mockImportService = jasmine.createSpyObj('TransactionImportService', [
      'getBatch',
      'getStagedTransactionsForBatch',
      'approveTransaction',
      'rejectTransaction',
      'approveBatch',
    ]);
    mockImportService.getBatch.and.returnValue(undefined);
    mockImportService.getStagedTransactionsForBatch.and.returnValue([]);

    mockAIService = jasmine.createSpyObj('AICategorizationService', ['categorize']);

    mockBudgetService = jasmine.createSpyObj('BudgetReconciliationService', ['reconcile']);

    mockActivatedRoute = {
      snapshot: {
        paramMap: {
          get: jasmine.createSpy('get').and.returnValue('test-batch-id'),
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
