import { ComponentFixture, TestBed } from '@angular/core/testing';
import { PerseusWrapperComponent } from './perseus-wrapper.component';

describe('PerseusWrapperComponent', () => {
  let component: PerseusWrapperComponent;
  let fixture: ComponentFixture<PerseusWrapperComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PerseusWrapperComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(PerseusWrapperComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
