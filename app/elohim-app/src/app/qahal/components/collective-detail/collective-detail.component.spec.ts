import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';
import { CollectiveDetailComponent } from './collective-detail.component';

describe('CollectiveDetailComponent', () => {
  let component: CollectiveDetailComponent;
  let fixture: ComponentFixture<CollectiveDetailComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CollectiveDetailComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        {
          provide: ActivatedRoute,
          useValue: {
            paramMap: of({ get: (key: string) => key === 'id' ? 'household-dowell' : null }),
          },
        },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(CollectiveDetailComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should default to members tab', () => {
    expect(component.activeTab()).toBe('members');
  });

  it('should switch tabs', () => {
    component.setTab('proposals');
    expect(component.activeTab()).toBe('proposals');
  });
});
