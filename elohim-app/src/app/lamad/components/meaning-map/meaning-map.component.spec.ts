import { ComponentFixture, TestBed } from '@angular/core/testing';
import { MeaningMapComponent } from './meaning-map.component';

describe('MeaningMapComponent', () => {
  let component: MeaningMapComponent;
  let fixture: ComponentFixture<MeaningMapComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [MeaningMapComponent]
    }).compileComponents();

    fixture = TestBed.createComponent(MeaningMapComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
