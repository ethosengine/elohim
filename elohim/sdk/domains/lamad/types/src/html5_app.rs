//! The `html5-app` content body, and the two readings the EPR chrome derives
//! from a record without being told: its witness and its grant key.
//!
//! The body's contract is `elohim/sdk/domains/lamad/schemas/html5-app-body.schema.json`
//! (wired as the format's `bodySchema` in `manifest/content-formats.json`). The
//! record that carries it is Notarized (Category A) on the elohim DNA; the
//! integrity zome treats `contentBody` as opaque, so nothing here moves a DNA hash.
//!
//! Two ways in, deliberately different:
//! - `serde::Deserialize` on [`Html5AppBody`] is the STRICT authoring shape — an
//!   unknown capability or a mistyped field is an error, as it should be when a
//!   record is being written.
//! - [`Html5AppBody::from_json_str`] / [`Html5AppBody::from_json_value`] (feature
//!   `json`) are the LENIENT ingest path for every reader of an already-published
//!   record (doorway, storage, the chrome's producers). A malformed field falls
//!   back to its default and an unknown capability is dropped — never granted —
//!   so one odd record can neither panic a reader nor widen what a frame may do.

use serde::{Deserialize, Serialize};

pub use crate::generated_vocab::FrameCapability;

/// The entry file a bundle is entered at when its body names none.
pub const DEFAULT_ENTRY_POINT: &str = "index.html";

/// Browser-storage key prefix for a visitor's or hosted session's capability
/// grants (chrome-context schema, `$defs/grantStorageKey`).
pub const GRANT_STORAGE_KEY_PREFIX: &str = "elohim-grants:";

fn default_entry_point() -> String {
    DEFAULT_ENTRY_POINT.to_string()
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// The colour scheme a bundle asks the chrome to sit in. Absent means auto.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorScheme {
    Light,
    Dark,
}

/// `themeHint` — the ambient theme the bundle wants around it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Html5AppThemeHint {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<ColorScheme>,
    /// A CSS colour, as the entry HTML's `theme-color` would declare it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
}

/// The `contentBody` of an `html5-app` record (schema `lamad:schema:body:html5-app`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Html5AppBody {
    /// The app id: the bundle's URL namespace, `/apps/<slug>/<entryPoint>`.
    /// Distinct from the record id and from the Holochain app id. `None` only on
    /// the lenient path, when a published body lacks it — readers then fall back
    /// to the record id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slug: Option<String>,
    /// The bundle file a visit enters at, as declared (default `index.html`).
    /// Read it through [`Html5AppBody::entry_file`].
    #[serde(default = "default_entry_point")]
    pub entry_point: String,
    /// Where the same work lives outside the protocol ("open elsewhere"). Never
    /// framed in its place.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_url: Option<String>,
    /// What the frame may do beyond running its own scripts, as the record
    /// requests it, in declared order without repeats. `[]` when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<FrameCapability>,
    /// Origins (`scheme://host[:port]`) the app may reach beyond its own, as the
    /// record declares them. The chrome's web edge observes the content against
    /// this list with a Report-Only policy; it is not a grant. `[]` when none.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_hint: Option<Html5AppThemeHint>,
    /// `true` when this bundle is a wrapper that presents other content inside
    /// its own trust surface and fulfils the chrome's claims for it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub presents: bool,
}

impl Default for Html5AppBody {
    fn default() -> Self {
        Self {
            slug: None,
            entry_point: default_entry_point(),
            fallback_url: None,
            capabilities: Vec::new(),
            sources: Vec::new(),
            theme_hint: None,
            presents: false,
        }
    }
}

