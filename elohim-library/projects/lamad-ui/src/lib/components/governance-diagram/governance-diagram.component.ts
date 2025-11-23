/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';

interface GovernanceLayer {
  id: number;
  name: string;
  desc: string;
  consensus: string;
}

@Component({
  selector: 'lamad-governance-diagram',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './governance-diagram.component.html',
  styleUrls: ['./governance-diagram.component.css']
})
export class GovernanceDiagramComponent {
  activeLayer = 2;

  layers: GovernanceLayer[] = [
    { id: 0, name: "Global", desc: "Universal Principles (No Extinction)", consensus: "All Elohim + Human Council" },
    { id: 1, name: "Governing States", desc: "Constitutional Interpretations", consensus: "State Elohim + Citizenry" },
    { id: 2, name: "Regional", desc: "Local Communities, Municipal, and Bioregional Norms & Policies", consensus: "Local Elohim + Residents" },
    { id: 3, name: "Family", desc: "Traditions & Private Governance", consensus: "Family Elohim + Consensus" },
    { id: 4, name: "Individual", desc: "Maximum Autonomy", consensus: "Sovereign Choice" },
  ];

  setActiveLayer(id: number) {
    this.activeLayer = id;
  }
}