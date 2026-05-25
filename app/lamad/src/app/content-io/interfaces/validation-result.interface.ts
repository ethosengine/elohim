/**
 * Validation result from content I/O operations.
 */
export interface ValidationError {
  code: string;
  message: string;
  line?: number;
  column?: number;
  context?: string;
}

export interface ValidationWarning {
  code: string;
  message: string;
  line?: number;
  column?: number;
  suggestion?: string;
}

export interface ValidationResult {
  /** Whether the content is valid for the format */
  valid: boolean;

  /** Critical errors that prevent import/export */
  errors: ValidationError[];

  /** Non-blocking warnings about potential issues */
  warnings: ValidationWarning[];

  /** Preview of what would be created (for import validation) */
  parsedPreview?: {
    title?: string;
    description?: string;
    contentType?: string;
    tags?: string[];
    [key: string]: unknown;
  };

  /** Statistics about the content */
  stats?: {
    wordCount?: number;
    lineCount?: number;
    sectionCount?: number;
    [key: string]: unknown;
  };
}
