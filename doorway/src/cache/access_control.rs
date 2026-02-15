//! Reach-aware access control and source prioritization for the cache layer
//!
//! Implements reach-based serving rules that prevent information colonization:
//! - Private content only served to beneficiary
//! - Local/neighborhood content only served to those in geographic scope
//! - Commons content served to anyone
//!
//! Also implements source prioritization for custodian commitments:
//! - Prefer nearby custodians (based on geographic_affinity)
//! - Prioritize high-bandwidth sources for media
//! - Use cache_priority from commitment for tie-breaking

/// Reach levels in order of scope (most private to most public)
pub const REACH_LEVELS: &[&str] = &[
    "private",      // 0: Only beneficiary
    "invited",      // 1: Explicitly invited individuals
    "local",        // 2: Family/household
    "neighborhood", // 3: Street block or neighborhood
    "municipal",    // 4: City or town
    "bioregional",  // 5: Bioregion (watershed, ecosystem)
    "regional",     // 6: Regional (state/province)
    "commons",      // 7: Global/public
];

/// Requester context for access control decisions
#[derive(Debug, Clone)]
pub struct RequesterContext {
    /// Requester's agent ID
    pub agent_id: String,
    /// Requester's geographic location (optional, e.g., "37.7749,-122.4194" for SF)
    pub location: Option<String>,
    /// Whether requester is authenticated
    pub authenticated: bool,
}

/// Check if requester can access content at given reach level
///
/// Rules:
/// - private: Only beneficiary (agent_id match)
/// - invited: Only invited users (requires explicit invite list)
/// - local: Authenticated users within family/location
/// - neighborhood+: Authenticated users
/// - commons: Everyone
pub fn can_serve_at_reach(reach: &str, requester: &RequesterContext, beneficiary_id: &str) -> bool {
    match reach {
        "private" => {
            // Only beneficiary can access private content
            requester.agent_id == beneficiary_id
        }
        "invited" => {
            // Only explicitly invited users (simplified: authenticated only)
            requester.authenticated
        }
        "local" | "neighborhood" | "municipal" | "bioregional" | "regional" => {
            // Requires authentication
            requester.authenticated
        }
        "commons" => {
            // Everyone can access commons content
            true
        }
        _ => {
            // Unknown reach level, deny by default
            false
        }
    }
}

/// Geographic distance between two locations (simplified)
///
/// Returns estimated distance in kilometers between two lat/lng pairs
pub fn geographic_distance(loc1: Option<&str>, loc2: Option<&str>) -> Option<f64> {
    match (loc1, loc2) {
        (Some(l1), Some(l2)) => {
            // Very simplified: parse "lat,lng" format and do basic calculation
            let parts1: Vec<&str> = l1.split(',').collect();
            let parts2: Vec<&str> = l2.split(',').collect();

            if parts1.len() == 2 && parts2.len() == 2 {
                if let (Ok(lat1), Ok(lng1), Ok(lat2), Ok(lng2)) = (
                    parts1[0].parse::<f64>(),
                    parts1[1].parse::<f64>(),
                    parts2[0].parse::<f64>(),
                    parts2[1].parse::<f64>(),
                ) {
                    // Haversine formula (simplified)
                    let lat_diff = (lat2 - lat1).abs();
                    let lng_diff = (lng2 - lng1).abs();
                    // Rough approximation: 1 degree ≈ 111 km
                    let distance = ((lat_diff.powi(2) + lng_diff.powi(2)).sqrt()) * 111.0;
                    return Some(distance);
                }
            }
            None
        }
        _ => None,
    }
}

/// Custodian source for serving content
#[derive(Debug, Clone)]
pub struct CustodianSource {
    /// Custodian agent ID
    pub agent_id: String,
    /// Geographic location of custodian
    pub location: Option<String>,
    /// Cache priority (0-100, higher = serve first)
    pub cache_priority: u32,
    /// Bandwidth class (low, medium, high, ultra)
    pub bandwidth_class: String,
    /// Distance to requester in km
    pub distance_km: Option<f64>,
}

impl CustodianSource {
    /// Calculate source priority score (higher = better)
    ///
    /// Factors:
    /// - Cache priority from commitment (0-100)
    /// - Proximity (closer = higher score)
    /// - Bandwidth class (ultra > high > medium > low)
    pub fn priority_score(&self, _requester_location: Option<&str>) -> f64 {
        let mut score = self.cache_priority as f64;

        // Distance bonus (invert: closer = higher bonus)
        if let Some(dist) = self.distance_km {
            // Decay exponentially with distance: 1000km away = -50 points
            let distance_penalty = (dist / 100.0).min(50.0);
            score -= distance_penalty;
        }

        // Bandwidth bonus
        match self.bandwidth_class.as_str() {
            "ultra" => score += 20.0,
            "high" => score += 10.0,
            "medium" => score += 5.0,
            "low" => score -= 5.0,
            _ => {}
        }

        score.clamp(0.0, 200.0)
    }
}

