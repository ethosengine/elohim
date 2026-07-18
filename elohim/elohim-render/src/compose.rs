//! Compose a V8-rendered SSR document with the bundle-carrying browser shell.
//!
//! The V8 render (see [`crate::angular`]) is produced against a MINIMAL
//! template — no client bundles, no `<title>` — because the runtime only needs
//! Angular to materialize the app's hydratable root-element markup, not a full
//! browsable page. Served as-is, that document can never hydrate: there is no
//! client JS to pick it up. The browser build's `index.html` (the "shell") is
//! the inverse — it carries `<title>`, `<base>`, styles, and the hashed client
//! bundle `<script>` tags, but its root element is an empty placeholder (no
//! server-rendered content, no `ngh` hydration attributes).
//!
//! [`compose_ssr_with_shell`] is the pure splice that reconciles the two: pull
//! the rendered root element (+ its `ngh` attributes, inner content, and
//! TransferState `<script id="ng-state">`) out of the V8 render, and drop it
//! into the shell in place of the empty placeholder. The result carries BOTH
//! the server-rendered content AND the client bundles, so the browser can
//! hydrate it.
//!
//! The root tag is derived from the rendered document itself — `<app-root>`,
//! `<lamad-root>`, any app's selector — never hard-coded, because every peer
//! runtime that serves apps (doorway projection, storage sidecar, steward hub)
//! serves MANY apps. A shell without the derived tag belongs to a different
//! app and is refused with a typed [`ComposeError`] so serving layers can name
//! the seam that failed.
//!
//! Dependency-free: no HTML-parser crate, just byte-index string scanning —
//! mirrors the splice style in [`crate::chrome::element::inject_element`].

/// Why a compose refused — the shared skip vocabulary every serving runtime
/// (doorway projection, storage sidecar, steward hub) reports identically, so
/// an SSR shed names its failing seam instead of hiding behind one opaque tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComposeError {
    /// The rendered document's `<body>` holds no component root element (a
    /// custom element — its tag carries a dash). Nothing to transplant.
    RenderedRootNotFound,
    /// The rendered root `<{tag}>` opens but is never closed with `</{tag}>` —
    /// a truncated or malformed render.
    RenderedRootUnclosed { tag: String },
    /// The shell has no `<{tag}>` element matching the rendered root — the
    /// render and the shell belong to DIFFERENT apps (or the shell is not this
    /// app's browser build). Splicing would serve the wrong app's markup.
    ShellRootMissing { tag: String },
}

impl ComposeError {
    /// Stable kebab-case reason string for observability surfaces
    /// (`x-ssr-skipped:` headers, log fields, counters).
    #[must_use]
    pub fn reason_str(&self) -> &'static str {
        match self {
            ComposeError::RenderedRootNotFound => "render-root-missing",
            ComposeError::RenderedRootUnclosed { .. } => "render-root-unclosed",
            ComposeError::ShellRootMissing { .. } => "shell-root-missing",
        }
    }
}

impl std::fmt::Display for ComposeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ComposeError::RenderedRootNotFound => {
                write!(f, "rendered document has no component root element")
            }
            ComposeError::RenderedRootUnclosed { tag } => {
                write!(f, "rendered root <{tag}> is never closed")
            }
            ComposeError::ShellRootMissing { tag } => {
                write!(f, "shell has no <{tag}> element matching the rendered root")
            }
        }
    }
}

impl std::error::Error for ComposeError {}

