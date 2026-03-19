//! Weather Risk Layer Service
//!
//! Integrates Open-Meteo (free, no API key) for weather data at Places.
//! Provides risk assessment for logistics planning.
//!
//! ## External API
//!
//! Open-Meteo: `https://api.open-meteo.com/v1/forecast?latitude={lat}&longitude={lng}&hourly=...`
//! Free, no API key required. Fair-use rate limits.
//!
//! ## Caching
//!
//! Weather data is cached per Place for 1 hour to avoid hammering the external API.
//! Uses in-memory DashMap cache (operational, not persisted).

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

// ============================================================================
// Types
// ============================================================================

/// Weather risk level for logistics operations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum WeatherRisk {
    /// No significant impact on logistics
    None,
    /// Mild impact — plan adjustments may be needed
    Advisory,
    /// Significant impact — plan alternatives
    Warning,
    /// Halt operations, protect resources
    Severe,
}

/// Weather forecast for a single time point.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct WeatherDataPoint {
    /// ISO timestamp
    pub time: String,
    /// Temperature in Celsius
    pub temperature_c: f64,
    /// Precipitation in mm
    pub precipitation_mm: f64,
    /// Wind speed in km/h
    pub wind_speed_kmh: f64,
    /// WMO weather code
    pub weather_code: i32,
    /// Risk assessment
    pub risk_level: WeatherRisk,
}

/// Weather forecast for a Place.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct WeatherForecastView {
    pub place_id: String,
    pub latitude: f64,
    pub longitude: f64,
    /// Current conditions (most recent data point)
    pub current: WeatherDataPoint,
    /// Hourly forecast (next 24 hours)
    pub hourly: Vec<WeatherDataPoint>,
    /// Overall risk level (worst across forecast period)
    pub overall_risk: WeatherRisk,
    /// ISO timestamp when this forecast was fetched
    pub fetched_at: String,
}

// ============================================================================
// Open-Meteo Response Types (internal)
// ============================================================================

#[derive(Debug, Deserialize)]
struct OpenMeteoResponse {
    #[serde(default)]
    hourly: Option<OpenMeteoHourly>,
}

#[derive(Debug, Deserialize)]
struct OpenMeteoHourly {
    #[serde(default)]
    time: Vec<String>,
    #[serde(default, rename = "temperature_2m")]
    temperature_2m: Vec<f64>,
    #[serde(default)]
    precipitation: Vec<f64>,
    #[serde(default, rename = "windspeed_10m")]
    windspeed_10m: Vec<f64>,
    #[serde(default, rename = "weathercode")]
    weathercode: Vec<i32>,
}

// ============================================================================
// Weather Cache
// ============================================================================

struct CacheEntry {
    forecast: WeatherForecastView,
    inserted_at: Instant,
}

/// In-memory weather cache. Thread-safe via DashMap.
pub struct WeatherCache {
    entries: DashMap<String, CacheEntry>,
    ttl: Duration,
}

impl WeatherCache {
    pub fn new(ttl_secs: u64) -> Self {
        Self {
            entries: DashMap::new(),
            ttl: Duration::from_secs(ttl_secs),
        }
    }

    fn get(&self, place_id: &str) -> Option<WeatherForecastView> {
        let entry = self.entries.get(place_id)?;
        if entry.inserted_at.elapsed() < self.ttl {
            Some(entry.forecast.clone())
        } else {
            drop(entry);
            self.entries.remove(place_id);
            None
        }
    }

    fn insert(&self, place_id: &str, forecast: WeatherForecastView) {
        self.entries.insert(
            place_id.to_string(),
            CacheEntry {
                forecast,
                inserted_at: Instant::now(),
            },
        );
    }
}

impl Default for WeatherCache {
    fn default() -> Self {
        Self::new(3600) // 1 hour TTL
    }
}

/// Global weather cache — shared across requests.
pub fn global_cache() -> &'static Arc<WeatherCache> {
    use std::sync::OnceLock;
    static CACHE: OnceLock<Arc<WeatherCache>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(WeatherCache::default()))
}

// ============================================================================
// Core Functions
// ============================================================================

/// Get weather forecast for a Place.
///
/// Uses cache if available (1h TTL). Falls back to zero-data response
/// if Open-Meteo is unreachable.
pub async fn get_forecast(place_id: &str, latitude: f64, longitude: f64) -> WeatherForecastView {
    let cache = global_cache();

    // Check cache first
    if let Some(cached) = cache.get(place_id) {
        return cached;
    }

    // Fetch from Open-Meteo
    let forecast = match fetch_open_meteo(place_id, latitude, longitude).await {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("Open-Meteo fetch failed for place {}: {}", place_id, e);
            fallback_forecast(place_id, latitude, longitude)
        }
    };

    // Cache the result
    cache.insert(place_id, forecast.clone());
    forecast
}

