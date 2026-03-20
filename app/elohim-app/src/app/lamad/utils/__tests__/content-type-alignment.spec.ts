/**
 * Content Type Alignment Tests
 *
 * Enforces that all content type lists stay in sync:
 * - Wire types (generated from healing.rs)
 * - ALL_CONTENT_TYPES (wire + app extensions)
 * - Icon maps (must cover every type)
 * - Resource explorer folders (must cover every wire type)
 *
 * When a new type is added to healing.rs and regenerated,
 * these tests fail with clear pointers to what needs updating.
 */
import { describe, it, expect } from 'vitest';

import { CONTENT_TYPES, CONTENT_FORMATS, CORE_CONTENT_TYPES } from '@app/generated/schema-enums';
import { ALL_CONTENT_TYPES, ALL_CONTENT_FORMATS } from '@app/lamad/models/content-node.model';
import { CONTENT_TYPE_FOLDERS } from '@app/shefa/services/resource-explorer.service';

import { CONTENT_TYPE_ICONS, CONTENT_FORMAT_ICONS } from '../content-icons';

describe('Content Type Alignment', () => {
  it('every wire content type appears in ALL_CONTENT_TYPES', () => {
    const allTypes = ALL_CONTENT_TYPES as readonly string[];
    for (const ct of CONTENT_TYPES) {
      expect(allTypes, `wire type '${ct}' missing from ALL_CONTENT_TYPES`).toContain(ct);
    }
  });

  it('every ContentType has an icon', () => {
    for (const ct of ALL_CONTENT_TYPES) {
      expect(
        CONTENT_TYPE_ICONS[ct as keyof typeof CONTENT_TYPE_ICONS],
        `missing icon for content type '${ct}'`
      ).toBeDefined();
    }
  });

  it('every core content type maps to at least one resource explorer folder', () => {
    const allFolderTypes = CONTENT_TYPE_FOLDERS.flatMap(f => f.types) as string[];
    for (const ct of CORE_CONTENT_TYPES) {
      expect(allFolderTypes, `core type '${ct}' not in any CONTENT_TYPE_FOLDERS`).toContain(ct);
    }
  });
});

describe('Content Format Alignment', () => {
  it('every wire content format appears in ALL_CONTENT_FORMATS', () => {
    const allFormats = ALL_CONTENT_FORMATS as readonly string[];
    for (const cf of CONTENT_FORMATS) {
      expect(allFormats, `wire format '${cf}' missing from ALL_CONTENT_FORMATS`).toContain(cf);
    }
  });

  it('every ContentFormat has an icon', () => {
    for (const cf of ALL_CONTENT_FORMATS) {
      expect(
        CONTENT_FORMAT_ICONS[cf as keyof typeof CONTENT_FORMAT_ICONS],
        `missing icon for content format '${cf}'`
      ).toBeDefined();
    }
  });
});
