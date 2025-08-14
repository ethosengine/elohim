import { ComponentFixture, TestBed } from '@angular/core/testing';

import { PathForwardComponent } from './path-forward.component';

describe('PathForwardComponent', () => {
  let component: PathForwardComponent;
  let fixture: ComponentFixture<PathForwardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PathForwardComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(PathForwardComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
