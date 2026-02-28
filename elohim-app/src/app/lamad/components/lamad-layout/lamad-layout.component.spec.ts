import { ComponentFixture, TestBed } from '@angular/core/testing';
import { LamadLayoutComponent } from './lamad-layout.component';
import { provideRouter } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { ELOHIM_CLIENT } from '@app/elohim/providers/elohim-client.provider';
import { vi } from 'vitest';

describe('LamadLayoutComponent', () => {
  let component: LamadLayoutComponent;
  let fixture: ComponentFixture<LamadLayoutComponent>;

  const mockElohimClient = {
    get: vi.fn().mockReturnValue(Promise.resolve(null)),
    query: vi.fn().mockReturnValue(Promise.resolve([])),
    supportsOffline: vi.fn().mockReturnValue(false),
    backpressure: vi.fn().mockReturnValue(Promise.resolve(0)),
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