/// Compose a V8-rendered SSR document with the bundle-carrying browser shell.
///
/// The rendered doc carries the app's hydratable root element (with `ngh`
/// attributes) + its TransferState `<script id="ng-state">`, but was rendered
/// against a minimal template with NO client bundles. The shell (the browser
/// build's index.html) carries `<title>`, `<base>`, styles, and the hashed
/// client bundle `<script>` tags, but an EMPTY root placeholder. This splices
/// the rendered root chunk into the shell in place of that placeholder, so the
/// served document has BOTH the server-rendered content AND the client
/// bundles → it hydrates.
///
/// The root tag is DERIVED from the rendered document (the first component
/// element in its `<body>`), never assumed: `<app-root>`, `<lamad-root>`, any
/// app's selector composes without code changes here or in any consumer. A
/// shell that lacks the derived tag is a DIFFERENT app's shell — refused with
/// [`ComposeError::ShellRootMissing`] rather than spliced wrong.
pub fn compose_ssr_with_shell(
    rendered_html: &str,
    shell_html: &str,
) -> Result<String, ComposeError> {
    let tag = derive_rendered_root_tag(rendered_html).ok_or(ComposeError::RenderedRootNotFound)?;
    let (chunk, tag) = extract_rendered_chunk(rendered_html, tag)?;
    let (shell_root_start, shell_root_end) = find_root_element(shell_html, &tag)
        .ok_or_else(|| ComposeError::ShellRootMissing { tag: tag.clone() })?;

    let mut out =
        String::with_capacity(shell_html.len() - (shell_root_end - shell_root_start) + chunk.len());
    out.push_str(&shell_html[..shell_root_start]);
    out.push_str(chunk);
    out.push_str(&shell_html[shell_root_end..]);
    Ok(out)
}

/// Derive the app's root element tag from a rendered document: the tag name of
/// the first component element (name contains `-`, per the custom-element
/// contract Angular selectors follow) found in the document. Skips plain HTML
/// elements so wrapper markup never shadows the component root.
///
/// Returns `None` when no component element opens anywhere in the document.
fn derive_rendered_root_tag(rendered_html: &str) -> Option<String> {
    let bytes = rendered_html.as_bytes();
    let mut i = 0;
    while let Some(rel) = rendered_html[i..].find('<') {
        i += rel + 1;
        // Skip closing tags, comments, doctype, processing instructions.
        match bytes.get(i) {
            Some(b'/') | Some(b'!') | Some(b'?') | None => continue,
            _ => {}
        }
        let name_end = rendered_html[i..]
            .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
            .map(|r| i + r)?;
        let name = &rendered_html[i..name_end];
        if name.contains('-') && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return Some(name.to_ascii_lowercase());
        }
        i = name_end;
    }
    None
}

/// Locate + extract the transplant chunk from a V8-rendered SSR document: the
/// `<{tag}>…</{tag}>` element (optionally preceded by an immediately adjacent
/// `<!--nghm-->` hydration marker), plus — when immediately present — the
/// trailing TransferState `<script id="ng-state">…</script>`.
fn extract_rendered_chunk(
    rendered_html: &str,
    tag: String,
) -> Result<(&str, String), ComposeError> {
    let (root_start, root_close_end) = match find_root_element(rendered_html, &tag) {
        Some(range) => range,
        // The tag was derived from an OPEN in this same document, so absence of
        // the pair means the close tag never arrives — a truncated render.
        None => return Err(ComposeError::RenderedRootUnclosed { tag }),
    };

    // Optionally extend chunk_start LEFT over an immediately-preceding
    // `<!--nghm-->` marker (only whitespace allowed between it and the root).
    let chunk_start = extend_left_over_nghm_marker(rendered_html, root_start);

    // After the closing tag, skip whitespace + HTML comments; if a
    // `<script id="ng-state"…>` immediately follows, extend the chunk through
    // its `</script>`.
    let chunk_end = extend_right_over_ng_state_script(rendered_html, root_close_end);

    Ok((&rendered_html[chunk_start..chunk_end], tag))
}

/// Byte range `(start, end)` of the FIRST `<{tag}>` element in `html`:
/// `start` is the index of the `<{tag}` open, `end` is the index just past its
/// matching `</{tag}>` close. Tag names are matched case-sensitively lowercase
/// (Angular always emits lowercase element names), and the open match requires
/// a delimiter after the name so `<app-root` never matches `<app-root-banner`.
///
/// Returns `None` if no `<{tag}` opens, or one opens but never closes.
fn find_root_element(html: &str, tag: &str) -> Option<(usize, usize)> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");

    let mut from = 0;
    loop {
        let rel = html[from..].find(&open)?;
        let start = from + rel;
        // The char after the tag name must delimit it (whitespace, `>`, `/`) —
        // otherwise this is a longer tag sharing the prefix; keep scanning.
        match html[start + open.len()..].chars().next() {
            Some(c) if c.is_ascii_whitespace() || c == '>' || c == '/' => {
                let close_rel = html[start..].find(&close)?;
                return Some((start, start + close_rel + close.len()));
            }
            Some(_) => from = start + open.len(),
            None => return None,
        }
    }
}

