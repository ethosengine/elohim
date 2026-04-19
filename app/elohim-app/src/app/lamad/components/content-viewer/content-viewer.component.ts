import { CommonModule, DOCUMENT } from '@angular/common';
import {
  AfterViewChecked,
  Component,
  ComponentRef,
  HostListener,
  OnDestroy,
  OnInit,
  ViewChild,
  ViewContainerRef,
  inject,
} from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

// @coverage: 90.5% (2026-03-03)

import { catchError, takeUntil } from 'rxjs/operators';

import { Subject, Subscription, forkJoin, of } from 'rxjs';

import { ContentAnalyticsComponent } from '@app/elohim/components/content-analytics/content-analytics.component';
import { EprRelationshipsPanelComponent } from '@app/elohim/components/epr-relationships-panel/epr-relationships-panel.component';
import {
  ProtocolOmnibarComponent,
  OmnibarSteward,
} from '@app/elohim/components/protocol-omnibar/protocol-omnibar.component';
import { TrustBadge } from '@app/elohim/models/trust-badge.model';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { AgentService } from '@app/elohim/services/agent.service';
import {
  ChallengeRecord,
  DataLoaderService,
  DiscussionRecord,
  GovernanceStateRecord,
} from '@app/elohim/services/data-loader.service';
import { EprResolverService } from '@app/elohim/services/epr-resolver.service';
import {
  AggregatedSignals,
  GovernanceSignalService,
} from '@app/elohim/services/governance-signal.service';
import { GovernanceService } from '@app/elohim/services/governance.service';
import { TrustBadgeService } from '@app/elohim/services/trust-badge.service';
import {
  DEFAULT_FEEDBACK_PROFILES,
  EmotionalReactionType,
  FeedbackProfile,
  createProfileFromTemplate,
} from '@app/lamad/models/feedback-profile.model';
import { FeedbackMechanismGatewayComponent } from '@app/qahal';
import {
  FeedbackContext,
  GraduatedFeedbackComponent,
} from '@app/qahal/components/graduated-feedback/graduated-feedback.component';
import { ReactionBarComponent } from '@app/qahal/components/reaction-bar/reaction-bar.component';
import { AttentionTrackerService } from '@app/shefa/services/attention-tracker.service';

import type { ContentStewardshipView } from '@elohim/storage-client/generated';

import { SeoService } from '../../../services/seo.service';
import { ContentDownloadComponent } from '../../content-io/components/content-download/content-download.component';
import { ContentEditorService } from '../../content-io/services/content-editor.service';
import { ContentNode, ContentFlag } from '../../models/content-node.model';
import { PathContext } from '../../models/exploration-context.model';
import {
  ContentRenderer,
  RendererCompletionEvent,
  RendererRegistryService,
} from '../../renderers/renderer-registry.service';
import { ContentService } from '../../services/content.service';
import { HouseholdResilienceService } from '../../services/household-resilience.service';
import { PathContextService } from '../../services/path-context.service';
import {
  ResilienceService,
  type ResilienceView,
  type VerificationResultView,
} from '../../services/resilience.service';
import { SignalHarnessService } from '../../services/signal-harness.service';
import { StewardshipAllocationService } from '../../services/stewardship-allocation.service';
import { FocusedViewToggleComponent } from '../focused-view-toggle/focused-view-toggle.component';
import { MiniGraphComponent } from '../mini-graph/mini-graph.component';

import type { HouseholdResilienceView } from '../../../generated/household-resilience-view';
import type { EprRelationship } from '@app/elohim/models/epr-head.model';

@Component({
  selector: 'app-content-viewer',
  standalone: true,
  imports: [
    CommonModule,
    RouterModule,
    ContentDownloadComponent,
    MiniGraphComponent,
    ReactionBarComponent,
    GraduatedFeedbackComponent,
    FeedbackMechanismGatewayComponent,
    FocusedViewToggleComponent,
    ContentAnalyticsComponent,
    ProtocolOmnibarComponent,
    EprRelationshipsPanelComponent,
  ],
  templateUrl: './content-viewer.component.html',
  styleUrls: ['./content-viewer.component.css'],
})
export class ContentViewerComponent implements OnInit, OnDestroy, AfterViewChecked {
  node: ContentNode | null = null;
  affinity = 0;
  relatedNodes: ContentNode[] = [];
  isLoading = true;
  error: string | null = null;

  // Tab state
  activeTab: 'content' | 'trust' | 'governance' | 'network' = 'content';

