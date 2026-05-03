import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import { CommitmentBarComponent } from './commitment-bar.component';

describe('CommitmentBarComponent', () => {
  let fixture: ComponentFixture<CommitmentBarComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CommitmentBarComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(CommitmentBarComponent);
  });

  it('renders the honored percent', () => {
    fixture.componentInstance.committedBytes = 14_000_000_000;
    fixture.componentInstance.deliveredBytes = 11_200_000_000;
    fixture.detectChanges();
    const percentEl = fixture.nativeElement.querySelector('[data-testid="commitment-bar-percent"]');
    expect(percentEl?.textContent).toMatch(/80/);
  });

  it('flags over-delivered when delivered exceeds committed', () => {
    fixture.componentInstance.committedBytes = 3_000_000_000;
    fixture.componentInstance.deliveredBytes = 3_100_000_000;
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="commitment-bar-over-delivered"]'),
    ).toBeTruthy();
  });

  it('renders 0% when no commitment', () => {
    fixture.componentInstance.committedBytes = 0;
    fixture.componentInstance.deliveredBytes = 0;
    fixture.detectChanges();
    const percentEl = fixture.nativeElement.querySelector('[data-testid="commitment-bar-percent"]');
    expect(percentEl?.textContent).toMatch(/^0%/);
  });
});
