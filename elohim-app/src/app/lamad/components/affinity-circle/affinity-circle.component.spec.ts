import { ComponentFixture, TestBed } from '@angular/core/testing';
import { AffinityCircleComponent } from './affinity-circle.component';

describe('AffinityCircleComponent', () => {
  let component: AffinityCircleComponent;
  let fixture: ComponentFixture<AffinityCircleComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [AffinityCircleComponent]
    }).compileComponents();

    fixture = TestBed.createComponent(AffinityCircleComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