/// If `html[..app_start]` ends with `<!--nghm-->` separated from `app_start`
/// only by ASCII whitespace, return the index of the marker's `<`; otherwise
/// return `app_start` unchanged.
fn extend_left_over_nghm_marker(html: &str, app_start: usize) -> usize {
    const MARKER: &str = "<!--nghm-->";
    let before = &html[..app_start];
    let trimmed_end = before
        .trim_end_matches(|c: char| c.is_ascii_whitespace())
        .len();
    if before[..trimmed_end].ends_with(MARKER) {
        trimmed_end - MARKER.len()
    } else {
        app_start
    }
}

/// Starting from `after`, skip ASCII whitespace and HTML comments
/// (`<!-- ... -->`, including the empty `<!---->` form); if a
/// `<script id="ng-state"…>` (single or double quotes, any attribute order)
/// then immediately follows, return the index just past that script's
/// `</script>`. Otherwise return `after` unchanged (no ng-state script rides
/// along).
fn extend_right_over_ng_state_script(html: &str, after: usize) -> usize {
    let mut i = after;
    let bytes = html.as_bytes();

    loop {
        // Skip ASCII whitespace.
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        // Skip an HTML comment, if one starts here.
        if html[i..].starts_with("<!--") {
            if let Some(close_rel) = html[i..].find("-->") {
                i += close_rel + "-->".len();
                continue;
            }
            // Unterminated comment: nothing sensible to extend over.
            return after;
        }
        break;
    }

    if !is_ng_state_script_open(html, i) {
        return after;
    }
    // Find this script's closing tag from the tag-open position.
    match html[i..].find("</script>") {
        Some(close_rel) => i + close_rel + "</script>".len(),
        None => after,
    }
}