/// The canonical spelling of one declared origin (`scheme://host[:port]`,
/// scheme and host lowercased), or `None` when `raw` is not exactly an origin:
/// a path, query, credentials, wildcard or whitespace makes it no origin at all.
/// Mirrors the `sources` item pattern in `html5-app-body.schema.json`.
pub fn normalize_origin(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let (scheme, rest) = raw.split_once("://")?;
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(scheme.as_str(), "https" | "http" | "wss" | "ws") || rest.is_empty() {
        return None;
    }
    let (host, port) = if let Some(after) = rest.strip_prefix('[') {
        let (inner, tail) = after.split_once(']')?;
        if inner.is_empty()
            || !inner
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == ':' || c == '.')
        {
            return None;
        }
        (format!("[{}]", inner.to_ascii_lowercase()), tail)
    } else {
        let end = rest.find(':').unwrap_or(rest.len());
        let host = &rest[..end];
        let labels_ok = !host.is_empty()
            && host.split('.').all(|label| {
                !label.is_empty()
                    && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
                    && !label.starts_with('-')
                    && !label.ends_with('-')
            });
        if !labels_ok {
            return None;
        }
        (host.to_ascii_lowercase(), &rest[end..])
    };
    match port {
        "" => Some(format!("{scheme}://{host}")),
        p => {
            let digits = p.strip_prefix(':')?;
            if digits.is_empty() || digits.len() > 5 || !digits.chars().all(|c| c.is_ascii_digit())
            {
                return None;
            }
            Some(format!("{scheme}://{host}:{digits}"))
        }
    }
}

impl Html5AppBody {
    /// The entry file to serve and compare request paths against: the declared
    /// entry point without a leading `./`, or `index.html` when that leaves
    /// nothing.
    pub fn entry_file(&self) -> &str {
        let trimmed = self.entry_point.trim().trim_start_matches("./");
        if trimmed.is_empty() {
            DEFAULT_ENTRY_POINT
        } else {
            trimmed
        }
    }

    /// Whether the record declares `capability`.
    pub fn declares(&self, capability: FrameCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

#[cfg(feature = "json")]
impl Html5AppBody {
    /// Lenient parse of a `contentBody` carried as a JSON string (the wire form).
    /// Anything that is not a JSON object yields the defaults.
    pub fn from_json_str(body: &str) -> Self {
        match serde_json::from_str::<serde_json::Value>(body) {
            Ok(value) => Self::from_json_value(value),
            Err(_) => Self::default(),
        }
    }

    /// Lenient parse of a `contentBody` already in hand as a JSON value — the
    /// entry for a projection that stores the body as a document (convert BSON
    /// to `serde_json::Value` first). A JSON string VALUE is itself parsed once,
    /// so a string-wrapped body and an object body read the same. Each field is
    /// read on its own: a missing or mistyped field takes its default, blank
    /// strings count as absent, unknown and repeated capabilities are dropped.
    pub fn from_json_value(value: serde_json::Value) -> Self {
        use serde_json::Value;

        let map = match value {
            Value::Object(map) => map,
            Value::String(text) => {
                return match serde_json::from_str::<Value>(&text) {
                    Ok(Value::Object(map)) => Self::from_object(&map),
                    _ => Self::default(),
                };
            }
            _ => return Self::default(),
        };
        Self::from_object(&map)
    }

    fn from_object(map: &serde_json::Map<String, serde_json::Value>) -> Self {
        let text = |key: &str| {
            map.get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };

        let mut capabilities: Vec<FrameCapability> = Vec::new();
        if let Some(list) = map
            .get("capabilities")
            .and_then(serde_json::Value::as_array)
        {
            for item in list {
                if let Some(cap) = item
                    .as_str()
                    .map(str::trim)
                    .and_then(FrameCapability::parse)
                {
                    if !capabilities.contains(&cap) {
                        capabilities.push(cap);
                    }
                }
            }
        }

        let mut sources: Vec<String> = Vec::new();
        if let Some(list) = map.get("sources").and_then(serde_json::Value::as_array) {
            for origin in list
                .iter()
                .filter_map(serde_json::Value::as_str)
                .filter_map(normalize_origin)
            {
                if !sources.contains(&origin) {
                    sources.push(origin);
                }
            }
        }

        let theme_hint = map
            .get("themeHint")
            .and_then(serde_json::Value::as_object)
            .map(|hint| Html5AppThemeHint {
                color_scheme: match hint.get("colorScheme").and_then(serde_json::Value::as_str) {
                    Some("light") => Some(ColorScheme::Light),
                    Some("dark") => Some(ColorScheme::Dark),
                    _ => None,
                },
                theme_color: hint
                    .get("themeColor")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string),
            })
            .filter(|hint| hint.color_scheme.is_some() || hint.theme_color.is_some());

        Self {
            slug: text("slug"),
            entry_point: text("entryPoint").unwrap_or_else(default_entry_point),
            fallback_url: text("fallbackUrl"),
            capabilities,
            sources,
            theme_hint,
            presents: map
                .get("presents")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        }
    }
}

/// Whether the network has witnessed a record (chrome-context `WitnessState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Witness {
    /// Served because its bytes verified; the DHT has not yet witnessed the record.
    Amber,
    /// The DHT has validated and witnessed the record.
    Green,
}

