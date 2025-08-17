import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';

import { DebugBarComponent } from './debug-bar.component';

describe('DebugBarComponent', () => {
  let component: DebugBarComponent;
  let fixture: ComponentFixture<DebugBarComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DebugBarComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting()
      ]
    })
    .compileComponents();

    fixture = TestBed.createComponent(DebugBarComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
