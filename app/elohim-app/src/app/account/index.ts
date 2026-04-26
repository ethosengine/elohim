/**
 * Account Pillar — public API barrel.
 *
 * Composability boundary (per Task 16 ESLint config):
 *   account/ may import from: @app/imagodei, @app/elohim, @elohim/storage-client
 *   account/ must NOT import from: @app/lamad, @app/shefa, @app/qahal
 */

// Services
export { AccountService } from './services/account.service';
export { PortalHostService } from './services/portal-host.service';
export { PortalHostDiscoveryService } from './services/portal-host-discovery.service';
export { RevocationService } from './services/revocation.service';
export { HandoffService } from './services/handoff.service';

// Routes
export { ACCOUNT_ROUTES } from './account.routes';

// Models (re-exported for consumer convenience)
export type {
  AccountServiceState,
  AccountView,
  KeyRotationView,
  KeyRevocationView,
  RecoveryRequestView,
  HumanRelationshipView,
  HumanView,
  AgentPeerBindingView,
} from './models/account.model';

export type {
  PortalHostView,
  AddPortalHostInputView,
  PortalHostServiceState,
} from './models/portal-host.model';
