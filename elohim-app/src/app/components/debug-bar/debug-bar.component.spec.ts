import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { of } from 'rxjs';

import { DebugBarComponent } from './debug-bar.component';
import { ConfigService } from '../../services/config.service';

describe('DebugBarComponent', () => {
  let component: DebugBarComponent;
  let fixture: ComponentFixture<DebugBarComponent>;
  let configService: ConfigService;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DebugBarComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();

    fixture = TestBed.createComponent(DebugBarComponent);
    component = fixture.componentInstance;
    configService = TestBed.inject(ConfigService);
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should show debug bar for staging environment', done => {
    spyOn(configService, 'getConfig').and.returnValue(
      of({
        logLevel: 'debug',
        environment: 'staging',
      })
    );

    component.ngOnInit();

    configService.getConfig().subscribe(() => {
      expect(component.showDebugBar).toBe(true);
      expect(component.environmentLabel).toBe('STAGING');
      done();
    });
  });

  it('should show debug bar for alpha environment', done => {
    spyOn(configService, 'getConfig').and.returnValue(
      of({
        logLevel: 'debug',
        environment: 'alpha',
      })
    );

    component.ngOnInit();

    configService.getConfig().subscribe(() => {
      expect(component.showDebugBar).toBe(true);
      expect(component.environmentLabel).toBe('ALPHA');
      done();
    });
  });

  it('should not show debug bar for production environment', done => {
    spyOn(configService, 'getConfig').and.returnValue(
      of({
        logLevel: 'error',
        environment: 'production',
      })
    );

    component.ngOnInit();

    configService.getConfig().subscribe(() => {
      expect(component.showDebugBar).toBe(false);
      done();
    });
  });
});
