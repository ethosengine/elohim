import { Injectable, Type } from "@angular/core";

// @coverage: 100.0% (2026-02-24)

import {
  FRAME_CAPABILITIES_TABLE,
  FrameCapability,
} from "../../../../generated/schema-enums";
import { Html5AppBody } from "../../../generated/body-types";
import {
  IframeRendererComponent,
  Html5AppContent,
} from "../../../renderers/iframe-renderer/iframe-renderer.component";
import {
  ContentFormatPlugin,
  ContentRenderer,
  ContentEditorComponent,
  EditorConfig,
  DEFAULT_EDITOR_CONFIG,
} from "../../interfaces/content-format-plugin.interface";
import {
  ContentIOImportResult,
  ContentIOExportInput,
} from "../../interfaces/content-io-plugin.interface";
import { FormatMetadata } from "../../interfaces/format-metadata.interface";
import {
  ValidationResult,
  ValidationError,
  ValidationWarning,
} from "../../interfaces/validation-result.interface";

/**
 * Html5AppFormatPlugin - Plugin for HTML5 interactive applications.
 *
 * HTML5 apps are served from Doorway's /apps/ endpoint, which extracts
 * and caches zip files from the DHT. The Angular side is a thin iframe
 * wrapper that just sets the src URL.
 *
 * Content Structure — the record's contentBody, an Html5AppBody
 * (elohim/sdk/domains/lamad/schemas/html5-app-body.schema.json):
 * ```typescript
 * {
 *   contentFormat: 'html5-app',
 *   content: {
 *     slug: 'evolution-of-trust',   // URL namespace
 *     entryPoint: 'index.html',      // File to load
 *     fallbackUrl?: 'https://...',   // External fallback
 *     capabilities: ['forms'],       // What the frame may do beyond its own scripts
 *   }
 * }
 * ```
 *
 * Scripts are never a capability, so nothing declares `javascript`; the old
 * `metadata.requiredCapabilities` / `securityPolicy.sandbox` fields are not
 * written. A legacy sandbox token on the input (allow-forms, allow-popups,
 * allow-downloads) is read as the capability it requests; any other token
 * (allow-scripts, allow-same-origin) requests nothing.
 *
 * @example
 * ```typescript
 * // Register in content-io module
 * registry.register(new Html5AppFormatPlugin());
 * ```
 */
@Injectable({
  providedIn: "root",
})
export class Html5AppFormatPlugin implements ContentFormatPlugin {
  // ═══════════════════════════════════════════════════════════════════════════
  // Identity
  // ═══════════════════════════════════════════════════════════════════════════

  readonly formatId = "html5-app";
  readonly displayName = "HTML5 Application";
  readonly fileExtensions = [".zip"];
  readonly mimeTypes = ["application/zip", "application/x-zip-compressed"];

  // ═══════════════════════════════════════════════════════════════════════════
  // Capabilities
  // ═══════════════════════════════════════════════════════════════════════════

  readonly canImport = true;
  readonly canExport = true;
  readonly canValidate = true;
  readonly canRender = true;
  readonly canEdit = false; // HTML5 apps are not editable in-browser

  // ═══════════════════════════════════════════════════════════════════════════
  // Import
  // ═══════════════════════════════════════════════════════════════════════════

  // eslint-disable-next-line @typescript-eslint/require-await -- Method signature requires async for interface compliance
  async import(input: string | File): Promise<ContentIOImportResult> {
    // For HTML5 apps, import creates the content structure from a zip file
    // The actual zip is stored separately as a blob in the DHT

    if (typeof input === "string") {
      // Parse JSON content structure
      let parsed: unknown;
      try {
        parsed = JSON.parse(input);
      } catch {
        throw new Error("Invalid HTML5 app JSON structure");
      }
      if (
        typeof parsed !== "object" ||
        parsed === null ||
        Array.isArray(parsed)
      ) {
        throw new Error("Invalid HTML5 app JSON structure");
      }
      return this.createImportResult(
        toHtml5AppBody(parsed as Record<string, unknown>),
      );
    }

    // File upload - extract metadata from zip
    const filename = input.name.replace(/\.zip$/i, "");
    const slug = this.slugify(filename);

    const body: Html5AppBody = {
      slug,
      entryPoint: "index.html",
      capabilities: [],
    };
    return {
      content: body,
      contentFormat: "html5-app",
      contentType: "simulation",
      title: this.humanize(filename),
      description: `Interactive HTML5 application: ${this.humanize(filename)}`,
      tags: ["html5-app", "interactive", "simulation"],
      metadata: {
        originalFilename: input.name,
        sizeBytes: input.size,
        embedStrategy: "iframe",
      },
    };
  }

