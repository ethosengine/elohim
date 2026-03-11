import { ComponentFixture, TestBed } from '@angular/core/testing';
import { SophiaWrapperComponent } from './sophia-wrapper.component';

describe('SophiaWrapperComponent', () => {
  let component: SophiaWrapperComponent;
  let fixture: ComponentFixture<SophiaWrapperComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [SophiaWrapperComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(SophiaWrapperComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
