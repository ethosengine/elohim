import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ScenarioDetailComponent } from './scenario-detail.component';

describe('ScenarioDetailComponent', () => {
  let component: ScenarioDetailComponent;
  let fixture: ComponentFixture<ScenarioDetailComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ScenarioDetailComponent]
    }).compileComponents();

    fixture = TestBed.createComponent(ScenarioDetailComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
