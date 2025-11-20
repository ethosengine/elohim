import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LamadLayoutComponent } from './lamad-layout.component';
import { provideRouter } from '@angular/router';

describe('LamadLayoutComponent', () => {
  let component: LamadLayoutComponent;
  let fixture: ComponentFixture<LamadLayoutComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LamadLayoutComponent],
      providers: [
        provideRouter([])
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(LamadLayoutComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
