/**
 * Community Module - Community Context App
 *
 * Human relationships, consent, governance, and community features.
 *
 * Routes:
 * - /community - Community home (future)
 * - /community/human - Community-specific profile (future)
 */

// Interfaces (abstract contracts for IoC)
export * from './interfaces';

// Models
export * from './models';

// Components
export { FaceCardComponent } from './components/face-card/face-card.component';
export { CommunityDirectoryComponent } from './components/community-directory/community-directory.component';
export { CollectiveDetailComponent } from './components/collective-detail/collective-detail.component';
export {
  ContextMenuOnlyComponent,
  type ContextMenuAction,
} from './components/context-menu-only/context-menu-only.component';
export { FeedbackMechanismGatewayComponent } from './components/feedback-mechanism-gateway/feedback-mechanism-gateway.component';
export { PsephosBallotWrapperComponent } from './components/psephos-ballot-wrapper/psephos-ballot-wrapper.component';

// Services
export * from './services';
