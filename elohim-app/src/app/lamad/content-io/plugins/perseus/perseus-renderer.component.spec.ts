import { ComponentFixture, TestBed } from '@angular/core/testing';
import { PerseusRendererComponent } from './perseus-renderer.component';

describe('PerseusRendererComponent', () => {
  let component: PerseusRendererComponent;
  let fixture: ComponentFixture<PerseusRendererComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PerseusRendererComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(PerseusRendererComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
