import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LamadLayoutComponent } from './lamad-layout.component';

describe('LamadLayoutComponent', () => {
  let component: LamadLayoutComponent;
  let fixture: ComponentFixture<LamadLayoutComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LamadLayoutComponent]
    }).compileComponents();

    fixture = TestBed.createComponent(LamadLayoutComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
