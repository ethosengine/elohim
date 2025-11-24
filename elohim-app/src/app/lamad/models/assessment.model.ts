// ... existing ContentNode ...

/**
 * Represents an Assessment (Quiz) Node
 * Used for earning attestations and validating knowledge.
 */
export interface AssessmentNode extends ContentNode {
  contentType: 'assessment';
  contentFormat: 'json';
  
  metadata: ContentMetadata & {
    // What attestation is earned upon passing?
    attestationGranted?: string;
    
    // Configuration for the assessment
    assessmentConfig: {
      questions: AssessmentQuestion[];
      passingScore: number; // Percentage (0-100) or raw score
      timeLimit?: number; // Minutes
      allowedAttempts?: number;
    };
  };
}

export interface AssessmentQuestion {
  id: string;
  text: string;
  type: 'multiple-choice' | 'true-false' | 'short-answer';
  options?: string[]; // For MC/TF
  correctAnswer: string | string[]; // For auto-grading
  explanation?: string; // Shown after answering
  points?: number;
}
