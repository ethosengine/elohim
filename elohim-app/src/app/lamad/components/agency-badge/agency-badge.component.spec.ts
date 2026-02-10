import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { AgencyBadgeComponent } from './agency-badge.component';
import { AgencyService } from '@app/imagodei/services/agency.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { signal, computed } from '@angular/core';

describe('AgencyBadgeComponent', () => {
  let component: AgencyBadgeComponent;
  let fixture: ComponentFixture<AgencyBadgeComponent>;
  let routerSpy: jasmine.SpyObj<Router>;
  let agencySpy: jasmine.SpyObj<AgencyService>;
  let holochainSpy: jasmine.SpyObj<HolochainClientService>;

  beforeEach(async () => {
    routerSpy = jasmine.createSpyObj('Router', ['navigate']);
    agencySpy = jasmine.createSpyObj(
      'AgencyService',
      ['getStageSummary'],
      {
        agencyState: signal({
          currentStage: 'citizen',
          networked: true,
          edgeNodeConnected: true,
        }),
        stageInfo: signal({
          stage: 'citizen',
          description: 'Test stage',
          capabilities: [],
        }),
        connectionStatus: signal({
          state: 'connected',
          message: 'Connected',
          isOnline: true,
        }),
        canUpgrade: signal(false),
      }
    );
    agencySpy.getStageSummary.and.returnValue({
      data: 'Test summary',
      progress: '0%',
    });

    holochainSpy = jasmine.createSpyObj(
      'HolochainClientService',
      ['getDisplayInfo', 'disconnect', 'connect'],
      {
        isConnected: signal(true),
      }
    );
    holochainSpy.getDisplayInfo.and.returnValue({
      state: 'connected',
      mode: 'doorway',
      adminUrl: 'ws://localhost:4444',
      appUrl: 'ws://localhost:4445',
      agentPubKey: 'test-agent-key',
      cellId: { dnaHash: 'test-dna-hash', agentPubKey: 'test-agent-key' },
      appId: 'elohim',
      dnaHash: 'test-dna-hash',
      connectedAt: new Date(),
      hasStoredCredentials: true,
      networkSeed: null,
      error: null,
    });

    await TestBed.configureTestingModule({
      imports: [AgencyBadgeComponent],
      providers: [
        { provide: Router, useValue: routerSpy },
        { provide: AgencyService, useValue: agencySpy },
        { provide: HolochainClientService, useValue: holochainSpy },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(AgencyBadgeComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  describe('Component Signals and State', () => {
    it('should have expanded signal', () => {
      expect(component.expanded).toBeDefined();
      expect(component.expanded()).toBe(false);
    });

    it('should have viewDetails output', () => {
      expect(component.viewDetails).toBeDefined();
      expect(component.viewDetails.emit).toBeDefined();
    });

    it('should have upgrade output', () => {
      expect(component.upgrade).toBeDefined();
      expect(component.upgrade.emit).toBeDefined();
    });

    it('should expose agency service signals', () => {
      expect(component.state).toBeDefined();
      expect(component.stageInfo).toBeDefined();
      expect(component.connectionStatus).toBeDefined();
      expect(component.canUpgrade).toBeDefined();
    });

    it('should expose holochain service info', () => {
      expect(component.edgeNodeInfo).toBeDefined();
    });
  });

  describe('getStageBadgeClass()', () => {
    it('should have getStageBadgeClass method', () => {
      expect(component.getStageBadgeClass).toBeDefined();
      expect(typeof component.getStageBadgeClass).toBe('function');
    });

    it('should return class string based on current stage', () => {
      const result = component.getStageBadgeClass();
      expect(result).toBeDefined();
      expect(typeof result).toBe('string');
      expect(result).toContain('stage-badge--');
    });
  });

  describe('getStatusDotClass()', () => {
    it('should have getStatusDotClass method', () => {
      expect(component.getStatusDotClass).toBeDefined();
      expect(typeof component.getStatusDotClass).toBe('function');
    });

    it('should return class string based on connection status', () => {
      const result = component.getStatusDotClass();
      expect(result).toBeDefined();
      expect(typeof result).toBe('string');
      expect(result).toContain('status-dot--');
    });
  });

  describe('toggleExpand()', () => {
    it('should have toggleExpand method', () => {
      expect(component.toggleExpand).toBeDefined();
      expect(typeof component.toggleExpand).toBe('function');
    });

    it('should toggle expanded signal', () => {
      const initialState = component.expanded();
      component.toggleExpand();
      expect(component.expanded()).toBe(!initialState);
    });

    it('should toggle expanded multiple times', () => {
      const initial = component.expanded();
      component.toggleExpand();
      expect(component.expanded()).toBe(!initial);
      component.toggleExpand();
      expect(component.expanded()).toBe(initial);
    });
  });

  describe('onViewDetails()', () => {
    it('should have onViewDetails method', () => {
      expect(component.onViewDetails).toBeDefined();
      expect(typeof component.onViewDetails).toBe('function');
    });

    it('should emit viewDetails output', (done) => {
      component.viewDetails.subscribe(() => {
        expect(true).toBe(true);
        done();
      });
      component.onViewDetails();
    });

    it('should navigate to profile network section', () => {
      component.onViewDetails();
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/lamad/human'], {
        fragment: 'network',
      });
    });
  });

  describe('onUpgrade()', () => {
    it('should have onUpgrade method', () => {
      expect(component.onUpgrade).toBeDefined();
      expect(typeof component.onUpgrade).toBe('function');
    });

    it('should emit upgrade output', (done) => {
      component.upgrade.subscribe(() => {
        expect(true).toBe(true);
        done();
      });
      component.onUpgrade();
    });

    it('should navigate to profile upgrade section', () => {
      component.onUpgrade();
      expect(routerSpy.navigate).toHaveBeenCalledWith(['/identity/profile'], {
        fragment: 'upgrade',
      });
    });
  });

  describe('onReconnect()', () => {
    it('should have onReconnect method', () => {
      expect(component.onReconnect).toBeDefined();
      expect(typeof component.onReconnect).toBe('function');
    });

    it('should disconnect and reconnect to holochain service', async () => {
      holochainSpy.disconnect.and.returnValue(Promise.resolve());
      holochainSpy.connect.and.returnValue(Promise.resolve());

      await component.onReconnect();

      expect(holochainSpy.disconnect).toHaveBeenCalled();
      expect(holochainSpy.connect).toHaveBeenCalled();
    });
  });

  describe('copyToClipboard()', () => {
    it('should have copyToClipboard method', () => {
      expect(component.copyToClipboard).toBeDefined();
      expect(typeof component.copyToClipboard).toBe('function');
    });

    it('should copy text to clipboard', async () => {
      const mockEvent = new MouseEvent('click');
      spyOn(mockEvent, 'stopPropagation');
      spyOn(navigator.clipboard, 'writeText').and.returnValue(
        Promise.resolve()
      );

      await component.copyToClipboard('test-value', mockEvent);

      expect(mockEvent.stopPropagation).toHaveBeenCalled();
      expect(navigator.clipboard.writeText).toHaveBeenCalledWith('test-value');
    });
  });

  describe('truncateHash()', () => {
    it('should have truncateHash method', () => {
      expect(component.truncateHash).toBeDefined();
      expect(typeof component.truncateHash).toBe('function');
    });

    it('should return empty string for null hash', () => {
      expect(component.truncateHash(null)).toBe('');
    });

    it('should return full hash if <= 16 characters', () => {
      const shortHash = '12345678';
      expect(component.truncateHash(shortHash)).toBe(shortHash);
    });

    it('should truncate long hash to first 8 and last 4 characters', () => {
      const longHash = '1234567890abcdefgh';
      const result = component.truncateHash(longHash);
      expect(result).toContain('12345678');
      expect(result).toContain('efgh');
      expect(result).toContain('...');
    });
  });
});