impl Witness {
    /// The wire value (`amber` | `green`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Witness::Amber => "amber",
            Witness::Green => "green",
        }
    }
}

/// The one witness rule: green iff the record's projection carries a non-blank
/// `dht_anchor_hash`, else amber. Derived on every read, never written — so a
/// reader that re-asks after the DHT witnesses the record sees amber turn green.
pub fn witness_from_anchor(dht_anchor_hash: Option<&str>) -> Witness {
    match dht_anchor_hash {
        Some(anchor) if !anchor.trim().is_empty() => Witness::Green,
        _ => Witness::Amber,
    }
}

/// The browser-storage key a visitor's or hosted session's grants for this
/// bundle live under: `elohim-grants:<contentAddress>`. Keyed by content
/// address, so a new bundle (a new declared set) starts from its record's
/// declaration again.
pub fn grant_storage_key(content_address: &str) -> String {
    format!("{GRANT_STORAGE_KEY_PREFIX}{content_address}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn witness_is_green_only_with_a_non_blank_anchor() {
        assert_eq!(witness_from_anchor(Some("uhCkkabc")), Witness::Green);
        assert_eq!(witness_from_anchor(Some("")), Witness::Amber);
        assert_eq!(witness_from_anchor(Some("   ")), Witness::Amber);
        assert_eq!(witness_from_anchor(None), Witness::Amber);
        assert_eq!(Witness::Green.as_str(), "green");
        assert_eq!(Witness::Amber.as_str(), "amber");
    }

    #[test]
    fn grant_key_is_prefixed_content_address() {
        assert_eq!(grant_storage_key("sha256-abc"), "elohim-grants:sha256-abc");
    }

    #[test]
    fn entry_file_trims_dot_slash_and_defaults() {
        let mut body = Html5AppBody::default();
        assert_eq!(body.entry_file(), "index.html");
        body.entry_point = "./game.html".into();
        assert_eq!(body.entry_file(), "game.html");
        body.entry_point = "./".into();
        assert_eq!(body.entry_file(), "index.html");
    }

    #[test]
    fn strict_serde_round_trips_and_rejects_unknown_capability() {
        let body = Html5AppBody {
            slug: Some("evolution-of-trust".into()),
            capabilities: vec![FrameCapability::Forms, FrameCapability::ClipboardWrite],
            theme_hint: Some(Html5AppThemeHint {
                color_scheme: Some(ColorScheme::Dark),
                theme_color: None,
            }),
            ..Default::default()
        };
        let bytes = rmp_serde::to_vec_named(&body).unwrap();
        let back: Html5AppBody = rmp_serde::from_slice(&bytes).unwrap();
        assert_eq!(back, body);

        // Authoring is strict: a capability outside the vocabulary is an error.
        let bad = rmp_serde::to_vec_named(&serde_json_free_map(&[("capabilities", &["teleport"])]))
            .unwrap();
        assert!(rmp_serde::from_slice::<Html5AppBody>(&bad).is_err());
    }

    /// A tiny map value without serde_json (which is optional), for the strict test.
    fn serde_json_free_map(
        entries: &[(&str, &[&str])],
    ) -> std::collections::BTreeMap<String, Vec<String>> {
        entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.iter().map(|s| s.to_string()).collect()))
            .collect()
    }

    #[test]
    fn frame_capability_annotations_match_the_chrome_vocabulary() {
        use FrameCapability as C;
        assert_eq!(C::Forms.sandbox_token(), Some("allow-forms"));
        assert_eq!(C::Popups.sandbox_token(), Some("allow-popups"));
        assert_eq!(C::Downloads.sandbox_token(), Some("allow-downloads"));
        assert_eq!(C::Fullscreen.allow_feature(), Some("fullscreen"));
        assert_eq!(C::ClipboardWrite.allow_feature(), Some("clipboard-write"));
        for cap in C::ALL {
            // Exactly one of the two attribute channels carries each capability.
            assert!(
                cap.sandbox_token().is_some() ^ cap.allow_feature().is_some(),
                "{cap}"
            );
            assert_eq!(C::parse(cap.as_str()), Some(*cap));
        }
        let device: Vec<_> = C::ALL.iter().filter(|c| c.is_device()).collect();
        assert_eq!(device, vec![&C::Camera, &C::Microphone, &C::Geolocation]);
        assert_eq!(C::parse("javascript"), None);
    }
}

