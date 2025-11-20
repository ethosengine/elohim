import { ComponentFixture, TestBed } from '@angular/core/testing';
import { EpicViewerComponent } from './epic-viewer.component';
import { ActivatedRoute } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { of } from 'rxjs';

describe('EpicViewerComponent', () => {
  let component: EpicViewerComponent;
  let fixture: ComponentFixture<EpicViewerComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EpicViewerComponent],
      providers: [
        provideHttpClient(),
        {
          provide: ActivatedRoute,
          useValue: {
            params: of({ id: 'test-epic-1' })
          }
        }
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(EpicViewerComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