/// Whether `html[at..]` begins a `<script …id="ng-state"…>` (or `'ng-state'`)
/// opening tag — i.e. `at` is the index of a `<script` whose tag carries an
/// `id` attribute (single or double quoted, any attribute position) with the
/// exact value `ng-state`, and the tag is properly closed with `>` before any
/// following `<`.
fn is_ng_state_script_open(html: &str, at: usize) -> bool {
    const OPEN_TAG: &str = "<script";
    if !html[at..].starts_with(OPEN_TAG) {
        return false;
    }
    // The opening tag ends at the first unescaped `>` after `<script`. HTML
    // attribute values in this codebase (Angular-emitted) never contain a
    // literal `>`, so a plain scan to `>` is sufficient here.
    let Some(gt_rel) = html[at..].find('>') else {
        return false;
    };
    let tag = &html[at..at + gt_rel];
    tag.contains(r#"id="ng-state""#) || tag.contains("id='ng-state'")
}

#[cfg(test)]
mod tests {
    use super::*;

    const RENDERED: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>x</style>\
</head><body><!--nghm--><app-root ng-version=\"19.2.19\" ngh=\"0\"><h1>Human flourishing</h1>\
</app-root><script id=\"ng-state\" type=\"application/json\">{\"k\":1}</script></body></html>";

    // A DIFFERENT app's artifacts — the lamad SPA renders `<lamad-root>`, and
    // its shell wraps the placeholder in protocol chrome. The compose primitive
    // must be selector-agnostic: any peer runtime (doorway, storage sidecar,
    // steward hub) serves MANY apps, each with its own root element.
    const LAMAD_RENDERED: &str = "<!DOCTYPE html><html><head><meta charset=\"utf-8\"></head>\
<body><!--nghm--><lamad-root ng-version=\"19.2.19\" ngh=\"0\"><h2>Walk the path</h2>\
</lamad-root><script id=\"ng-state\" type=\"application/json\">{\"p\":2}</script></body></html>";

    const LAMAD_SHELL: &str = "<!DOCTYPE html><html><head><base href=\"/lamad/\">\
<title>Lamad — Learning Platform</title>\
<script src=\"main-ASS77WHR.js\" type=\"module\"></script></head>\
<body><elohim-page-chrome><lamad-root></lamad-root></elohim-page-chrome></body></html>";

    #[test]
    fn composes_lamad_root_rendered_into_lamad_root_shell() {
        // The regression this file exists to prevent: compose hard-coded
        // `<app-root>`, so every non-landing app (lamad-root, any future
        // pillar app) could never SSR-compose — the manifesto page served
        // blank to crawlers. The root tag must be DERIVED from the rendered
        // document, not assumed.
        let composed = compose_ssr_with_shell(LAMAD_RENDERED, LAMAD_SHELL)
            .expect("same-app rendered+shell must compose regardless of selector");

        assert_eq!(composed.matches("<lamad-root").count(), 1, "{composed}");
        assert!(composed.contains("Walk the path"), "{composed}");
        assert!(composed.contains("ngh="), "{composed}");
        // Shell chrome + head survive.
        assert!(composed.contains("<elohim-page-chrome>"), "{composed}");
        assert!(
            composed.contains("<title>Lamad — Learning Platform</title>"),
            "{composed}"
        );
        assert!(composed.contains("main-ASS77WHR.js"), "{composed}");
        // ng-state rides along exactly once.
        assert_eq!(composed.matches("id=\"ng-state\"").count(), 1, "{composed}");
    }

    #[test]
    fn cross_app_mismatch_names_the_missing_shell_root() {
        // A landing-app render (`<app-root>`) must NOT splice into the lamad
        // shell (`<lamad-root>`) — that would serve the WRONG APP's markup.
        // The refusal must carry a typed reason naming the tag it looked for,
        // so the serving layer can tag observability (not an opaque None).
        let err = compose_ssr_with_shell(RENDERED, LAMAD_SHELL)
            .expect_err("cross-app compose must refuse");
        assert_eq!(
            err,
            ComposeError::ShellRootMissing {
                tag: "app-root".into()
            }
        );
        assert_eq!(err.reason_str(), "shell-root-missing");
    }

    #[test]
    fn rendered_without_root_component_is_typed() {
        let rendered = "<!DOCTYPE html><html><body><!-- just comments --></body></html>";
        let err = compose_ssr_with_shell(rendered, LAMAD_SHELL)
            .expect_err("no root component to transplant");
        assert_eq!(err, ComposeError::RenderedRootNotFound);
        assert_eq!(err.reason_str(), "render-root-missing");
    }

    #[test]
    fn unclosed_rendered_root_is_typed() {
        let rendered = "<html><body><app-root ngh=\"0\"><p>truncated";
        let err = compose_ssr_with_shell(rendered, SHELL).expect_err("unclosed root");
        assert_eq!(
            err,
            ComposeError::RenderedRootUnclosed {
                tag: "app-root".into()
            }
        );
        assert_eq!(err.reason_str(), "render-root-unclosed");
    }

    #[test]
    fn root_tag_prefix_does_not_false_match() {
        // `<app-root-banner>` must not satisfy a search for `<app-root>` —
        // tag-name matching requires a delimiter after the name.
        let shell = "<!DOCTYPE html><html><body><app-root-banner></app-root-banner>\
<app-root></app-root></body></html>";
        let composed =
            compose_ssr_with_shell(RENDERED, shell).expect("real app-root exists after decoy");
        assert!(composed.contains("Human flourishing"), "{composed}");
        assert!(
            composed.contains("<app-root-banner></app-root-banner>"),
            "decoy untouched: {composed}"
        );
    }

    const SHELL: &str = "<!DOCTYPE html><html><head><base href=\"/\">\
<title>elohim.host - Digital Infrastructure</title>\
<link rel=\"stylesheet\" href=\"styles-EBN7HOSH.css\">\
<script src=\"main-DJCMUMWV.js\" type=\"module\"></script></head>\
<body><app-root></app-root></body></html>";

    fn extract_head(html: &str) -> &str {
        let start = html.find("<head>").expect("has <head>");
        let end = html.find("</head>").expect("has </head>") + "</head>".len();
        &html[start..end]
    }

    #[test]
    fn composes_rendered_app_root_into_bundle_shell() {
        let composed = compose_ssr_with_shell(RENDERED, SHELL).expect("both inputs have app-root");

        // #1 risk: exactly one <app-root> in the result (a duplicate breaks
        // Angular's client bootstrap, which selects the first match).
        assert_eq!(
            composed.matches("<app-root").count(),
            1,
            "composed: {composed}"
        );

        // The rendered inner content + ngh= attribute survived (proves the
        // hydratable render, not the empty shell placeholder, won).
        assert!(composed.contains("Human flourishing"), "{composed}");
        assert!(composed.contains("ngh="), "{composed}");

        // The shell's head/scripts are untouched.
        assert!(
            composed.contains("<title>elohim.host - Digital Infrastructure</title>"),
            "{composed}"
        );
        assert!(composed.contains("main-DJCMUMWV.js"), "{composed}");
        assert!(composed.contains("<base href=\"/\">"), "{composed}");

        // ng-state appears exactly once, and is adjacent to app-root (not left
        // behind in the discarded rendered wrapper).
        assert_eq!(
            composed.matches("id=\"ng-state\"").count(),
            1,
            "composed: {composed}"
        );
        let app_close = composed.find("</app-root>").expect("has </app-root>");
        let after_close = app_close + "</app-root>".len();
        let script_open = after_close
            + composed[after_close..]
                .find("<script")
                .expect("has ng-state script");
        let between = &composed[after_close..script_open];
        assert!(
            between.trim().is_empty(),
            "ng-state script must sit directly after </app-root>, found: {between:?}"
        );
    }

    #[test]
    fn rendered_lacking_custom_element_root_is_refused() {
        // `<h1>` is a plain HTML element, not a component root (custom elements
        // carry a dash). A rendered doc whose body holds no component cannot be
        // transplanted.
        let rendered = "<!DOCTYPE html><html><body><h1>no app root here</h1></body></html>";
        assert_eq!(
            compose_ssr_with_shell(rendered, SHELL),
            Err(ComposeError::RenderedRootNotFound)
        );
    }

    #[test]
    fn shell_lacking_the_rendered_root_tag_is_refused() {
        let shell = "<!DOCTYPE html><html><body><h1>no app root here</h1></body></html>";
        assert_eq!(
            compose_ssr_with_shell(RENDERED, shell),
            Err(ComposeError::ShellRootMissing {
                tag: "app-root".into()
            })
        );
    }

    #[test]
    fn composes_without_ng_state_when_render_has_none() {
        let rendered = "<html><body><!--nghm--><app-root ngh=\"0\"><p>content</p></app-root>\
</body></html>";
        let composed = compose_ssr_with_shell(rendered, SHELL).expect("both inputs have app-root");

        assert_eq!(composed.matches("<app-root").count(), 1, "{composed}");
        assert!(composed.contains("content"), "{composed}");
        assert!(!composed.contains("ng-state"), "{composed}");
        // No dangling artifacts from the discarded rendered wrapper.
        assert!(!composed.contains("<html><body>"), "{composed}");
    }

    #[test]
    fn includes_nghm_marker_when_present() {
        let composed = compose_ssr_with_shell(RENDERED, SHELL).expect("both inputs have app-root");
        assert!(composed.contains("<!--nghm-->"), "{composed}");
        // The marker must ride in immediately before <app-root (not detached).
        let marker = composed.find("<!--nghm-->").expect("has nghm marker");
        let app_root = composed.find("<app-root").expect("has app-root");
        let between = &composed[marker + "<!--nghm-->".len()..app_root];
        assert!(
            between.trim().is_empty(),
            "nghm marker must sit directly before <app-root, found: {between:?}"
        );
    }

    #[test]
    fn shell_head_is_preserved_verbatim() {
        let expected_head = extract_head(SHELL);
        let composed = compose_ssr_with_shell(RENDERED, SHELL).expect("both inputs have app-root");
        assert_eq!(extract_head(&composed), expected_head, "{composed}");
    }

    #[test]
    fn shell_app_root_with_attributes_is_still_found() {
        // The finder must not be anchored to the exact empty `<app-root></app-root>`
        // form — an attribute-bearing shell placeholder must also be replaced.
        let shell = "<!DOCTYPE html><html><head><title>t</title></head>\
<body><app-root _ngcontent-x=\"\"></app-root></body></html>";
        let composed = compose_ssr_with_shell(RENDERED, shell).expect("both inputs have app-root");

        assert_eq!(composed.matches("<app-root").count(), 1, "{composed}");
        assert!(composed.contains("Human flourishing"), "{composed}");
        assert!(!composed.contains("_ngcontent-x"), "{composed}");
        assert!(composed.contains("<title>t</title>"), "{composed}");
    }
}
