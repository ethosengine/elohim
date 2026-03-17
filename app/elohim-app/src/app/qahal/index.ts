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
export { FileChallengeComponent } from './components/file-challenge/file-challenge.component';
export { ChallengeListComponent } from './components/challenge-list/challenge-list.component';
export { ChallengeDetailComponent } from './components/challenge-detail/challenge-detail.component';
export { ChallengeRouteComponent } from './components/challenge-route/challenge-route.component';
export { RespondToChallengeComponent } from './components/respond-to-challenge/respond-to-challenge.component';
export { FileAppealComponent } from './components/file-appeal/file-appeal.component';
export { FeedbackAggregateComponent } from './components/feedback-aggregate/feedback-aggregate.component';
export { ContributeStatementComponent } from './components/contribute-statement/contribute-statement.component';
export { SensemakingPageComponent } from './components/sensemaking-page/sensemaking-page.component';
export { GovernanceDispositionComponent } from './components/governance-disposition/governance-disposition.component';
export { ProxyVoteNotificationComponent } from './components/proxy-vote-notification/proxy-vote-notification.component';

// Services
export * from './services';