/// Fetch weather data from Open-Meteo API.
async fn fetch_open_meteo(
    place_id: &str,
    latitude: f64,
    longitude: f64,
) -> Result<WeatherForecastView, String> {
    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m,precipitation,windspeed_10m,weathercode&forecast_days=1",
        latitude, longitude
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .get(&url)
        .header("User-Agent", "ElohimProtocol/1.0")
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("Open-Meteo returned status {}", resp.status()));
    }

    let data: OpenMeteoResponse = resp.json().await.map_err(|e| e.to_string())?;

    let hourly = data
        .hourly
        .ok_or_else(|| "No hourly data in response".to_string())?;

    let len = hourly.time.len();
    if len == 0 {
        return Err("Empty hourly data".to_string());
    }

    // Build data points
    let points: Vec<WeatherDataPoint> = (0..len)
        .map(|i| {
            let temp = *hourly.temperature_2m.get(i).unwrap_or(&0.0);
            let precip = *hourly.precipitation.get(i).unwrap_or(&0.0);
            let wind = *hourly.windspeed_10m.get(i).unwrap_or(&0.0);
            let code = *hourly.weathercode.get(i).unwrap_or(&0);
            let risk = assess_risk(temp, precip, wind, code);

            WeatherDataPoint {
                time: hourly.time.get(i).cloned().unwrap_or_default(),
                temperature_c: temp,
                precipitation_mm: precip,
                wind_speed_kmh: wind,
                weather_code: code,
                risk_level: risk,
            }
        })
        .collect();

    // Current = first data point
    let current = points.first().cloned().unwrap_or_else(|| WeatherDataPoint {
        time: String::new(),
        temperature_c: 0.0,
        precipitation_mm: 0.0,
        wind_speed_kmh: 0.0,
        weather_code: 0,
        risk_level: WeatherRisk::None,
    });

    // Overall risk = worst across all points
    let overall_risk = points
        .iter()
        .map(|p| risk_severity(&p.risk_level))
        .max()
        .map(severity_to_risk)
        .unwrap_or(WeatherRisk::None);

    Ok(WeatherForecastView {
        place_id: place_id.to_string(),
        latitude,
        longitude,
        current,
        hourly: points,
        overall_risk,
        fetched_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Fallback forecast when Open-Meteo is unavailable.
fn fallback_forecast(place_id: &str, latitude: f64, longitude: f64) -> WeatherForecastView {
    let now = chrono::Utc::now().to_rfc3339();
    let current = WeatherDataPoint {
        time: now.clone(),
        temperature_c: 0.0,
        precipitation_mm: 0.0,
        wind_speed_kmh: 0.0,
        weather_code: 0,
        risk_level: WeatherRisk::None,
    };

    WeatherForecastView {
        place_id: place_id.to_string(),
        latitude,
        longitude,
        current,
        hourly: vec![],
        overall_risk: WeatherRisk::None,
        fetched_at: now,
    }
}

// ============================================================================
// Risk Assessment
// ============================================================================

/// Assess weather risk from conditions.
///
/// Uses WMO weather codes + thresholds:
/// - Precipitation >10mm/h → Warning, >25mm/h → Severe
/// - Wind >50 km/h → Warning, >80 km/h → Severe
/// - Temperature <-10°C or >40°C → Warning
/// - WMO codes 95-99 (thunderstorm) → Warning/Severe
fn assess_risk(temp_c: f64, precip_mm: f64, wind_kmh: f64, weather_code: i32) -> WeatherRisk {
    // Severe conditions
    if wind_kmh > 80.0 || precip_mm > 25.0 || (95..=99).contains(&weather_code) {
        return WeatherRisk::Severe;
    }

    // Warning conditions
    if wind_kmh > 50.0 || precip_mm > 10.0 || !(-10.0..=40.0).contains(&temp_c) {
        return WeatherRisk::Warning;
    }

    // Advisory conditions
    if wind_kmh > 30.0
        || precip_mm > 2.0
        || !(0.0..=35.0).contains(&temp_c)
        || (61..=67).contains(&weather_code) // freezing rain
        || (71..=77).contains(&weather_code)
    // snow
    {
        return WeatherRisk::Advisory;
    }

    WeatherRisk::None
}

fn risk_severity(risk: &WeatherRisk) -> u8 {
    match risk {
        WeatherRisk::None => 0,
        WeatherRisk::Advisory => 1,
        WeatherRisk::Warning => 2,
        WeatherRisk::Severe => 3,
    }
}

fn severity_to_risk(severity: u8) -> WeatherRisk {
    match severity {
        0 => WeatherRisk::None,
        1 => WeatherRisk::Advisory,
        2 => WeatherRisk::Warning,
        _ => WeatherRisk::Severe,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_clear_weather() {
        assert_eq!(assess_risk(20.0, 0.0, 10.0, 0), WeatherRisk::None);
    }

    #[test]
    fn test_risk_advisory_light_rain() {
        assert_eq!(assess_risk(15.0, 3.0, 10.0, 61), WeatherRisk::Advisory);
    }

    #[test]
    fn test_risk_warning_heavy_rain() {
        assert_eq!(assess_risk(15.0, 15.0, 20.0, 65), WeatherRisk::Warning);
    }

    #[test]
    fn test_risk_severe_storm() {
        assert_eq!(assess_risk(20.0, 30.0, 90.0, 99), WeatherRisk::Severe);
    }

    #[test]
    fn test_risk_cold_warning() {
        assert_eq!(assess_risk(-15.0, 0.0, 5.0, 0), WeatherRisk::Warning);
    }

    #[test]
    fn test_cache_miss_returns_none() {
        let cache = WeatherCache::new(3600);
        assert!(cache.get("nonexistent").is_none());
    }
}
