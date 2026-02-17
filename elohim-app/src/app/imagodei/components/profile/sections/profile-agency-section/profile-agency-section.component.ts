import { CommonModule } from '@angular/common';
import { Component, input, output } from '@angular/core';
import { FormsModule } from '@angular/forms';

import {
  type AgencyStageInfo,
  type AgencyState,
  type ConnectionStatus,
  AGENCY_STAGES,
} from '../../../../models/agency.model';

@Component({
  selector: 'app-profile-agency-section',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './profile-agency-section.component.html',
  styleUrls: ['./profile-agency-section.component.css'],
})
export class ProfileAgencySectionComponent {
  readonly agencyInfo = input.required<AgencyStageInfo>();
  readonly agencyState = input.required<AgencyState>();
  readonly connectionStatus = input.required<ConnectionStatus>();
  readonly canUpgrade = input(false);
  readonly nextStageInfo = input<AgencyStageInfo | null>(null);
  readonly nextStageLabel = input('Next Stage');

  readonly isTauriApp = input(false);
  readonly graduationStatus = input<string>('idle');
  readonly graduationError = input<string>('');
  readonly isGraduationEligible = input(false);

  readonly confirmGraduation = output<string>();

  graduationPassword = '';

  /** All stages in order for the progression stepper */
  readonly stages: AgencyStageInfo[] = [
    AGENCY_STAGES['visitor'],
    AGENCY_STAGES['hosted'],
    AGENCY_STAGES['app-steward'],
    AGENCY_STAGES['node-steward'],
  ];

  getStageBadgeClass(): string {
    return `stage-badge--${this.agencyState().currentStage}`;
  }

  getStatusDotClass(): string {
    return `status-dot--${this.connectionStatus().state}`;
  }

  getStepperClass(stage: AgencyStageInfo): string {
    const currentOrder = this.agencyInfo().order;
    if (stage.order < currentOrder) return 'step--completed';
    if (stage.order === currentOrder) return 'step--current';
    return 'step--future';
  }

  onConfirmGraduation(): void {
    if (this.graduationPassword.trim()) {
      this.confirmGraduation.emit(this.graduationPassword);
      this.graduationPassword = '';
    }
  }
}
