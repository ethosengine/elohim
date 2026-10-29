//! AUTO-GENERATED from protocol vocabulary schemas carrying a `_table` marker.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/schemas/v1/enums/*.schema.json (`_table`, no `_dna`)

use serde::{Deserialize, Serialize};

/// FrameCapability — What content framed by the EPR chrome may be given beyond running its own scripts.
///
/// Generated from `epr:schema:enum:frame-capability`; wire values are the schema's kebab-case strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum FrameCapability {
    #[serde(rename = "forms")]
    Forms,
    #[serde(rename = "popups")]
    Popups,
    #[serde(rename = "downloads")]
    Downloads,
    #[serde(rename = "fullscreen")]
    Fullscreen,
    #[serde(rename = "clipboard-write")]
    ClipboardWrite,
    #[serde(rename = "camera")]
    Camera,
    #[serde(rename = "microphone")]
    Microphone,
    #[serde(rename = "geolocation")]
    Geolocation,
}

impl FrameCapability {
    /// Every value, in schema order.
    pub const ALL: &'static [FrameCapability] = &[
        FrameCapability::Forms,
        FrameCapability::Popups,
        FrameCapability::Downloads,
        FrameCapability::Fullscreen,
        FrameCapability::ClipboardWrite,
        FrameCapability::Camera,
        FrameCapability::Microphone,
        FrameCapability::Geolocation,
    ];

    /// The wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            FrameCapability::Forms => "forms",
            FrameCapability::Popups => "popups",
            FrameCapability::Downloads => "downloads",
            FrameCapability::Fullscreen => "fullscreen",
            FrameCapability::ClipboardWrite => "clipboard-write",
            FrameCapability::Camera => "camera",
            FrameCapability::Microphone => "microphone",
            FrameCapability::Geolocation => "geolocation",
        }
    }

    /// Parse a wire value; `None` for anything outside the vocabulary.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "forms" => Some(FrameCapability::Forms),
            "popups" => Some(FrameCapability::Popups),
            "downloads" => Some(FrameCapability::Downloads),
            "fullscreen" => Some(FrameCapability::Fullscreen),
            "clipboard-write" => Some(FrameCapability::ClipboardWrite),
            "camera" => Some(FrameCapability::Camera),
            "microphone" => Some(FrameCapability::Microphone),
            "geolocation" => Some(FrameCapability::Geolocation),
            _ => None,
        }
    }

    /// The value's `x-label` annotation.
    pub const fn label(self) -> &'static str {
        match self {
            FrameCapability::Forms => "Submit forms",
            FrameCapability::Popups => "Open windows",
            FrameCapability::Downloads => "Download files",
            FrameCapability::Fullscreen => "Go fullscreen",
            FrameCapability::ClipboardWrite => "Copy to your clipboard",
            FrameCapability::Camera => "Camera",
            FrameCapability::Microphone => "Microphone",
            FrameCapability::Geolocation => "Location",
        }
    }

    /// The value's `x-sandbox` annotation.
    pub const fn sandbox_token(self) -> Option<&'static str> {
        match self {
            FrameCapability::Forms => Some("allow-forms"),
            FrameCapability::Popups => Some("allow-popups"),
            FrameCapability::Downloads => Some("allow-downloads"),
            FrameCapability::Fullscreen => None,
            FrameCapability::ClipboardWrite => None,
            FrameCapability::Camera => None,
            FrameCapability::Microphone => None,
            FrameCapability::Geolocation => None,
        }
    }

    /// The value's `x-allow` annotation.
    pub const fn allow_feature(self) -> Option<&'static str> {
        match self {
            FrameCapability::Forms => None,
            FrameCapability::Popups => None,
            FrameCapability::Downloads => None,
            FrameCapability::Fullscreen => Some("fullscreen"),
            FrameCapability::ClipboardWrite => Some("clipboard-write"),
            FrameCapability::Camera => Some("camera"),
            FrameCapability::Microphone => Some("microphone"),
            FrameCapability::Geolocation => Some("geolocation"),
        }
    }

    /// The value's `x-device` annotation.
    pub const fn is_device(self) -> bool {
        match self {
            FrameCapability::Forms => false,
            FrameCapability::Popups => false,
            FrameCapability::Downloads => false,
            FrameCapability::Fullscreen => false,
            FrameCapability::ClipboardWrite => false,
            FrameCapability::Camera => true,
            FrameCapability::Microphone => true,
            FrameCapability::Geolocation => true,
        }
    }
}

