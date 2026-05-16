//! AUTO-GENERATED from pillar manifests' attestations + governance-actions sections.
//! DO NOT EDIT — regenerate with: pnpm run schema:codegen:rs
//!
//! Source: elohim/sdk/domains/{imagodei,lamad,infrastructure,mishpat}/manifest.json

/// Every attestation subtype declared across pillar manifests. Sorted alphabetically.
pub const ATTESTATION_KINDS: &[&str] = &[
    "attestation:challenge-support",
    "attestation:content-quality",
    "attestation:content-succession",
    "attestation:custodian-commitment",
    "attestation:device-health",
    "attestation:doorway-health-summary",
    "attestation:forget-decision",
    "attestation:gate-decision",
    "attestation:governance-reaction",
    "attestation:governance-role",
    "attestation:humanness",
    "attestation:identity-credential",
    "attestation:identity-freeze",
    "attestation:key-stewardship",
    "attestation:mastery",
    "attestation:policy-inheritance",
    "attestation:proposal-vote",
    "attestation:recovery-approval",
    "attestation:renewal-approval",
    "attestation:revocation-vote",
    "attestation:statement-vote",
    "attestation:stewardship-appeal",
    "attestation:stewardship-grant",
];

/// Every governance-action kind declared across pillar manifests. Sorted alphabetically.
pub const GOVERNANCE_ACTION_KINDS: &[&str] = &[
    "governance-action:challenge",
    "governance-action:election",
    "governance-action:identity-challenge",
    "governance-action:identity-freeze",
    "governance-action:key-revocation",
    "governance-action:proposal",
    "governance-action:recovery-request",
    "governance-action:renewal-request",
    "governance-action:shamir-custody-setup",
];

/// Maps an attestation subtype to the pillar manifest that declares it.
pub fn manifest_ref_for_attestation_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "attestation:challenge-support" => Some("imagodei"),
        "attestation:content-quality" => Some("lamad"),
        "attestation:content-succession" => Some("lamad"),
        "attestation:custodian-commitment" => Some("lamad"),
        "attestation:device-health" => Some("infrastructure"),
        "attestation:doorway-health-summary" => Some("infrastructure"),
        "attestation:forget-decision" => Some("mishpat"),
        "attestation:gate-decision" => Some("mishpat"),
        "attestation:governance-reaction" => Some("mishpat"),
        "attestation:governance-role" => Some("mishpat"),
        "attestation:humanness" => Some("imagodei"),
        "attestation:identity-credential" => Some("imagodei"),
        "attestation:identity-freeze" => Some("imagodei"),
        "attestation:key-stewardship" => Some("imagodei"),
        "attestation:mastery" => Some("lamad"),
        "attestation:policy-inheritance" => Some("imagodei"),
        "attestation:proposal-vote" => Some("mishpat"),
        "attestation:recovery-approval" => Some("imagodei"),
        "attestation:renewal-approval" => Some("imagodei"),
        "attestation:revocation-vote" => Some("imagodei"),
        "attestation:statement-vote" => Some("mishpat"),
        "attestation:stewardship-appeal" => Some("imagodei"),
        "attestation:stewardship-grant" => Some("imagodei"),
        _ => None,
    }
}

/// Maps a governance-action kind to the pillar manifest that declares it.
pub fn manifest_ref_for_governance_action_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "governance-action:challenge" => Some("mishpat"),
        "governance-action:election" => Some("mishpat"),
        "governance-action:identity-challenge" => Some("imagodei"),
        "governance-action:identity-freeze" => Some("imagodei"),
        "governance-action:key-revocation" => Some("imagodei"),
        "governance-action:proposal" => Some("mishpat"),
        "governance-action:recovery-request" => Some("imagodei"),
        "governance-action:renewal-request" => Some("imagodei"),
        "governance-action:shamir-custody-setup" => Some("imagodei"),
        _ => None,
    }
}