#[cfg(all(test, feature = "json"))]
mod json_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn string_and_object_bodies_read_the_same() {
        let object = json!({
            "slug": "evolution-of-trust",
            "entryPoint": "index.html",
            "fallbackUrl": "https://ncase.me/trust/",
            "capabilities": ["forms", "fullscreen"],
            "themeHint": { "colorScheme": "dark", "themeColor": "#111" },
            "presents": false
        });
        let from_value = Html5AppBody::from_json_value(object.clone());
        let from_str = Html5AppBody::from_json_str(&object.to_string());
        let from_wrapped = Html5AppBody::from_json_value(json!(object.to_string()));
        assert_eq!(from_value, from_str);
        assert_eq!(from_value, from_wrapped);
        assert_eq!(from_value.slug.as_deref(), Some("evolution-of-trust"));
        assert_eq!(
            from_value.capabilities,
            vec![FrameCapability::Forms, FrameCapability::Fullscreen]
        );
        assert_eq!(
            from_value.theme_hint,
            Some(Html5AppThemeHint {
                color_scheme: Some(ColorScheme::Dark),
                theme_color: Some("#111".into()),
            })
        );
    }

    #[test]
    fn today_seed_shape_parses_with_defaults() {
        // The shape every html5-app seed carried before capabilities moved here.
        let body = Html5AppBody::from_json_str(r#"{"slug":"quiz-1","entryPoint":"./game.html"}"#);
        assert_eq!(body.slug.as_deref(), Some("quiz-1"));
        assert_eq!(body.entry_file(), "game.html");
        assert!(body.capabilities.is_empty());
        assert!(!body.presents);
        assert!(body.theme_hint.is_none());
    }

    #[test]
    fn malformed_bodies_fall_back_to_defaults_and_never_panic() {
        for raw in [
            "",
            "not json",
            "[]",
            "null",
            "42",
            r#""just a string""#,
            r#"{"slug":7,"entryPoint":["x"],"capabilities":"forms","presents":"yes","themeHint":3}"#,
        ] {
            let body = Html5AppBody::from_json_str(raw);
            assert_eq!(body.slug, None, "{raw}");
            assert_eq!(body.entry_file(), "index.html", "{raw}");
            assert!(body.capabilities.is_empty(), "{raw}");
            assert!(!body.presents, "{raw}");
            assert!(body.theme_hint.is_none(), "{raw}");
        }
        assert_eq!(
            Html5AppBody::from_json_value(json!(null)),
            Html5AppBody::default()
        );
        assert_eq!(
            Html5AppBody::from_json_value(json!(false)),
            Html5AppBody::default()
        );
    }

    #[test]
    fn unknown_blank_and_repeated_capabilities_are_dropped_in_order() {
        let body = Html5AppBody::from_json_value(json!({
            "slug": "x",
            "capabilities": ["popups", "javascript", "", 5, "forms", "popups", " camera "]
        }));
        assert_eq!(
            body.capabilities,
            vec![
                FrameCapability::Popups,
                FrameCapability::Forms,
                FrameCapability::Camera
            ]
        );
        assert!(body.declares(FrameCapability::Camera));
        assert!(!body.declares(FrameCapability::Downloads));
    }

    #[test]
    fn sources_keep_only_origins_normalized_and_deduplicated() {
        let body = Html5AppBody::from_json_value(json!({
            "slug": "x",
            "sources": [
                "https://API.Example.org",
                "https://api.example.org",
                "wss://live.example.org:8443",
                "http://[::1]:8090",
                "https://api.example.org/path",
                "https://*.example.org",
                "ftp://files.example.org",
                "https://evil.example; script-src *",
                "",
                7
            ]
        }));
        assert_eq!(
            body.sources,
            vec![
                "https://api.example.org",
                "wss://live.example.org:8443",
                "http://[::1]:8090"
            ]
        );
        assert!(Html5AppBody::from_json_str(r#"{"slug":"x"}"#)
            .sources
            .is_empty());
        assert_eq!(normalize_origin("https://a.b:99999999"), None);
        assert_eq!(normalize_origin("https://-a.b"), None);
    }

    #[test]
    fn blank_strings_count_as_absent() {
        let body = Html5AppBody::from_json_value(json!({
            "slug": "  ",
            "entryPoint": "",
            "fallbackUrl": " ",
            "themeHint": { "colorScheme": "sepia", "themeColor": "" }
        }));
        assert_eq!(body, Html5AppBody::default());
    }

    #[test]
    fn rust_shape_matches_the_body_schema() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../schemas/html5-app-body.schema.json")).unwrap();
        let mut schema_props: Vec<String> = schema["properties"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        schema_props.sort();
        let full = Html5AppBody {
            slug: Some("s".into()),
            entry_point: "e.html".into(),
            fallback_url: Some("https://example.org/".into()),
            capabilities: vec![FrameCapability::Forms],
            sources: vec!["https://api.example.org".into()],
            theme_hint: Some(Html5AppThemeHint {
                color_scheme: Some(ColorScheme::Light),
                theme_color: Some("#fff".into()),
            }),
            presents: true,
        };
        let wire = serde_json::to_value(&full).unwrap();
        let mut rust_props: Vec<String> = wire.as_object().unwrap().keys().cloned().collect();
        rust_props.sort();
        assert_eq!(rust_props, schema_props);
        let hint_props: Vec<&String> = schema["$defs"]["Html5AppThemeHint"]["properties"]
            .as_object()
            .unwrap()
            .keys()
            .collect();
        for key in hint_props {
            assert!(wire["themeHint"].get(key).is_some(), "themeHint.{key}");
        }
        assert_eq!(
            schema["properties"]["entryPoint"]["default"],
            DEFAULT_ENTRY_POINT
        );
    }

    #[test]
    fn vocabulary_matches_the_schemas() {
        let caps: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../schemas/v1/enums/frame-capability.schema.json"
        ))
        .unwrap();
        let wire: Vec<&str> = FrameCapability::ALL.iter().map(|c| c.as_str()).collect();
        let schema: Vec<&str> = caps["enum"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(wire, schema);
        let table: serde_json::Value =
            serde_json::from_str(crate::generated_vocab::FRAME_CAPABILITIES_TABLE_JSON).unwrap();
        assert_eq!(table.as_array().unwrap().len(), schema.len());

        let ctx: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../schemas/v1/views/chrome-context.schema.json"
        ))
        .unwrap();
        let witness = &ctx["$defs"]["contentIsland"]["properties"]["witness"]["enum"];
        assert_eq!(witness, &serde_json::json!(["amber", "green"]));
        let key = ctx["$defs"]["grantStorageKey"]["pattern"].as_str().unwrap();
        assert_eq!(key, format!("^{GRANT_STORAGE_KEY_PREFIX}.+$"));
    }
}
