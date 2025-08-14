import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ElohimHostComponent } from './elohim-host.component';

describe('ElohimHostComponent', () => {
  let component: ElohimHostComponent;
  let fixture: ComponentFixture<ElohimHostComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ElohimHostComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(ElohimHostComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