  // Trust data
  trustBadge: TrustBadge | null = null;
  isLoadingTrust = false;

  // Stewardship data
  stewardship: ContentStewardshipView | null = null;

  // Resilience data
  resilience: ResilienceView | null = null;
  householdResilience: HouseholdResilienceView | null = null;
  verificationResult: VerificationResultView | null = null;
  isVerifying = false;

  // Governance data
  governanceState: GovernanceStateRecord | null = null;
  challenges: ChallengeRecord[] = [];
  discussions: DiscussionRecord[] = [];
  isLoadingGovernance = false;

  // Governance feedback (signals)
  feedbackProfile: FeedbackProfile | null = null;
  aggregatedSignals: AggregatedSignals | null = null;
  allowedReactions: EmotionalReactionType[] = [];
  feedbackContext: FeedbackContext = 'usefulness';
  showFeedbackSection = true;

  // "Appears in paths" back-links (Wikipedia-style)
  containingPaths: { pathId: string; pathTitle: string; stepIndex: number }[] = [];
  loadingPaths = false;

  // Edit capability
  canEditContent = false;

  // Path context for return navigation (when viewing from a detour)
  pathContext: PathContext | null = null;
  hasReturnPath = false;

  // Focused view (immersive mode) state
  isFocusedView = false;
  private readonly TRANSITION_DURATION = 300; // Match CSS transition duration

  // Protocol omnibar data (shown in focused view — like browser padlock)
  omnibarStewards: OmnibarSteward[] = [];
  omnibarContentAddress = '';
  omnibarReach = '';
  omnibarDeliverySource = '';

  // Dynamic renderer hosting
  @ViewChild('rendererHost', { read: ViewContainerRef, static: false })
  rendererHost!: ViewContainerRef;
  private rendererRef: ComponentRef<ContentRenderer> | null = null;
  private rendererSubscription: Subscription | null = null;

  /** Whether we have a registered renderer for the current content format */
  hasRegisteredRenderer = false;

  /** Flag to trigger renderer loading in AfterViewChecked */
  private pendingRendererLoad = false;

  private readonly destroy$ = new Subject<void>();
  private nodeId: string | null = null;
  private readonly seoService = inject(SeoService);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly affinityService = inject(AffinityTrackingService);
  private readonly agentService = inject(AgentService);
  private readonly rendererRegistry = inject(RendererRegistryService);
  private readonly contentService = inject(ContentService);
  private readonly dataLoader = inject(DataLoaderService);
  private readonly trustBadgeService = inject(TrustBadgeService);
  private readonly editorService = inject(ContentEditorService);
  private readonly pathContextService = inject(PathContextService);
  private readonly governanceService = inject(GovernanceService);
  private readonly signalService = inject(GovernanceSignalService);
  private readonly document = inject(DOCUMENT);
  private readonly signalHarness = inject(SignalHarnessService);
  private readonly stewardshipService = inject(StewardshipAllocationService);
  private readonly resilienceService = inject(ResilienceService);
  private readonly householdResilienceService = inject(HouseholdResilienceService);
  private readonly attentionTracker = inject(AttentionTrackerService);
  private readonly eprResolver = inject(EprResolverService);

  eprRelationships: EprRelationship[] = [];

  /** Default feedback profile type for learning content */
  private readonly LEARNING_CONTENT_PROFILE = 'learning-content';

  /** CSS class for focused view mode */
  private readonly FOCUSED_VIEW_MODE_CLASS = 'focused-view-mode';

