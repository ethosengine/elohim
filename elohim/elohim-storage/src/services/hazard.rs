//! Hazard domain types and weather-hazard auto-detection
//!
//! Defines the controlled vocabularies for hazard classification and
//! provides the auto-detection bridge from weather forecasts to hazard records.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::services::weather::{WeatherForecastView, WeatherRisk};

// ============================================================================
// Controlled Vocabularies
// ============================================================================

/// Hazard type classification
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum HazardType {
    Flood,
    Drought,
    Wildfire,
    Storm,
    Earthquake,
    SupplyDisruption,
    InfrastructureFailure,
    Epidemic,
    Custom,
}

/// Hazard severity classification
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum HazardSeverity {
    Watch,
    Advisory,
    Warning,
    Emergency,
}

/// Hazard data source
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum HazardSource {
    WeatherApi,
    ManualReport,
    CapacityBreach,
    DistributionGap,
    External,
}

// ============================================================================
// Weather-to-Hazard Bridge
// ============================================================================

/// Scan a weather forecast and return (hazard_type, severity, title) tuples
/// for conditions that warrant creating a hazard record.
///
/// The caller is responsible for persisting the records via the hazard DB layer.
/// Only Warning or Severe overall risk triggers auto-detection — Advisory and
/// None are informational and do not warrant a hazard record.
pub fn auto_detect_weather_hazards(
    place_id: &str,
    forecast: &WeatherForecastView,
) -> Vec<(String, String, String)> {
    let _ = place_id; // used by caller for context, not needed for detection logic

    match forecast.overall_risk {
        WeatherRisk::Warning => {
            // Inspect the forecast for the dominant condition
            let hazard_type = dominant_weather_hazard_type(forecast);
            vec![(
                hazard_type.clone(),
                "warning".to_string(),
                format!(
                    "Weather {} detected — significant operational impact expected",
                    hazard_type
                ),
            )]
        }
        WeatherRisk::Severe => {
            let hazard_type = dominant_weather_hazard_type(forecast);
            vec![(
                hazard_type.clone(),
                "emergency".to_string(),
                format!(
                    "Severe weather {} — halt operations, protect resources",
                    hazard_type
                ),
            )]
        }
        // Advisory and None do not generate hazard records
        WeatherRisk::Advisory | WeatherRisk::None => vec![],
    }
}

/// Determine the dominant hazard type from forecast data points.
/// Uses the worst hourly data point to classify: high precipitation → flood/storm,
/// high wind → storm, otherwise generic storm.
fn dominant_weather_hazard_type(forecast: &WeatherForecastView) -> String {
    let max_precip = forecast
        .hourly
        .iter()
        .map(|p| p.precipitation_mm)
        .fold(0.0_f64, f64::max);

    let max_wind = forecast
        .hourly
        .iter()
        .map(|p| p.wind_speed_kmh)
        .fold(0.0_f64, f64::max);

    if max_precip > 50.0 {
        "flood".to_string()
    } else {
        "storm".to_string()
    }
}
