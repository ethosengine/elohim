import { ComponentFixture, TestBed } from '@angular/core/testing';

import { CrisisComponent } from './crisis.component';

describe('CrisisComponent', () => {
  let component: CrisisComponent;
  let fixture: ComponentFixture<CrisisComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CrisisComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(CrisisComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
