---
id: "backlog-security-markdown-renderer-raw-html-bypasses-sanitizer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "lamad's markdown renderer passes author HTML and unescaped link/image attributes through bypassSecurityTrustHtml into the shell document — stored script execution for any markdown content author"
slug: "security-markdown-renderer-raw-html-bypasses-sanitizer"
written: "2026-10-29"
author: "claude (surfaced by a Codex gpt-6-astra adversarial review of the html5-app iframe entry; chain re-read on disk)"
status: "open"
priority: "high"
area: "app/lamad"
domain: "protocol"
jobs: [elohim-app]
relatedNodeIds:
  - "concern:doorway-portal-login"
tags: [security, xss, markdown, sanitizer-bypass, session-token, signing-credentials, lamad]
---

# Markdown content runs script in the shell itself

**One line.** lamad renders markdown ContentNodes with `marked` and hands the result to
`bypassSecurityTrustHtml`, then binds it with `[innerHTML]`. Nothing sanitizes it. Any author whose
markdown reaches a viewer can run script **in the shell document**, with no frame and no sandbox. That
means everything the [html5-app iframe entry](epr:security-html5-app-iframe-same-origin-reads-session-token)
describes (session token renewable by `/auth/refresh`, stored signing credentials, the shell DOM),
reached more directly, through the most common content format in the corpus.

## The chain (read on disk, 2026-10-29)

| # | Link | Where |
|---|---|---|
| 1 | `marked` is configured with `gfm: true` and syntax highlighting, and **no `html` renderer override**. Raw HTML in the markdown source passes through to the output, which is `marked`'s documented default | `app/lamad/src/app/renderers/markdown-renderer/markdown-renderer.component.ts:186-201` |
| 2 | The custom `image` renderer interpolates `token.text` into `alt="…"` and `token.title` into `title="…"` **without escaping**. The `link` renderer interpolates `href`, `title` and `text` the same way, and does not restrict the `href` scheme | same file `:205-229` |
| 3 | Post-processing (image URL rewrite, heading IDs, leading-title drop) transforms the string without sanitizing it | same file `:316-328` |
| 4 | `this.renderedContent = this.sanitizer.bypassSecurityTrustHtml(processedHtml)`, with the comment *"HTML is generated from trusted markdown content stored in backend"* | same file `:329-332` |
| 5 | Bound as `<div [innerHTML]="renderedContent">` inside the shell's own document | same file `:108` |

`<script>` elements inserted through `innerHTML` do not execute, but event-handler attributes do. The
following all reach the shell's origin:

- `<img src=x onerror=…>` in raw HTML;
- a markdown image whose alt text closes the attribute (`![x" onerror="…](y)`, via link 2);
- `[click](javascript:…)`, through the unrestricted link `href`.

## Why the "trusted content" comment does not hold

The comment treats *stored in backend* as *trusted*. In this protocol content is authored by many
people. Reach governs **who sees** a node, not **what the node may do** to the person who sees it. A
CID proves which bytes these are, not that their author should run code as the viewer. The same
false premise sits behind the html5-app iframe flags.

## Scope

- **Exposed:** every signed-in viewer of any markdown ContentNode rendered by this component in the
  browser app. Markdown is lamad's default text format, so this is broader than html5-app.
- **Not conflated:** ordinary Angular `[innerHTML]` bindings are sanitized by Angular. Only the bypass
  makes this a sink. For example, content-viewer's fallback for `contentFormat === 'html'` binds
  `[innerHTML]="node.content"` *without* a bypass
  (`app/lamad/src/app/components/content-viewer/content-viewer.component.html:293-296`), so Angular
  strips script-bearing attributes there.
- **Sweep (done 2026-10-29):** this is the only `bypassSecurityTrustHtml` call site in non-spec code
  under `app/` (`grep -rn bypassSecurityTrustHtml app --include=*.ts`). The resource-URL bypasses
  belong to the iframe entry.

## Fix

1. **Sanitize after rendering, before binding.** Run the final HTML through an allow-list sanitizer
   (DOMPurify, or Angular's own sanitizer by dropping the bypass), keeping the attributes the renderer
   needs: `data-epr` on links, `class`, heading `id`s, and `img src` limited to blob/doorway URLs.
2. **Escape inside the custom renderers** (`alt`, `title`, link text) and restrict `href` to `http(s):`,
   `mailto:`, relative paths, and the `epr:` interception the renderer already handles.
3. **Decide raw HTML deliberately.** Either strip it (marked's `html` renderer returns an escaped
   string) or admit an explicit allow-list. Seeded content that relies on raw HTML (the `<img src="/blob/...">`
   patterns `transformHtmlImageUrls` handles, `:387`) must keep working.
4. Remove the "trusted content" comment, and add an `.epr-meta` inject rule on this file: a sanitizer
   bypass here needs a stated reason (RS-GOV-04's TypeScript analogue).

## Acceptance

1. A fixture markdown node carrying each of the three payloads (raw `onerror`, alt-text breakout,
   `javascript:` link) renders **inert**, checked eyes-on in a real browser (`pnpm look`), with a
   positive control showing the session is live.
2. The existing corpus renders unchanged in a diff of rendered output over the seeded markdown nodes:
   blob images, `epr:` links, code highlighting, the heading TOC.
3. A lint rail or `.epr-meta` rule keeps the count of HTML sanitizer bypasses in `app/` at zero unless a
   reviewed reason accompanies each one.

Source: the Codex adversarial review recorded in the iframe entry's author line. Prior art for the
general rule (an app or content author never acts as the viewer): the
[Nostr Compass survey](epr:nostr-compass-ecosystem-cross-pollination-2026-10-29), N7.
