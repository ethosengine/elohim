import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  Input,
  OnChanges,
  SimpleChanges,
} from '@angular/core';

import { eprToUniversalHref, parseEpr } from '@elohim/service';

import type { EprCompositeBody, Item, Section } from '../../generated/body-types';
import { ContentNode } from '../../models/content-node.model';

/**
 * View-model row for a single outline item — pairs the parsed body Item with
 * its minted universal address. Purely display-scoped: the href is derived
 * from the item ref via the EPR minter, never a route literal.
 */
interface OutlineItem {
  ref: string;
  role?: Item['role'];
  title?: string;
  narrative?: string;
  /** Universal EPR address (/epr/{id}) minted from the item ref. */
  href: string;
}

/**
 * View-model row for a section — its display fields plus its outline items.
 */
interface OutlineSection {
  id: string;
  title?: string;
  description?: string;
  estimatedDuration?: string;
  items: OutlineItem[];
}

/**
 * PathViewerComponent — renders a `contentFormat: 'epr-composite'` node AS the
 * focal lens of an EPR atom: a clean, accessible OUTLINE of the path.
 *
 * This is NOT the full path player (progress / mastery / step navigation) —
 * that lives at the pillar mount (lamad/path/:id). Here the composite is
 * shown as an atom would be seen at its universal address: a legible table of
 * contents whose items link to their own universal addresses.
 *
 * Body shape comes from the lamad manifest's `epr-composite-body` schema
 * (generated `EprCompositeBody`). `node.content` arrives as a JSON string; we
 * parse it tolerantly and fall back to a graceful "no sections yet" note on
 * empty / absent / malformed bodies — never a crash, never a raw JSON dump.
 *
 * Item links are minted via `eprToUniversalHref(parseEpr(ref))` — `parseEpr`
 * normalizes both `epr:concept-id` and bare `concept-id` refs to a head-tier
 * EprRef, yielding the universal address. Route literals are forbidden here
 * (the bundle route-literal linter enforces the minter).
 */
@Component({
  selector: 'app-path-viewer',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './path-viewer.component.html',
  styleUrls: ['./path-viewer.component.css'],
})
export class PathViewerComponent implements OnChanges {
  @Input() node!: ContentNode;

  /** Parsed, display-ready sections. Empty when the body is absent/invalid. */
  sections: OutlineSection[] = [];

  /** True when there is nothing to render (graceful empty state). */
  isEmpty = true;

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['node']) {
      this.buildOutline();
    }
  }

  private buildOutline(): void {
    const body = this.parseBody(this.node?.content);
    const rawSections = body?.sections ?? [];

    this.sections = rawSections
      .filter((section): section is Section => !!section && typeof section.id === 'string')
      .map(section => this.toOutlineSection(section));

    // Empty when no section carries displayable content (a section is only
    // meaningful here if it has a heading or at least one linkable item).
    this.isEmpty = !this.sections.some(s => !!s.title || s.items.length > 0);
  }

  /**
   * Tolerantly parse the epr-composite body. `node.content` is a JSON string
   * per the wire contract, but we accept an already-parsed object too (some
   * upstream adapters hydrate it). Any parse failure → undefined (graceful).
   */
  private parseBody(content: string | object | undefined): EprCompositeBody | undefined {
    if (!content) return undefined;

    if (typeof content === 'object') {
      return content as EprCompositeBody;
    }

    try {
      const parsed = JSON.parse(content) as unknown;
      if (parsed && typeof parsed === 'object') {
        return parsed as EprCompositeBody;
      }
      return undefined;
    } catch {
      return undefined;
    }
  }

  private toOutlineSection(section: Section): OutlineSection {
    const items = (section.items ?? [])
      .filter((item): item is Item => !!item && typeof item.ref === 'string')
      .map(item => this.toOutlineItem(item));

    return {
      id: section.id,
      title: section.title,
      description: section.description,
      estimatedDuration: section.estimatedDuration,
      items,
    };
  }

  private toOutlineItem(item: Item): OutlineItem {
    // parseEpr normalizes `epr:`-prefixed and bare refs to a head-tier EprRef;
    // the minter then produces the universal /epr/{id} address.
    const href = eprToUniversalHref(parseEpr(item.ref));
    return {
      ref: item.ref,
      role: item.role,
      title: item.title,
      narrative: item.narrative,
      href,
    };
  }
}