  private createImportResult(content: Html5AppBody): ContentIOImportResult {
    return {
      content,
      contentFormat: "html5-app",
      contentType: "simulation",
      title: this.humanize(content.slug),
      description: `Interactive HTML5 application: ${this.humanize(content.slug)}`,
      tags: ["html5-app", "interactive"],
      metadata: {
        embedStrategy: "iframe",
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export
  // ═══════════════════════════════════════════════════════════════════════════

  // eslint-disable-next-line @typescript-eslint/require-await -- Method signature requires async for interface compliance
  async export(node: ContentIOExportInput): Promise<string> {
    // Export the content structure as JSON
    // The actual zip blob is handled separately by the blob service
    const content = node.content as Html5AppBody;

    return JSON.stringify(
      {
        slug: content.slug,
        entryPoint: content.entryPoint,
        fallbackUrl: content.fallbackUrl,
        capabilities: content.capabilities,
      },
      null,
      2,
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Validation
  // ═══════════════════════════════════════════════════════════════════════════

  // eslint-disable-next-line @typescript-eslint/require-await -- Method signature requires async for interface compliance
  async validate(input: string | File): Promise<ValidationResult> {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    if (typeof input === "string") {
      const jsonResult = this.validateJsonContent(input);
      errors.push(...jsonResult.errors);
      warnings.push(...jsonResult.warnings);
    } else {
      const fileResult = this.validateZipFile(input);
      errors.push(...fileResult.errors);
      warnings.push(...fileResult.warnings);
    }

    return {
      valid: errors.length === 0,
      errors,
      warnings,
      stats: {
        formatId: this.formatId,
        isFile: input instanceof File,
      },
    };
  }

  private validateJsonContent(input: string): {
    errors: ValidationError[];
    warnings: ValidationWarning[];
  } {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    let parsed: unknown;
    try {
      parsed = JSON.parse(input);
    } catch (e) {
      errors.push({
        code: "INVALID_JSON",
        message: `Invalid JSON: ${e instanceof Error ? e.message : "Parse error"}`,
        line: 1,
      });
      return { errors, warnings };
    }

    const content = parsed as Partial<Html5AppContent>;
    this.validateSlug(content, errors);
    this.validateEntryPoint(content, errors, warnings);
    this.validateFallbackUrl(content, warnings);

    return { errors, warnings };
  }

  private validateSlug(
    content: Partial<Html5AppContent>,
    errors: ValidationError[],
  ): void {
    if (!content.slug || typeof content.slug !== "string") {
      errors.push({
        code: "MISSING_SLUG",
        message: "Missing required field: slug (string)",
      });
    } else if (!/^[a-z0-9-]+$/.test(content.slug)) {
      errors.push({
        code: "INVALID_SLUG",
        message: "slug must be lowercase alphanumeric with hyphens only",
      });
    }
  }

  private validateEntryPoint(
    content: Partial<Html5AppContent>,
    errors: ValidationError[],
    warnings: ValidationWarning[],
  ): void {
    if (!content.entryPoint || typeof content.entryPoint !== "string") {
      errors.push({
        code: "MISSING_ENTRY_POINT",
        message: "Missing required field: entryPoint (string)",
      });
    } else if (!content.entryPoint.endsWith(".html")) {
      warnings.push({
        code: "ENTRY_POINT_NOT_HTML",
        message: "entryPoint should typically be an HTML file",
        suggestion: "Use index.html as the entry point",
      });
    }
  }

  private validateFallbackUrl(
    content: Partial<Html5AppContent>,
    warnings: ValidationWarning[],
  ): void {
    if (!content.fallbackUrl) {
      warnings.push({
        code: "NO_FALLBACK",
        message:
          "No fallbackUrl specified. App will fail if doorway is unavailable.",
        suggestion: "Add fallbackUrl pointing to the original source",
      });
    }
  }

  private validateZipFile(input: File): {
    errors: ValidationError[];
    warnings: ValidationWarning[];
  } {
    const errors: ValidationError[] = [];
    const warnings: ValidationWarning[] = [];

    if (!input.name.endsWith(".zip")) {
      errors.push({
        code: "NOT_ZIP",
        message: "File must be a .zip archive",
      });
    }

    const MAX_SIZE_BYTES = 100 * 1024 * 1024; // 100MB limit
    if (input.size > MAX_SIZE_BYTES) {
      warnings.push({
        code: "LARGE_FILE",
        message: "Zip file is larger than 100MB. Consider optimizing assets.",
        suggestion: "Compress images, remove unused files",
      });
    }

    return { errors, warnings };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Rendering
  // ═══════════════════════════════════════════════════════════════════════════

  getRendererComponent(): Type<ContentRenderer> {
    return IframeRendererComponent;
  }

  getRendererPriority(): number {
    return 15; // Medium priority - specific format
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Editing
  // ═══════════════════════════════════════════════════════════════════════════

  getEditorComponent(): Type<ContentEditorComponent> | null {
    return null; // HTML5 apps are not editable in-browser
  }

  getEditorConfig(): EditorConfig {
    return {
      ...DEFAULT_EDITOR_CONFIG,
      editorMode: "code",
      supportsLivePreview: false,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Format Detection
  // ═══════════════════════════════════════════════════════════════════════════

  detectFormat(content: string): number | null {
    try {
      const parsed: unknown = JSON.parse(content);

      if (typeof parsed === "object" && parsed !== null) {
        const obj = parsed as Record<string, unknown>;

        // Check for Html5AppContent structure
        if (
          typeof obj["slug"] === "string" &&
          typeof obj["entryPoint"] === "string"
        ) {
          return 0.9; // High confidence
        }

        // Check for ContentNode with html5-app format
        if (obj["contentFormat"] === "html5-app") {
          return 0.95;
        }
      }
    } catch {
      return null;
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Metadata
  // ═══════════════════════════════════════════════════════════════════════════

  getFormatMetadata(): FormatMetadata {
    return {
      formatId: this.formatId,
      displayName: this.displayName,
      description:
        "Interactive HTML5 applications served from Doorway. Content is extracted from zip archives and cached for fast access.",
      fileExtensions: this.fileExtensions,
      mimeTypes: this.mimeTypes,
      icon: "web",
      category: "media", // HTML5 apps are interactive media content
      supportsRoundTrip: false, // Zip content isn't editable
      priority: 15,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Private Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  private slugify(text: string): string {
    return (
      text
        .toLowerCase()
        .replaceAll(/[^a-z0-9]+/g, "-")
        // eslint-disable-next-line sonarjs/slow-regex -- Safe: slug transformation on user input
        .replaceAll(/^-+|-+$/g, "")
    );
  }

  private humanize(slug: string): string {
    return slug
      .replaceAll("-", " ")
      .replaceAll(/\b\w/g, (c) => c.toUpperCase());
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// Html5AppBody from an import document
// ═══════════════════════════════════════════════════════════════════════════

const CAPABILITY_VALUES: ReadonlySet<string> = new Set(
  FRAME_CAPABILITIES_TABLE.map((r) => r.value),
);

/** A legacy iframe sandbox token, as the capability it requests (forms, popups, downloads). */
const CAPABILITY_BY_SANDBOX_TOKEN: ReadonlyMap<string, FrameCapability> =
  new Map(
    FRAME_CAPABILITIES_TABLE.filter((r) => r.sandbox !== null).map((r) => [
      r.sandbox!,
      r.value,
    ]),
  );

function stringsOf(value: unknown): string[] {
  return Array.isArray(value)
    ? value.filter((v): v is string => typeof v === "string")
    : [];
}

function sandboxTokensOf(policy: unknown): string[] {
  return typeof policy === "object" && policy !== null
    ? stringsOf((policy as Record<string, unknown>)["sandbox"])
    : [];
}

/**
 * What an import document requests of the frame, in the capability vocabulary:
 * a declared `capabilities` list (unknown values dropped, never granted), plus
 * any legacy sandbox tokens mapped to the capability each requests. Declared
 * order, no repeats.
 */
export function declaredCapabilities(
  doc: Record<string, unknown>,
): FrameCapability[] {
  const metadata = doc["metadata"];
  const legacyTokens = [
    ...sandboxTokensOf(doc["securityPolicy"]),
    ...(typeof metadata === "object" && metadata !== null
      ? sandboxTokensOf((metadata as Record<string, unknown>)["securityPolicy"])
      : []),
  ];
  const requested = [
    ...stringsOf(doc["capabilities"]).filter((v) => CAPABILITY_VALUES.has(v)),
    ...legacyTokens
      .map((t) => CAPABILITY_BY_SANDBOX_TOKEN.get(t))
      .filter((v) => v !== undefined),
  ] as FrameCapability[];
  return [...new Set(requested)];
}

/** An import document as the record's Html5AppBody; unknown fields are not carried. */
export function toHtml5AppBody(doc: Record<string, unknown>): Html5AppBody {
  const body: Html5AppBody = {
    slug: typeof doc["slug"] === "string" ? doc["slug"] : "",
    capabilities: declaredCapabilities(doc),
  };
  if (typeof doc["entryPoint"] === "string")
    body.entryPoint = doc["entryPoint"];
  if (typeof doc["fallbackUrl"] === "string")
    body.fallbackUrl = doc["fallbackUrl"];
  if (typeof doc["presents"] === "boolean") body.presents = doc["presents"];
  return body;
}
