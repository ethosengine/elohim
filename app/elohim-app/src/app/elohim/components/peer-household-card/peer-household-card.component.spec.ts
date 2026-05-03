import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import type { PeerHouseholdEdge } from '@app/generated/peer-household-edge';

import { PeerHouseholdCardComponent } from './peer-household-card.component';

function mk(p: Partial<PeerHouseholdEdge> = {}): PeerHouseholdEdge {
  return {
    householdId: 'adam',
    online: true,
    myCidsHostedByThem: 0,
    theirCidsHostedByMe: 0,
    netDiff: 0,
    ...p,
  };
}

describe('PeerHouseholdCardComponent', () => {
  let fixture: ComponentFixture<PeerHouseholdCardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PeerHouseholdCardComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(PeerHouseholdCardComponent);
  });

  it('shows the reciprocity diff with sign', () => {
    fixture.componentInstance.edge = mk({
      myCidsHostedByThem: 7,
      theirCidsHostedByMe: 12,
      netDiff: 5,
    });
    fixture.detectChanges();
    const diff = fixture.nativeElement.querySelector('[data-testid="peer-card-net-diff"]');
    expect(diff?.textContent).toContain('+5');
  });

  it('flags critical-for-me when set', () => {
    fixture.componentInstance.edge = mk({ isCriticalForMe: true });
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="peer-card-critical-for-me"]')
    ).toBeTruthy();
  });

  it('marks card dark when peer is offline', () => {
    fixture.componentInstance.edge = mk({ online: false });
    fixture.detectChanges();
    const card = fixture.nativeElement.querySelector('[data-testid="peer-household-card"]');
    expect(card?.classList.contains('dark')).toBe(true);
  });
});
