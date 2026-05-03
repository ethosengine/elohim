import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import { DiversityHintComponent } from './diversity-hint.component';

describe('DiversityHintComponent', () => {
  let fixture: ComponentFixture<DiversityHintComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DiversityHintComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(DiversityHintComponent);
  });

  it('renders region metros for region_metro kind', () => {
    fixture.componentInstance.hint = {
      kind: 'region_metro',
      value: ['us-central', 'eu-west'],
    };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toMatch(/us-central/);
    expect(fixture.nativeElement.textContent).toMatch(/eu-west/);
  });

  it('renders household archetypes with human labels', () => {
    fixture.componentInstance.hint = {
      kind: 'household_archetypes',
      value: ['desktop', 'node'],
    };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toMatch(/Laptop/);
    expect(fixture.nativeElement.textContent).toMatch(/Home server/);
  });

  it('renders member count for collective', () => {
    fixture.componentInstance.hint = { kind: 'collective_member_count', value: 8 };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent).toMatch(/8/);
  });

  it('renders nothing for none', () => {
    fixture.componentInstance.hint = { kind: 'none', value: null };
    fixture.detectChanges();
    expect(fixture.nativeElement.textContent.trim()).toBe('');
  });
});
