/**
 * AppealWizardComponent - Multi-step appeal filing wizard
 *
 * Guides users through filing an appeal against stewardship grants or policies.
 * Appeals are an inalienable right - this feature can never be disabled.
 *
 * Steps:
 * 1. Select appeal type
 * 2. Provide grounds and evidence
 * 3. Optional: Request advocate
 * 4. Review and submit
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, signal, computed, input, output } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { Router, ActivatedRoute, RouterModule } from '@angular/router';

import {
  type StewardshipGrant,
  type StewardshipAppeal,
  type AppealType,
  type FileAppealInput,
  getAuthorityBasisLabel,
} from '../../models/stewardship.model';
import { StewardshipService } from '../../services/stewardship.service';

/** Appeal wizard step */
type WizardStep = 'type' | 'grounds' | 'advocate' | 'review';

/** Predefined grounds for appeals */
const APPEAL_GROUNDS: Record<AppealType, string[]> = {
  scope: [
    'Authority does not extend to these capabilities',
    'Grant exceeds original agreement',
    'Capabilities should be narrower',
    'Changed circumstances require scope reduction',
  ],
  excessive: [
    'Restrictions are disproportionate to need',
    'Less restrictive alternatives available',
    'Restrictions harm my wellbeing',
    'Pattern of over-restriction',
  ],
  invalid_evidence: [
    'Evidence was falsified or misleading',
    'Evidence no longer applies',
    'Evidence was obtained improperly',
    'Verifier was not qualified',
  ],
  capability_request: [
    'Demonstrated responsibility over time',
    'Changed circumstances support expansion',
    'Need capability for legitimate purpose',
    'Restriction no longer necessary',
  ],
};

@Component({
  selector: 'app-appeal-wizard',
  standalone: true,
  imports: [CommonModule, FormsModule, RouterModule],
  templateUrl: './appeal-wizard.component.html',
  styleUrls: ['./appeal-wizard.component.css'],
})
export class AppealWizardComponent implements OnInit {
  private readonly stewardship = inject(StewardshipService);
  private readonly router = inject(Router);
  private readonly route = inject(ActivatedRoute);

  // ===========================================================================
  // Inputs/Outputs
  // ===========================================================================

  /** Grant ID to appeal (from route or input) */
  readonly grantIdInput = input<string | undefined>(undefined, { alias: 'grantId' });

  /** Policy ID to appeal */
  readonly policyIdInput = input<string | undefined>(undefined, { alias: 'policyId' });

  /** Emitted when appeal is filed */
  readonly appealFiled = output<StewardshipAppeal>();

  // ===========================================================================
  // State
  // ===========================================================================

  readonly isLoading = signal(true);
  readonly isSubmitting = signal(false);
  readonly error = signal<string | null>(null);
  readonly currentStep = signal<WizardStep>('type');

  /** The grant being appealed */
  readonly grant = signal<StewardshipGrant | null>(null);

  // Form state
  readonly appealType = signal<AppealType | null>(null);
  readonly selectedGrounds = signal<string[]>([]);
  readonly customGrounds = signal('');
  readonly evidenceDescription = signal('');
  readonly wantsAdvocate = signal(false);
  readonly advocateNotes = signal('');

  // ===========================================================================
  // Computed State
  // ===========================================================================

  /** Steps and their completion status */
  readonly steps = computed<{ id: WizardStep; label: string; completed: boolean }[]>(() => {
    const type = this.appealType();
    const grounds = this.selectedGrounds();

    return [
      { id: 'type', label: 'Appeal Type', completed: type !== null },
      {
        id: 'grounds',
        label: 'Grounds',
        completed: grounds.length > 0 || this.customGrounds().length > 0,
      },
      { id: 'advocate', label: 'Advocate', completed: true }, // Optional step
      { id: 'review', label: 'Review', completed: false },
    ];
  });

  /** Current step index */
  readonly currentStepIndex = computed(() => {
    const steps = this.steps();
    const current = this.currentStep();
    return steps.findIndex(s => s.id === current);
  });

  /** Whether can proceed to next step */
  readonly canProceed = computed(() => {
    const step = this.currentStep();

    switch (step) {
      case 'type':
        return this.appealType() !== null;
      case 'grounds':
        return this.selectedGrounds().length > 0 || this.customGrounds().trim().length > 0;
      case 'advocate':
        return true; // Optional
      case 'review':
        return false; // Submit instead
      default:
        return false;
    }
  });

  /** Available grounds for selected appeal type */
  readonly availableGrounds = computed(() => {
    const type = this.appealType();
    if (!type) return [];
    return APPEAL_GROUNDS[type] || [];
  });

  /** Current appeal type label for display */
  readonly appealTypeLabel = computed(() => {
    const type = this.appealType();
    if (!type) return '';
    const option = this.appealTypeOptions.find(o => o.value === type);
    return option?.label ?? type;
  });

