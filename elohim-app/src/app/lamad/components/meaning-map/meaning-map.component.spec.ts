import { ComponentFixture, TestBed } from '@angular/core/testing';
import { MeaningMapComponent } from './meaning-map.component';
import { provideHttpClient } from '@angular/common/http';

describe('MeaningMapComponent', () => {
  let component: MeaningMapComponent;
  let fixture: ComponentFixture<MeaningMapComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [MeaningMapComponent],
      providers: [
        provideHttpClient()
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(MeaningMapComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