  ngOnInit(): void {
    // Handle direct content access: /lamad/resource/:resourceId
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const resourceId = params['resourceId'] as string;
      if (resourceId) {
        // Leave previous content if navigating within the viewer
        if (this.nodeId && this.nodeId !== resourceId) {
          this.attentionTracker.trackContentLeave(this.nodeId);
        }
        this.nodeId = resourceId;
        this.loadContent(resourceId);
      }
    });

    // Listen for affinity changes
    this.affinityService.changes$.pipe(takeUntil(this.destroy$)).subscribe(change => {
      if (change?.nodeId === this.nodeId) {
        this.affinity = change.newValue;
      }
    });

    // Subscribe to path context for return navigation
    this.pathContextService.context$.pipe(takeUntil(this.destroy$)).subscribe(context => {
      this.pathContext = context;
      this.hasReturnPath = context?.detourStack !== undefined && context.detourStack.length > 0;
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
    // Stop attention tracking for current content
    if (this.nodeId) {
      this.attentionTracker.trackContentLeave(this.nodeId);
    }
    // Clean up focused view mode if active
    this.document.body.classList.remove(this.FOCUSED_VIEW_MODE_CLASS);
  }

  ngAfterViewChecked(): void {
    // Load renderer when view is ready and we have a pending load request
    if (this.pendingRendererLoad && this.node && this.rendererHost) {
      this.pendingRendererLoad = false;
      this.loadRenderer();
    }
  }

  /**
   * Clean up the current renderer instance
   */
  private destroyRenderer(): void {
    if (this.rendererSubscription) {
      this.rendererSubscription.unsubscribe();
      this.rendererSubscription = null;
    }
    if (this.rendererRef) {
      this.rendererRef.destroy();
      this.rendererRef = null;
    }
  }

  /**
   * Dynamically instantiate the appropriate renderer for the current node.
   * Called after the node is loaded and the view is ready.
   */
  private loadRenderer(): void {
    if (!this.node || !this.rendererHost) {
      return;
    }

    // Clean up previous renderer
    this.destroyRenderer();
    this.rendererHost.clear();

    // Get the renderer component for this content format
    const rendererComponent = this.rendererRegistry.getRenderer(this.node);

    if (!rendererComponent) {
      this.hasRegisteredRenderer = false;
      return;
    }

    this.hasRegisteredRenderer = true;

    // Create the renderer component
    this.rendererRef = this.rendererHost.createComponent(rendererComponent);

    // Set the node input using setInput to trigger ngOnChanges
    this.rendererRef.setInput('node', this.node);

    // Subscribe to completion events if the renderer supports them
    const instance = this.rendererRef.instance as unknown as Record<string, unknown>;
    if (
      instance['complete'] &&
      instance['complete'] instanceof Object &&
      'subscribe' in (instance['complete'] as object)
    ) {
      this.rendererSubscription = (
        instance['complete'] as {
          subscribe: (fn: (event: RendererCompletionEvent) => void) => Subscription;
        }
      ).subscribe((event: RendererCompletionEvent) => {
        // Manifest-driven: translate to REA economic event
        if (this.node) {
          void this.signalHarness.onRendererComplete(this.node, event);
        }
        // Existing UI handling (affinity, governance signals)
        this.onRendererComplete(event);
      });
    }
  }

  /**
   * Handle completion events from interactive renderers (quiz, simulation, etc.)
   * Updates affinity based on the completion result.
   * Also emits governance signals for content effectiveness tracking.
   */
  private onRendererComplete(event: RendererCompletionEvent): void {
    if (!this.nodeId) return;

    // Map completion result to affinity delta
    // Passing increases affinity more than failing
    const affinityDelta = event.passed
      ? 0.3 + (event.score / 100) * 0.2 // 0.3 to 0.5 for passing
      : 0.1; // Small bump for attempting

    this.affinityService.incrementAffinity(this.nodeId, affinityDelta);

    // Emit governance signal for content effectiveness tracking
    this.signalService
      .recordInteractiveCompletion({
        contentId: this.nodeId,
        interactionType: event.type,
        passed: event.passed,
        score: event.score,
        details: event.details,
      })
      .subscribe();

    // Check if this triggers an attestation suggestion
    const attempts = (event.details?.['attempts'] as number) ?? 1;
    this.signalService
      .checkAttestationTrigger(this.nodeId, event.score / 100, attempts)
      .subscribe();
  }

  /**
   * Load content node by ID
   */
  private loadContent(nodeId: string): void {
    this.isLoading = true;
    this.error = null;

    this.dataLoader
      .getContent(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: contentNode => {
          if (!contentNode) {
            this.error = 'Content not found';
            this.isLoading = false;
            return;
          }

          // Set ContentNode
          this.node = contentNode;

          // Check if content is editable
          this.canEditContent = this.editorService.canEdit(contentNode);

          // Update SEO metadata for this content
          this.seoService.updateForContent({
            id: contentNode.id,
            title: contentNode.title,
            summary: contentNode.description,
            contentType: contentNode.contentType,
            thumbnailUrl: contentNode.metadata?.['thumbnailUrl'],
            authors: contentNode.metadata?.['authors'],
            createdAt: contentNode.createdAt,
            updatedAt: contentNode.updatedAt,
          });

          // Populate protocol omnibar data (shown in focused view)
          this.omnibarContentAddress = contentNode.id;
          this.omnibarReach = (contentNode.reach as string) || 'commons';
          this.omnibarDeliverySource = globalThis.location.hostname;
          // Omnibar stewards populated from allocation data in loadStewardship()
          this.omnibarStewards = [];

          // Get current affinity
          this.affinity = this.affinityService.getAffinity(nodeId);

          // Auto-track view (increment if first time)
          this.affinityService.trackView(nodeId);

          // Record attention event (dwell-qualified, deduplicated)
          this.attentionTracker.trackContentView(nodeId);

          // Manifest-driven attention signal (onConsume economic event)
          void this.signalHarness.onRendererComplete(contentNode, {
            type: 'view',
            passed: false,
            score: 0,
          });

          // Mark content as "seen" for mastery tracking
          this.agentService.markContentSeen(nodeId).pipe(takeUntil(this.destroy$)).subscribe();

          // Load related nodes
          this.loadRelatedNodes(contentNode.relatedNodeIds);

          // Load EPR relationships (protocol-level relationships from EPR Head)
          this.loadEprRelationships(contentNode.id);

          // Load containing paths (Wikipedia-style "appears in" back-links)
          this.loadContainingPaths(nodeId);

          // Load trust badge data for Attestations tab
          this.loadTrustBadge(nodeId);
          this.loadStewardship(nodeId);
          this.loadResilience(nodeId);

          // Load governance data for Governance tab
          this.loadGovernanceData(nodeId);

          // Load feedback profile and aggregated signals
          this.loadFeedbackProfile(contentNode);
          this.loadAggregatedSignals(nodeId);

          this.isLoading = false;

          // Schedule renderer loading for next change detection cycle
          // The AfterViewChecked hook will load it once the ViewChild is available
          this.pendingRendererLoad = true;
        },
        error: () => {
          this.error = 'Failed to load content';
          this.isLoading = false;
        },
      });
  }

  /**
   * Load paths that contain this content (Wikipedia-style back-links)
   */
  private loadContainingPaths(nodeId: string): void {
    this.loadingPaths = true;
    this.containingPaths = [];

    this.contentService
      .getContainingPathsSummary(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: paths => {
          this.containingPaths = paths;
          this.loadingPaths = false;
        },
        error: () => {
          this.loadingPaths = false;
        },
      });
  }

  /**
   * Load Trust Badge data for the Attestations tab
   */
  private loadTrustBadge(nodeId: string): void {
    this.isLoadingTrust = true;
    this.trustBadge = null;

    this.trustBadgeService
      .getBadge(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: badge => {
          this.trustBadge = badge;
          this.isLoadingTrust = false;
        },
        error: () => {
          this.isLoadingTrust = false;
        },
      });
  }

  /**
   * Load stewardship allocation data for the trust tab.
   */
  private loadStewardship(nodeId: string): void {
    this.stewardshipService
      .getContentStewardship(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: stewardship => {
          this.stewardship = stewardship;
          this.updateOmnibarStewards();
        },
        error: () => {
          // Stewardship is supplemental — don't block on failure
        },
      });
  }

  /**
   * Update omnibar stewards from allocation data.
   */
  private updateOmnibarStewards(): void {
    if (!this.stewardship?.allocations?.length) {
      this.omnibarStewards = [];
      return;
    }
    this.omnibarStewards = this.stewardship.allocations.map(a => ({
      humanId: a.stewardPresenceId || '',
      displayName: a.steward?.displayName || a.stewardPresenceId || 'Unknown',
      ratio: a.allocationRatio ?? 0,
    }));
  }

  /**
   * Load resilience data for the Network tab.
   * Fires both shard-level and household-level requests in parallel.
   */
  private loadResilience(nodeId: string): void {
    this.resilienceService
      .getContentResilience(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: resilience => {
          this.resilience = resilience;
        },
        error: () => {
          // Resilience is supplemental — don't block on failure
        },
      });

    this.householdResilienceService
      .get(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: hr => {
          this.householdResilience = hr;
        },
        error: () => {
          // Household resilience is supplemental — don't block on failure
        },
      });
  }

  /**
   * Trigger on-demand resilience verification (reconstruct from shards).
   */
  verifyResilience(): void {
    if (!this.node || this.isVerifying) return;
    this.isVerifying = true;
    this.verificationResult = null;

    this.resilienceService
      .verifyResilience(this.node.id)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: result => {
          this.verificationResult = result;
          this.isVerifying = false;
        },
        error: () => {
          this.isVerifying = false;
        },
      });
  }

  /**
   * Load Governance data for the Governance tab
   */
  private loadGovernanceData(nodeId: string): void {
    this.isLoadingGovernance = true;
    this.governanceState = null;
    this.challenges = [];
    this.discussions = [];

    // Load governance state
    this.governanceService
      .getGovernanceState('content', nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: state => {
          this.governanceState = state;
        },
        error: () => {
          // Governance state is optional - content may not have explicit state
        },
      });

    // Load challenges for this content
    this.governanceService
      .getChallengesForEntity('content', nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: challenges => {
          this.challenges = challenges;
          this.isLoadingGovernance = false;
        },
        error: () => {
          this.isLoadingGovernance = false;
        },
      });

    // Load discussions for this content
    this.governanceService
      .getDiscussionsForEntity('content', nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: discussions => {
          this.discussions = discussions;
        },
        error: () => {
          // Discussions are optional
        },
      });
  }

  /**
   * Load feedback profile based on content type.
   * Determines what feedback mechanisms are allowed.
   */
  private loadFeedbackProfile(node: ContentNode): void {
    // Map content type to feedback profile template
    const contentType = node.contentType;
    const profileType = this.mapContentTypeToProfileType(contentType);

    const template = DEFAULT_FEEDBACK_PROFILES[profileType];
    if (template) {
      this.feedbackProfile = createProfileFromTemplate(template, `profile-${node.id}`);

      // Extract allowed reactions from profile
      if (this.feedbackProfile.emotionalReactionConstraints?.permittedTypes) {
        this.allowedReactions = this.feedbackProfile.emotionalReactionConstraints.permittedTypes;
      } else {
        // Default reactions for learning content
        this.allowedReactions = ['moved', 'grateful', 'inspired', 'challenged', 'concerned'];
      }

      // Determine feedback context based on content type
      this.feedbackContext = this.determineFeedbackContext(node);

      // Check if feedback should be shown (view-only profile hides feedback)
      this.showFeedbackSection =
        this.feedbackProfile.permittedMechanisms.length > 0 &&
        !this.feedbackProfile.permittedMechanisms.includes('view-only');
    } else {
      // Default to learning content profile
      this.feedbackProfile = null;
      this.allowedReactions = ['moved', 'grateful', 'inspired', 'challenged', 'concerned'];
      this.feedbackContext = 'usefulness';
      this.showFeedbackSection = true;
    }
  }

  /**
   * Map content type to feedback profile type.
   */
  mapContentTypeToProfileType(contentType: string): string {
    const mapping: Record<string, string> = {
      epic: this.LEARNING_CONTENT_PROFILE,
      feature: this.LEARNING_CONTENT_PROFILE,
      scenario: this.LEARNING_CONTENT_PROFILE,
      tutorial: this.LEARNING_CONTENT_PROFILE,
      guide: this.LEARNING_CONTENT_PROFILE,
      concept: this.LEARNING_CONTENT_PROFILE,
      lesson: this.LEARNING_CONTENT_PROFILE,
      research: 'research-content',
      paper: 'research-content',
      testimony: 'personal-testimony',
      story: 'personal-testimony',
      announcement: 'community-announcement',
      proposal: 'governance-proposal',
    };
    return mapping[contentType.toLowerCase()] ?? this.LEARNING_CONTENT_PROFILE;
  }

  /**
   * Determine the appropriate feedback context for content.
   */
  private determineFeedbackContext(node: ContentNode): FeedbackContext {
    const contentType = node.contentType.toLowerCase();

    if (['research', 'paper'].includes(contentType)) {
      return 'accuracy';
    }
    if (['proposal'].includes(contentType)) {
      return 'proposal';
    }
    if (['tutorial', 'guide', 'lesson'].includes(contentType)) {
      return 'clarity';
    }
    // Default to usefulness for most learning content
    return 'usefulness';
  }

  /**
   * Load aggregated governance signals for content.
   */
  private loadAggregatedSignals(nodeId: string): void {
    this.signalService
      .getContentSignals(nodeId)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: signals => {
          this.aggregatedSignals = signals;
        },
        error: () => {
          // Signals are optional
          this.aggregatedSignals = null;
        },
      });
  }

  // =========================================================================
  // Governance Helper Methods
  // =========================================================================

  /**
   * Get human-readable governance status label
   */
  getGovernanceStatusLabel(): string {
    const status = this.governanceState?.status ?? 'unreviewed';
    const labels: Record<string, string> = {
      unreviewed: 'Unreviewed',
      'auto-approved': 'Auto-Approved',
      'community-reviewed': 'Community Reviewed',
      'elohim-reviewed': 'Elohim Reviewed',
      challenged: 'Under Challenge',
      restricted: 'Restricted',
      suspended: 'Suspended',
      removed: 'Removed',
      appealing: 'Under Appeal',
      restored: 'Restored',
      constitutional: 'Constitutional',
    };
    return labels[status] ?? status;
  }

  /**
   * Get icon for governance status
   */
  getGovernanceStatusIcon(): string {
    const status = this.governanceState?.status ?? 'unreviewed';
    const icons: Record<string, string> = {
      unreviewed: '❓',
      'auto-approved': '🤖',
      'community-reviewed': '👥',
      'elohim-reviewed': '✓',
      challenged: '⚠️',
      restricted: '🔒',
      suspended: '⏸️',
      removed: '🚫',
      appealing: '⚖️',
      restored: '↩️',
      constitutional: '📜',
    };
    return icons[status] ?? '❓';
  }

  /**
   * Get SLA status for a challenge
   */
  getSlaStatus(challenge: ChallengeRecord): string {
    if (!challenge.slaDeadline) return 'unknown';

    const deadline = new Date(challenge.slaDeadline);
    const now = new Date();
    const daysRemaining = Math.ceil((deadline.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));

    if (daysRemaining < 0) return 'sla-breached';
    if (daysRemaining <= 3) return 'sla-warning';
    return 'sla-on-track';
  }

  /**
   * Get days remaining until SLA deadline
   */
  getDaysRemaining(deadline: string | undefined): number {
    if (!deadline) return -1;

    const deadlineDate = new Date(deadline);
    const now = new Date();
    return Math.ceil((deadlineDate.getTime() - now.getTime()) / (1000 * 60 * 60 * 24));
  }

  /**
   * Format ISO date for display
   */
  formatGovernanceDate(isoDate: string | undefined): string {
    if (!isoDate) return 'Unknown';

    try {
      const date = new Date(isoDate);
      return date.toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
      });
    } catch {
      return 'Invalid date';
    }
  }

  /**
   * Switch active tab
   */
  setActiveTab(tab: 'content' | 'trust' | 'governance' | 'network'): void {
    this.activeTab = tab;
  }

  /**
   * Handle badge action click
   */
  handleAction(action: { route?: string }): void {
    if (action.route) {
      void this.router.navigate([action.route]);
    }
  }

  /**
   * Navigate to a path that contains this content
   */
  navigateToPath(pathId: string, stepIndex: number): void {
    void this.router.navigate(['/lamad/path', pathId, 'step', stepIndex]);
  }

  /**
   * Load related content nodes
   */
  private loadRelatedNodes(relatedIds: string[]): void {
    if (!relatedIds || relatedIds.length === 0) {
      this.relatedNodes = [];
      return;
    }

    // Load related nodes in parallel (limit to 5)
    const loadObservables = relatedIds
      .slice(0, 5)
      .map(id => this.dataLoader.getContent(id).pipe(catchError(() => of(null))));

    forkJoin(loadObservables)
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: nodes => {
          this.relatedNodes = nodes.filter((n): n is ContentNode => n !== null);
        },
        error: () => {
          this.relatedNodes = [];
        },
      });
  }

  /**
   * Load EPR relationships from the EPR Head for this content.
   */
  private loadEprRelationships(contentId: string): void {
    this.eprRelationships = [];
    this.eprResolver
      .resolveEprHead(contentId)
      .pipe(takeUntil(this.destroy$))
      .subscribe(head => {
        this.eprRelationships = head?.relationships ?? [];
      });
  }

  /**
   * Manually adjust affinity
   */
  adjustAffinity(delta: number): void {
    if (!this.nodeId) return;
    this.affinityService.incrementAffinity(this.nodeId, delta);
  }

  /**
   * Set affinity to a specific value
   */
  setAffinity(value: number): void {
    if (!this.nodeId) return;
    this.affinityService.setAffinity(this.nodeId, value);
  }

  /**
   * Navigate to related content
   */
  viewRelatedContent(node: ContentNode): void {
    void this.router.navigate(['/lamad/content', node.id]);
  }

  /**
   * Navigate back to lamad home
   */
  backToHome(): void {
    void this.router.navigate(['/lamad']);
  }

  /**
   * Get affinity level
   */
  getAffinityLevel(): string {
    if (this.affinity === 0) return 'unseen';
    if (this.affinity <= 0.33) return 'low';
    if (this.affinity <= 0.66) return 'medium';
    return 'high';
  }

  /**
   * Get affinity percentage
   */
  getAffinityPercentage(): number {
    return Math.round(this.affinity * 100);
  }

  /**
   * Get content type display
   */
  getContentTypeDisplay(): string {
    if (!this.node) return '';
    const displays: Record<string, string> = {
      epic: 'Epic',
      feature: 'Feature',
      scenario: 'Scenario',
    };
    return displays[this.node.contentType] ?? this.node.contentType;
  }

  /**
   * Get content type icon
   */
  getContentTypeIcon(): string {
    if (!this.node) return '';
    const icons: Record<string, string> = {
      epic: '📖',
      feature: '⚙️',
      scenario: '✓',
    };
    return icons[this.node.contentType] ?? '📄';
  }

  /**
   * Get content as string (handles string | object union type)
   */
  getStringContent(content: string | object): string {
    if (typeof content === 'string') {
      return content;
    }
    return JSON.stringify(content, null, 2);
  }

  /**
   * Get affinity percentage for related node
   */
  getRelatedNodeAffinity(nodeId: string): number {
    return Math.round(this.affinityService.getAffinity(nodeId) * 100);
  }

  /**
   * Get metadata category
   */
  getMetadataCategory(): string | null {
    if (!this.node?.metadata?.category) return null;
    return this.node.metadata.category ?? null;
  }

  /**
   * Get metadata authors as joined string
   */
  getMetadataAuthors(): string | null {
    if (!this.node?.metadata?.authors) return null;
    const authors = this.node.metadata.authors;
    if (Array.isArray(authors) && authors.length > 0) {
      return authors.join(', ');
    }
    return null;
  }

  /**
   * Get metadata version
   */
  getMetadataVersion(): string | null {
    if (!this.node?.metadata?.version) return null;
    return this.node.metadata.version ?? null;
  }

  /**
   * Reach icon — concentric circles metaphor. More circles = wider reach.
   * Handles both frontend ReachLevel values and backend trust-tier values.
   */
  getReachIcon(): string {
    const reach = (this.node?.reach as string) || 'commons';
    switch (reach) {
      case 'private':
      case 'self':
        return '\u{1F512}'; // lock
      case 'intimate':
        return '\u{25C9}'; // fisheye (single filled dot)
      case 'trusted':
        return '\u{25CE}'; // bullseye
      case 'familiar':
      case 'invited':
        return '\u{25CB}\u{25CF}'; // two circles
      case 'community':
      case 'local':
      case 'neighborhood':
      case 'municipal':
        return '\u{25CE}\u{25CB}'; // bullseye + circle
      case 'commons':
      case 'public':
      case 'regional':
      case 'bioregional':
      default:
        return '\u{25CB}\u{25CB}\u{25CB}'; // three open circles
    }
  }

  /**
   * Reach tooltip — explains what the reach level means for this content.
   */
  getReachTooltip(): string {
    const reach = (this.node?.reach as string) || 'commons';
    const descriptions: Record<string, string> = {
      commons: 'Commons — accessible to everyone',
      public: 'Public — accessible to everyone',
      community: 'Community — requires collective membership',
      familiar: 'Familiar — requires shared collective with steward',
      trusted: 'Trusted — requires relationship with steward',
      intimate: 'Intimate — requires mutual intimate relationship',
      private: 'Private — creator only',
      self: 'Private — creator only',
      local: 'Local — household/immediate area',
      neighborhood: 'Neighborhood — local area',
      municipal: 'Municipal — city/town',
      regional: 'Regional — state/province',
      federated: 'Federated — multiple communities',
      invited: 'Invited — explicitly invited individuals',
    };
    return descriptions[reach] || `Reach: ${reach}`;
  }

  /**
   * Resilience icon — household-first protection status indicator.
   */
  getResilienceIcon(): string {
    const s = this.householdResilience?.protectionStatus;
    switch (s) {
      case 'protected':
        return '\u{1F7E2}'; // green circle
      case 'partial':
        return '\u{1F7E1}'; // yellow circle
      case 'at-risk':
        return '\u{1F534}'; // red circle
      default:
        return '\u{1F504}'; // loading (arrows)
    }
  }

  /**
   * Resilience tooltip — household-first with shard-level data as supplement.
   */
  getResilienceTooltip(): string {
    if (!this.householdResilience) return 'Loading resilience\u2026';
    const hr = this.householdResilience;
    const lines: string[] = [`Households stewarding: ${hr.householdsStewarding}`];
    if (hr.householdsReciprocated > 0) {
      lines.push(`Reciprocated: ${hr.householdsReciprocated}`);
    }
    lines.push(`Protection: ${hr.protectionStatus}`);
    if (this.resilience?.encoding) {
      lines.push(`Encoding: ${this.resilience.encoding.strategy}`);
    }
    if (this.resilience?.distribution) {
      lines.push(`Peers online: ${this.resilience.distribution.distinctPeers}`);
    }
    const healthScore = this.resilience?.health?.score;
    if (healthScore !== undefined) {
      lines.push(`Health: ${Math.round(healthScore * 100)}%`);
    }
    return lines.join('\n');
  }

  // =========================================================================
  // Content Flag Helpers
  // =========================================================================

  getFlags(): ContentFlag[] {
    return this.node?.flags || [];
  }

  getFlagLabel(type: string): string {
    const labels: Record<string, string> = {
      disputed: 'Disputed',
      outdated: 'Outdated',
      'appeal-pending': 'Appeal Pending',
      'under-review': 'Under Review',
      'partial-revocation': 'Partial Revocation',
    };
    return labels[type] || type;
  }

  getFlagClass(type: string): string {
    return `flag-tag flag-${type}`;
  }

  // =========================================================================
  // Path Context & Return Navigation Methods
  // =========================================================================

  /**
   * Return to the path from a detour.
   */
  returnToPath(): void {
    const returnRoute = this.pathContextService.returnToPath();
    if (returnRoute) {
      void this.router.navigate(returnRoute);
    }
  }

  /**
   * Handle node selection from the mini-graph.
   */
  onGraphNodeSelected(nodeId: string): void {
    // Track the detour if we're in a path context
    if (this.pathContext && this.nodeId) {
      this.pathContextService.startDetour({
        fromContentId: this.nodeId,
        toContentId: nodeId,
        detourType: 'related',
        timestamp: new Date().toISOString(),
      });
    }

    // Navigate to the selected content
    void this.router.navigate(['/resource', nodeId]);
  }

  /**
   * Navigate to full graph explorer with focus on current content.
   */
  exploreInGraph(): void {
    if (!this.nodeId) return;

    // Track the detour if we're in a path context
    if (this.pathContext) {
      this.pathContextService.startDetour({
        fromContentId: this.nodeId,
        toContentId: this.nodeId,
        detourType: 'graph-explore',
        timestamp: new Date().toISOString(),
      });
    }

    // Navigate to graph explorer
    void this.router.navigate(['/lamad/explore'], {
      queryParams: {
        focus: this.nodeId,
        ...(this.pathContext
          ? {
              fromPath: this.pathContext.pathId,
              returnStep: this.pathContext.stepIndex,
            }
          : {}),
      },
    });
  }

  // =========================================================================
  // Focused View (Immersive Mode) Methods
  // =========================================================================

  /**
   * Handle escape key to exit focused view mode.
   */
  @HostListener('document:keydown.escape')
  onEscapeKey(): void {
    if (this.isFocusedView) {
      this.onFocusedViewToggle(false);
    }
  }

  /**
   * Toggle focused view mode.
   * Waits for CSS transition to complete before reloading content
   * so iframes can measure the new viewport dimensions correctly.
   */
  onFocusedViewToggle(active: boolean): void {
    this.isFocusedView = active;

    // Toggle body class for global effects (hide navigation, lock scroll)
    if (active) {
      this.document.body.classList.add(this.FOCUSED_VIEW_MODE_CLASS);
    } else {
      this.document.body.classList.remove(this.FOCUSED_VIEW_MODE_CLASS);
    }

    // Wait for CSS transition to complete, then reload content
    // This ensures iframes get the correct viewport dimensions
    setTimeout(() => {
      this.reloadRenderer();
    }, this.TRANSITION_DURATION);
  }

  /**
   * Reload the renderer to refresh content with new dimensions.
   * Destroys and recreates the renderer component.
   */
  private reloadRenderer(): void {
    if (this.node && this.rendererHost) {
      this.destroyRenderer();
      this.rendererHost.clear();
      this.loadRenderer();
    }
  }
}