impl std::fmt::Display for FrameCapability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The FrameCapability table as compact JSON, one object per value in schema order,
/// keyed exactly like the TS `FRAME_CAPABILITIES_TABLE` rows — for a script that cannot
/// import TS (e.g. the chrome element, which splices it in at serve time).
pub const FRAME_CAPABILITIES_TABLE_JSON: &str = r#"[{"value":"forms","label":"Submit forms","sandbox":"allow-forms","allow":null,"device":false},{"value":"popups","label":"Open windows","sandbox":"allow-popups","allow":null,"device":false},{"value":"downloads","label":"Download files","sandbox":"allow-downloads","allow":null,"device":false},{"value":"fullscreen","label":"Go fullscreen","sandbox":null,"allow":"fullscreen","device":false},{"value":"clipboard-write","label":"Copy to your clipboard","sandbox":null,"allow":"clipboard-write","device":false},{"value":"camera","label":"Camera","sandbox":null,"allow":"camera","device":true},{"value":"microphone","label":"Microphone","sandbox":null,"allow":"microphone","device":true},{"value":"geolocation","label":"Location","sandbox":null,"allow":"geolocation","device":true}]"#;

/// PageInfoSection — The sections of a record's page info, in display order.
///
/// Generated from `epr:schema:enum:page-info-section`; wire values are the schema's kebab-case strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum PageInfoSection {
    #[serde(rename = "source")]
    Source,
    #[serde(rename = "delivery")]
    Delivery,
    #[serde(rename = "holders")]
    Holders,
    #[serde(rename = "steward")]
    Steward,
    #[serde(rename = "claims")]
    Claims,
    #[serde(rename = "permissions")]
    Permissions,
    #[serde(rename = "yours")]
    Yours,
}

impl PageInfoSection {
    /// Every value, in schema order.
    pub const ALL: &'static [PageInfoSection] = &[
        PageInfoSection::Source,
        PageInfoSection::Delivery,
        PageInfoSection::Holders,
        PageInfoSection::Steward,
        PageInfoSection::Claims,
        PageInfoSection::Permissions,
        PageInfoSection::Yours,
    ];

    /// The wire value.
    pub const fn as_str(self) -> &'static str {
        match self {
            PageInfoSection::Source => "source",
            PageInfoSection::Delivery => "delivery",
            PageInfoSection::Holders => "holders",
            PageInfoSection::Steward => "steward",
            PageInfoSection::Claims => "claims",
            PageInfoSection::Permissions => "permissions",
            PageInfoSection::Yours => "yours",
        }
    }

    /// Parse a wire value; `None` for anything outside the vocabulary.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "source" => Some(PageInfoSection::Source),
            "delivery" => Some(PageInfoSection::Delivery),
            "holders" => Some(PageInfoSection::Holders),
            "steward" => Some(PageInfoSection::Steward),
            "claims" => Some(PageInfoSection::Claims),
            "permissions" => Some(PageInfoSection::Permissions),
            "yours" => Some(PageInfoSection::Yours),
            _ => None,
        }
    }

    /// The value's `x-label` annotation.
    pub const fn label(self) -> &'static str {
        match self {
            PageInfoSection::Source => "Source",
            PageInfoSection::Delivery => "Delivery",
            PageInfoSection::Holders => "Holders",
            PageInfoSection::Steward => "Steward",
            PageInfoSection::Claims => "Claims",
            PageInfoSection::Permissions => "Permissions",
            PageInfoSection::Yours => "Yours",
        }
    }
}

impl std::fmt::Display for PageInfoSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The PageInfoSection table as compact JSON, one object per value in schema order,
/// keyed exactly like the TS `PAGE_INFO_SECTIONS_TABLE` rows — for a script that cannot
/// import TS (e.g. the chrome element, which splices it in at serve time).
pub const PAGE_INFO_SECTIONS_TABLE_JSON: &str = r#"[{"value":"source","label":"Source"},{"value":"delivery","label":"Delivery"},{"value":"holders","label":"Holders"},{"value":"steward","label":"Steward"},{"value":"claims","label":"Claims"},{"value":"permissions","label":"Permissions"},{"value":"yours","label":"Yours"}]"#;
