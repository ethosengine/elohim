import { CommonModule, DOCUMENT } from '@angular/common';
import {
  Component,
  OnInit,
  OnDestroy,
  ViewChild,
  ViewContainerRef,
  ComponentRef,
  inject,
  AfterViewChecked,
  HostListener,
  Inject,
} from '@angular/core';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

import { takeUntil, catchError } from 'rxjs/operators';

import { Subject, Subscription, forkJoin, of } from 'rxjs';

import { TrustBadge } from '@app/elohim/models/trust-badge.model';
import { AffinityTrackingService } from '@app/elohim/services/affinity-tracking.service';
import { AgentService } from '@app/elohim/services/agent.service';
import {
  DataLoaderService,
  ChallengeRecord,
  DiscussionRecord,
  GovernanceStateRecord,
} from '@app/elohim/services/data-loader.service';
import {
  GovernanceSignalService,
  AggregatedSignals,
} from '@app/elohim/services/governance-signal.service';
import { GovernanceService } from '@app/elohim/services/governance.service';
import { TrustBadgeService } from '@app/elohim/services/trust-badge.service';
import {
  EmotionalReactionType,
  FeedbackProfile,
  DEFAULT_FEEDBACK_PROFILES,
  createProfileFromTemplate,
} from '@app/lamad/models/feedback-profile.model';
import {
  GraduatedFeedbackComponent,
  FeedbackContext,
} from '@app/qahal/components/graduated-feedback/graduated-feedback.component';
import { ReactionBarComponent } from '@app/qahal/components/reaction-bar/reaction-bar.component';

import { SeoService } from '../../../services/seo.service';
import { ContentNode } from '../../models/content-node.model';
import { ContentService } from '../../services/content.service';

import {
  RendererRegistryService,
  ContentRenderer,
  RendererCompletionEvent,
} from '../../renderers/renderer-registry.service';

// Content I/O for download functionality
import { ContentDownloadComponent } from '../../content-io/components/content-download/content-download.component';
import { ContentEditorService } from '../../content-io/services/content-editor.service';

// Exploration components
import { PathContextService } from '../../services/path-context.service';
import { PathContext } from '../../models/exploration-context.model';
import { FocusedViewToggleComponent } from '../focused-view-toggle/focused-view-toggle.component';
import { MiniGraphComponent } from '../mini-graph/mini-graph.component';

// Governance feedback components

// Focused view toggle for immersive content

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
    FocusedViewToggleComponent,
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

  constructor(
    private readonly route: ActivatedRoute,
    private readonly router: Router,
    private readonly affinityService: AffinityTrackingService,
    private readonly agentService: AgentService,
    private readonly rendererRegistry: RendererRegistryService,
    private readonly contentService: ContentService,
    private readonly dataLoader: DataLoaderService,
    private readonly trustBadgeService: TrustBadgeService,
    private readonly editorService: ContentEditorService,
    private readonly pathContextService: PathContextService,
    private readonly governanceService: GovernanceService,
    private readonly signalService: GovernanceSignalService,
    @Inject(DOCUMENT) private readonly document: Document
  ) {}

  ngOnInit(): void {
    // Handle direct content access: /lamad/resource/:resourceId
    this.route.params.pipe(takeUntil(this.destroy$)).subscribe(params => {
      const resourceId = params['resourceId'];
      if (resourceId) {
        this.nodeId = resourceId;
        this.loadContent(resourceId);
      }
    });

    // Listen for affinity changes
    this.affinityService.changes$.pipe(takeUntil(this.destroy$)).subscribe(change => {
      if (change && change.nodeId === this.nodeId) {
        this.affinity = change.newValue;
      }
    });

    // Subscribe to path context for return navigation
    this.pathContextService.context$.pipe(takeUntil(this.destroy$)).subscribe(context => {
      this.pathContext = context;
      this.hasReturnPath =
        context !== null && context.detourStack !== undefined && context.detourStack.length > 0;
    });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
    this.destroyRenderer();
    // Clean up focused view mode if active
    this.document.body.classList.remove('focused-view-mode');
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
    const instance = this.rendererRef.instance as any;
    if (instance.complete) {
      this.rendererSubscription = instance.complete.subscribe((event: RendererCompletionEvent) =>
        this.onRendererComplete(event)
      );
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

          // Get current affinity
          this.affinity = this.affinityService.getAffinity(nodeId);

          // Auto-track view (increment if first time)
          this.affinityService.trackView(nodeId);

          // Mark content as "seen" for mastery tracking
          this.agentService.markContentSeen(nodeId).pipe(takeUntil(this.destroy$)).subscribe();

          // Load related nodes
          this.loadRelatedNodes(contentNode.relatedNodeIds);

          // Load containing paths (Wikipedia-style "appears in" back-links)
          this.loadContainingPaths(nodeId);

          // Load trust badge data for Attestations tab
          this.loadTrustBadge(nodeId);

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
      epic: 'learning-content',
      feature: 'learning-content',
      scenario: 'learning-content',
      tutorial: 'learning-content',
      guide: 'learning-content',
      concept: 'learning-content',
      lesson: 'learning-content',
      research: 'research-content',
      paper: 'research-content',
      testimony: 'personal-testimony',
      story: 'personal-testimony',
      announcement: 'community-announcement',
      proposal: 'governance-proposal',
    };
    return mapping[contentType.toLowerCase()] || 'learning-content';
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
    const status = this.governanceState?.status || 'unreviewed';
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
    return labels[status] || status;
  }

  /**
   * Get icon for governance status
   */
  getGovernanceStatusIcon(): string {
    const status = this.governanceState?.status || 'unreviewed';
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
    return icons[status] || '❓';
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
  handleAction(action: any): void {
    if (action.route) {
      this.router.navigate([action.route]);
    }
    // Actions without routes are no-ops (e.g., placeholder actions)
  }

  /**
   * Navigate to a path that contains this content
   */
  navigateToPath(pathId: string, stepIndex: number): void {
    this.router.navigate(['/lamad/path', pathId, 'step', stepIndex]);
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
    this.router.navigate(['/lamad/content', node.id]);
  }

  /**
   * Navigate back to lamad home
   */
  backToHome(): void {
    this.router.navigate(['/lamad']);
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
    return displays[this.node.contentType] || this.node.contentType;
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
    return icons[this.node.contentType] || '📄';
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
    if (!this.node?.metadata?.['category']) return null;
    return this.node.metadata['category'];
  }

  /**
   * Get metadata authors as joined string
   */
  getMetadataAuthors(): string | null {
    if (!this.node?.metadata?.['authors']) return null;
    const authors = this.node.metadata['authors'];
    if (Array.isArray(authors) && authors.length > 0) {
      return authors.join(', ');
    }
    return null;
  }

  /**
   * Get metadata version
   */
  getMetadataVersion(): string | null {
    if (!this.node?.metadata?.['version']) return null;
    return this.node.metadata['version'];
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
      this.router.navigate(returnRoute);
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
    this.router.navigate(['/lamad/resource', nodeId]);
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
    this.router.navigate(['/lamad/explore'], {
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
      this.document.body.classList.add('focused-view-mode');
    } else {
      this.document.body.classList.remove('focused-view-mode');
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