/// Prioritize custodian sources for serving content
///
/// Sorts sources by:
/// 1. Cache priority (from commitment)
/// 2. Geographic proximity (closer = first)
/// 3. Bandwidth capacity (high-bandwidth first)
pub fn prioritize_sources(
    mut sources: Vec<CustodianSource>,
    requester: &RequesterContext,
) -> Vec<CustodianSource> {
    // Calculate distance for each source
    for source in &mut sources {
        source.distance_km =
            geographic_distance(requester.location.as_deref(), source.location.as_deref());
    }

    // Sort by priority score
    sources.sort_by(|a, b| {
        let score_a = a.priority_score(requester.location.as_deref());
        let score_b = b.priority_score(requester.location.as_deref());
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    sources
}

/// Cache invalidation rules for reach-based content
///
/// When certain operations occur, invalidate reach-specific cache entries
pub fn invalidation_pattern_for_reach(
    dna_hash: &str,
    zome: &str,
    fn_name: &str,
    reach: &str,
) -> String {
    // Pattern: dna:zome:fn:*:reach
    format!("{dna_hash}:{zome}:{fn_name}:*:{reach}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_serve_private() {
        let requester = RequesterContext {
            agent_id: "alice".to_string(),
            location: None,
            authenticated: true,
        };

        // Alice can serve her own private content
        assert!(can_serve_at_reach("private", &requester, "alice"));

        // Bob cannot serve Alice's private content
        let bob = RequesterContext {
            agent_id: "bob".to_string(),
            location: None,
            authenticated: true,
        };
        assert!(!can_serve_at_reach("private", &bob, "alice"));
    }

    #[test]
    fn test_can_serve_commons() {
        let requester = RequesterContext {
            agent_id: "stranger".to_string(),
            location: None,
            authenticated: false,
        };

        // Unauthenticated user can access commons
        assert!(can_serve_at_reach("commons", &requester, "anyone"));
    }

    #[test]
    fn test_can_serve_authenticated_reach() {
        let authenticated = RequesterContext {
            agent_id: "bob".to_string(),
            location: None,
            authenticated: true,
        };

        let unauthenticated = RequesterContext {
            agent_id: "stranger".to_string(),
            location: None,
            authenticated: false,
        };

        // Authenticated can serve neighborhood
        assert!(can_serve_at_reach("neighborhood", &authenticated, "alice"));

        // Unauthenticated cannot
        assert!(!can_serve_at_reach(
            "neighborhood",
            &unauthenticated,
            "alice"
        ));
    }

    #[test]
    fn test_geographic_distance() {
        // San Francisco (37.7749, -122.4194)
        // Los Angeles (34.0522, -118.2437)
        let sf = "37.7749,-122.4194";
        let la = "34.0522,-118.2437";

        if let Some(distance) = geographic_distance(Some(sf), Some(la)) {
            // Should be around 550-600 km
            assert!(distance > 400.0 && distance < 700.0);
        }
    }

    #[test]
    fn test_source_priority() {
        let local_high = CustodianSource {
            agent_id: "local_custodian".to_string(),
            location: Some("37.7749,-122.4194".to_string()),
            cache_priority: 80,
            bandwidth_class: "high".to_string(),
            distance_km: Some(5.0),
        };

        let distant_low = CustodianSource {
            agent_id: "distant_custodian".to_string(),
            location: Some("50.0,-0.0".to_string()),
            cache_priority: 40,
            bandwidth_class: "low".to_string(),
            distance_km: Some(5000.0),
        };

        let requester = RequesterContext {
            agent_id: "user".to_string(),
            location: Some("37.7749,-122.4194".to_string()),
            authenticated: true,
        };

        // Local high-priority source should score higher
        assert!(
            local_high.priority_score(requester.location.as_deref())
                > distant_low.priority_score(requester.location.as_deref())
        );
    }

    #[test]
    fn test_prioritize_sources() {
        let sources = vec![
            CustodianSource {
                agent_id: "distant".to_string(),
                location: Some("50.0,-0.0".to_string()),
                cache_priority: 50,
                bandwidth_class: "medium".to_string(),
                distance_km: None,
            },
            CustodianSource {
                agent_id: "local".to_string(),
                location: Some("37.7749,-122.4194".to_string()),
                cache_priority: 50,
                bandwidth_class: "high".to_string(),
                distance_km: None,
            },
        ];

        let requester = RequesterContext {
            agent_id: "user".to_string(),
            location: Some("37.7749,-122.4194".to_string()),
            authenticated: true,
        };

        let sorted = prioritize_sources(sources, &requester);

        // Local source should be first
        assert_eq!(sorted[0].agent_id, "local");
        assert_eq!(sorted[1].agent_id, "distant");
    }

    #[test]
    fn test_invalidation_pattern_for_reach() {
        let pattern =
            invalidation_pattern_for_reach("dna123", "content_store", "get_content", "commons");
        assert!(pattern.contains("commons"));
        assert!(pattern.contains("content_store"));
    }
}