  /** Appeal type options */
  readonly appealTypeOptions: { value: AppealType; label: string; description: string }[] = [
    {
      value: 'scope',
      label: 'Scope Challenge',
      description: 'The capabilities granted exceed what is necessary or appropriate.',
    },
    {
      value: 'excessive',
      label: 'Excessive Restrictions',
      description: 'Current restrictions are disproportionate to the stated need.',
    },
    {
      value: 'invalid_evidence',
      label: 'Invalid Evidence',
      description:
        'The evidence supporting this grant is invalid, outdated, or improperly obtained.',
    },
    {
      value: 'capability_request',
      label: 'Capability Request',
      description: 'Request to expand capabilities based on demonstrated responsibility.',
    },
  ];

  // ===========================================================================
  // Lifecycle
  // ===========================================================================

  ngOnInit(): void {
    const grantId = this.route.snapshot.paramMap.get('grantId') || this.grantIdInput();

    if (grantId) {
      this.loadGrant(grantId);
    } else {
      this.error.set('No grant specified for appeal.');
      this.isLoading.set(false);
    }
  }

  // ===========================================================================
  // Data Loading
  // ===========================================================================

  async loadGrant(grantId: string): Promise<void> {
    this.isLoading.set(true);
    this.error.set(null);

    try {
      // Load my stewards to find the grant
      const stewards = await this.stewardship.getMyStewards();
      const grant = stewards.find(g => g.id === grantId);

      if (grant) {
        this.grant.set(grant);
      } else {
        this.error.set('Grant not found or you are not the subject of this grant.');
      }
    } catch (err) {
      console.error('[AppealWizard] Load failed:', err);
      this.error.set('Failed to load grant information.');
    } finally {
      this.isLoading.set(false);
    }
  }

  // ===========================================================================
  // Navigation
  // ===========================================================================

  nextStep(): void {
    const current = this.currentStep();
    const steps: WizardStep[] = ['type', 'grounds', 'advocate', 'review'];
    const index = steps.indexOf(current);

    if (index < steps.length - 1) {
      this.currentStep.set(steps[index + 1]);
    }
  }

  previousStep(): void {
    const current = this.currentStep();
    const steps: WizardStep[] = ['type', 'grounds', 'advocate', 'review'];
    const index = steps.indexOf(current);

    if (index > 0) {
      this.currentStep.set(steps[index - 1]);
    }
  }

  goToStep(step: WizardStep): void {
    // Can only go to completed steps or the current step
    const steps = this.steps();
    const targetIndex = steps.findIndex(s => s.id === step);
    const currentIndex = this.currentStepIndex();

    if (targetIndex <= currentIndex || steps[targetIndex - 1]?.completed) {
      this.currentStep.set(step);
    }
  }

  // ===========================================================================
  // Form Handlers
  // ===========================================================================

  selectAppealType(type: AppealType): void {
    this.appealType.set(type);
    // Clear grounds when type changes
    this.selectedGrounds.set([]);
  }

  toggleGround(ground: string): void {
    const current = this.selectedGrounds();
    if (current.includes(ground)) {
      this.selectedGrounds.set(current.filter(g => g !== ground));
    } else {
      this.selectedGrounds.set([...current, ground]);
    }
  }

  isGroundSelected(ground: string): boolean {
    return this.selectedGrounds().includes(ground);
  }

  // ===========================================================================
  // Submit
  // ===========================================================================

  async submitAppeal(): Promise<void> {
    const grant = this.grant();
    const type = this.appealType();

    if (!grant || !type) {
      this.error.set('Missing required information.');
      return;
    }

    this.isSubmitting.set(true);
    this.error.set(null);

    try {
      // Combine selected and custom grounds
      const grounds = [...this.selectedGrounds()];
      const custom = this.customGrounds().trim();
      if (custom) {
        grounds.push(custom);
      }

      // Build evidence JSON
      const evidence = {
        description: this.evidenceDescription(),
        submittedAt: new Date().toISOString(),
      };

      const input: FileAppealInput = {
        grantId: grant.id,
        policyId: this.policyIdInput(),
        appealType: type,
        grounds,
        evidenceJson: JSON.stringify(evidence),
        advocateId: this.wantsAdvocate() ? 'elohim' : undefined, // Request Elohim as advocate
      };

      const appeal = await this.stewardship.fileAppeal(input);

      if (appeal) {
        this.appealFiled.emit(appeal);
        // Navigate to confirmation
        this.router.navigate(['../appeal-filed', appeal.id], { relativeTo: this.route });
      } else {
        this.error.set('Failed to file appeal. Please try again.');
      }
    } catch (err) {
      console.error('[AppealWizard] Submit failed:', err);
      this.error.set('Failed to submit appeal.');
    } finally {
      this.isSubmitting.set(false);
    }
  }

  // ===========================================================================
  // Helpers
  // ===========================================================================

  getAuthorityBasisLabel(basis: string): string {
    return getAuthorityBasisLabel(basis as any);
  }

  clearError(): void {
    this.error.set(null);
  }
}
