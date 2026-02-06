import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LamadLayoutComponent } from './lamad-layout.component';
import { provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { ELOHIM_CLIENT } from '@app/elohim/providers/elohim-client.provider';

describe('LamadLayoutComponent', () => {
  let component: LamadLayoutComponent;
  let fixture: ComponentFixture<LamadLayoutComponent>;

  const mockElohimClient = {
    get: jasmine.createSpy('get').and.returnValue(Promise.resolve(null)),
    query: jasmine.createSpy('query').and.returnValue(Promise.resolve([])),
    supportsOffline: jasmine.createSpy('supportsOffline').and.returnValue(false),
    backpressure: jasmine.createSpy('backpressure').and.returnValue(Promise.resolve(0)),
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LamadLayoutComponent],
      providers: [
        provideRouter([]),
        provideHttpClient(),
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(LamadLayoutComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
