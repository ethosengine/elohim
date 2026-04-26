//! Local role marker — answers "is this elohim-agent configured as a defender
//! for the given human?" Hydrated from the DefenderManifest at startup.

use crate::manifest::DefenderManifest;

pub struct DefenderRoleMarker {
    manifest: DefenderManifest,
}

impl DefenderRoleMarker {
    pub fn new(manifest: DefenderManifest) -> Self {
        Self { manifest }
    }

    pub fn is_defender_for(&self, human_action_hash_b64: &str) -> bool {
        self.manifest
            .for_humans
            .iter()
            .any(|h| h == human_action_hash_b64)
    }
}
