import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';

import { ProtocolOmnibarComponent } from './protocol-omnibar.component';

describe('ProtocolOmnibarComponent', () => {
  let component: ProtocolOmnibarComponent;
  let fixture: ComponentFixture<ProtocolOmnibarComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ProtocolOmnibarComponent, RouterModule.forRoot([])],
    }).compileComponents();

    fixture = TestBed.createComponent(ProtocolOmnibarComponent);
    component = fixture.componentInstance;
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('starts in pill state', () => {
    fixture.detectChanges();
    expect(component.state).toBe('pill');
  });

  it('expands to details on pill click', () => {
    fixture.detectChanges();
    component.expand();
    expect(component.state).toBe('expanded');
  });

  it('collapses back to pill', () => {
    component.expand();
    component.collapse();
    expect(component.state).toBe('pill');
  });

  it('truncates long content addresses', () => {
    component.contentAddress = 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenora';
    expect(component.truncatedAddress).toBe('bafkrei...tenora');
  });

  it('shows reach icon for commons', () => {
    component.reach = 'commons';
    expect(component.reachIcon).toContain('\u{25CB}');
  });

  it('shows lock icon for private', () => {
    component.reach = 'private';
    expect(component.reachIcon).toBe('\u{1F512}');
  });

  it('provides resource route for drill-down', () => {
    component.contentId = 'manifesto';
    expect(component.inspectRoute).toEqual(['/resource', 'manifesto']);
  });

  it('toggles actions menu', () => {
    component.expand();
    expect(component.showActions).toBe(false);
    component.toggleActions();
    expect(component.showActions).toBe(true);
    component.toggleActions();
    expect(component.showActions).toBe(false);
  });

  it('displays steward names when expanded', () => {
    component.stewards = [
      { humanId: 'genesis', displayName: 'Genesis Collective', ratio: 0.8 },
    ];
    component.expand();
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const stewardEl = el.querySelector('[data-testid="omnibar-steward"]');
    expect(stewardEl?.textContent).toContain('Genesis Collective');
  });
});
