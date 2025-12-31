//! Streaming Endpoints for HLS/DASH Adaptive Streaming
//!
//! Provides endpoints for serving adaptive bitrate streaming content:
//! - **HLS (HTTP Live Streaming)**: Apple's streaming protocol using m3u8 manifests
//! - **DASH (Dynamic Adaptive Streaming over HTTP)**: MPEG-DASH using MPD manifests
//!
//! ## Endpoints
//!
//! | Endpoint | Description |
//! |----------|-------------|
//! | `GET /api/stream/hls/{content_id}` | HLS master playlist |
//! | `GET /api/stream/hls/{content_id}/{variant}` | HLS variant playlist |
//! | `GET /api/stream/dash/{content_id}` | DASH MPD manifest |
//! | `GET /api/stream/chunk/{hash}/{index}` | Individual chunk |
//!
//! ## Integration
//!
//! Works with:
//! - Tiered blob cache for chunk storage/retrieval
//! - Custodian service for fallback URL selection
//! - Blob metadata for variant information

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use std::sync::Arc;

use crate::cache::{BlobMetadata, VariantMetadata};
use crate::server::AppState;
use crate::services::CustodianSelectionCriteria;

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract streaming metadata from ProjectionStore
///
/// Attempts to get content metadata from ProjectionStore and convert it
/// to BlobMetadata for HLS/DASH playlist generation.
async fn get_streaming_metadata(state: &AppState, content_id: &str) -> Option<BlobMetadata> {
    // Get projection store
    let projection = state.projection.as_ref()?;

    // Look up the content document (Content type with content_id as doc_id)
    let doc = projection.get("Content", content_id).await?;

    // Try to deserialize BlobMetadata from the document's data field
    // The content's data should contain streaming metadata
    serde_json::from_value::<BlobMetadata>(doc.data.clone()).ok()
}

// ============================================================================
// Constants
// ============================================================================

/// Default HLS segment duration in seconds
const DEFAULT_SEGMENT_DURATION: u32 = 6;

/// Default target duration for HLS playlists
const DEFAULT_TARGET_DURATION: u32 = 7;

// ============================================================================
// Streaming Response Handlers
// ============================================================================

/// Handle HLS master playlist request
///
/// Returns an m3u8 master playlist listing all available quality variants.
pub async fn handle_hls_master(
    state: Arc<AppState>,
    content_id: &str,
    base_url: &str,
) -> Response<Full<Bytes>> {
    // Get streaming metadata from ProjectionStore
    let metadata = match get_streaming_metadata(&state, content_id).await {
        Some(m) => m,
        None => {
            return error_response(StatusCode::NOT_FOUND, "Content not found");
        }
    };

    // Generate master playlist
    let playlist = generate_hls_master(&metadata, content_id, base_url);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/vnd.apple.mpegurl")
        .header("Cache-Control", "max-age=60")
        .body(Full::new(Bytes::from(playlist)))
        .unwrap()
}

/// Handle HLS variant playlist request
///
/// Returns an m3u8 playlist for a specific quality variant.
pub async fn handle_hls_variant(
    state: Arc<AppState>,
    content_id: &str,
    variant_label: &str,
    base_url: &str,
) -> Response<Full<Bytes>> {
    // Get streaming metadata from ProjectionStore
    let metadata = match get_streaming_metadata(&state, content_id).await {
        Some(m) => m,
        None => {
            return error_response(StatusCode::NOT_FOUND, "Content not found");
        }
    };

    // Find the variant
    let variant = match metadata.get_variant(variant_label) {
        Some(v) => v,
        None => {
            return error_response(StatusCode::NOT_FOUND, "Variant not found");
        }
    };

    // Generate variant playlist
    let duration = metadata.duration_seconds.unwrap_or(0);
    let playlist = generate_hls_variant(
        &variant.hash,
        duration,
        DEFAULT_SEGMENT_DURATION,
        base_url,
        state.tiered_cache.config().default_chunk_size,
    );

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/vnd.apple.mpegurl")
        .header("Cache-Control", "max-age=60")
        .body(Full::new(Bytes::from(playlist)))
        .unwrap()
}

/// Handle DASH MPD manifest request
///
/// Returns an MPD (Media Presentation Description) manifest for DASH players.
pub async fn handle_dash_mpd(
    state: Arc<AppState>,
    content_id: &str,
    base_url: &str,
) -> Response<Full<Bytes>> {
    // Get streaming metadata from ProjectionStore
    let metadata = match get_streaming_metadata(&state, content_id).await {
        Some(m) => m,
        None => {
            return error_response(StatusCode::NOT_FOUND, "Content not found");
        }
    };

    // Generate DASH manifest
    let duration = metadata.duration_seconds.unwrap_or(0);
    let mpd = generate_dash_mpd(&metadata, content_id, duration, base_url);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/dash+xml")
        .header("Cache-Control", "max-age=60")
        .body(Full::new(Bytes::from(mpd)))
        .unwrap()
}

/// Handle chunk request
///
/// Returns a chunk from the tiered cache or fetches from custodian.
pub async fn handle_chunk(
    state: Arc<AppState>,
    blob_hash: &str,
    chunk_index: usize,
) -> Response<Full<Bytes>> {
    // Try to get chunk from cache
    if let Some(chunk_data) = state.tiered_cache.get_chunk(blob_hash, chunk_index) {
        return Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "video/mp2t")
            .header("Cache-Control", "max-age=86400")
            .body(Full::new(Bytes::from(chunk_data)))
            .unwrap();
    }

    // Chunk not in cache - try to get from custodian
    let criteria = CustodianSelectionCriteria::default();
    let urls = state.custodian.get_custodian_urls(blob_hash, &criteria);

    if urls.is_empty() {
        return error_response(StatusCode::NOT_FOUND, "Chunk not available");
    }

    // For now, return a redirect to the first custodian URL
    // In production, we would fetch and cache the chunk
    let chunk_url = format!("{}/chunk/{}", urls[0].trim_end_matches(&format!("/{}", blob_hash)), chunk_index);

    Response::builder()
        .status(StatusCode::TEMPORARY_REDIRECT)
        .header("Location", &chunk_url)
        .header("Cache-Control", "max-age=60")
        .body(Full::new(Bytes::new()))
        .unwrap()
}

/// Handle blob byte range request
///
/// Returns a byte range from the blob for resumable downloads.
pub async fn handle_blob_range(
    state: Arc<AppState>,
    blob_hash: &str,
    range_start: usize,
    range_end: usize,
) -> Response<Full<Bytes>> {
    // Try to get range from tiered cache
    if let Some(data) = state.tiered_cache.get_blob_range(blob_hash, range_start, range_end) {
        let content_range = format!("bytes {}-{}/{}", range_start, range_end - 1, range_end - range_start);
        return Response::builder()
            .status(StatusCode::PARTIAL_CONTENT)
            .header("Content-Type", "application/octet-stream")
            .header("Content-Range", content_range)
            .header("Accept-Ranges", "bytes")
            .header("Cache-Control", "max-age=86400")
            .body(Full::new(data))
            .unwrap();
    }

    error_response(StatusCode::NOT_FOUND, "Blob range not available")
}

// ============================================================================
// Playlist Generation
// ============================================================================

/// Generate HLS master playlist with all variants
///
/// # Arguments
/// * `metadata` - Blob metadata with variant information
/// * `content_id` - Content identifier
/// * `base_url` - Base URL for variant playlists
pub fn generate_hls_master(metadata: &BlobMetadata, content_id: &str, base_url: &str) -> String {
    let mut playlist = String::new();

    // Header
    playlist.push_str("#EXTM3U\n");
    playlist.push_str("#EXT-X-VERSION:3\n");

    // Add each variant
    for variant in &metadata.variants {
        let bandwidth = (variant.bitrate_mbps * 1_000_000.0) as u64;
        let resolution = match (variant.width, variant.height) {
            (Some(w), Some(h)) => format!(",RESOLUTION={}x{}", w, h),
            _ => String::new(),
        };

        playlist.push_str(&format!(
            "#EXT-X-STREAM-INF:BANDWIDTH={}{}\n",
            bandwidth, resolution
        ));
        playlist.push_str(&format!(
            "{}/api/stream/hls/{}/{}.m3u8\n",
            base_url, content_id, variant.label
        ));
    }

    // If no variants, add a single stream for the main blob
    if metadata.variants.is_empty() {
        let bandwidth = ((metadata.bitrate_mbps.unwrap_or(5.0)) * 1_000_000.0) as u64;
        playlist.push_str(&format!("#EXT-X-STREAM-INF:BANDWIDTH={}\n", bandwidth));
        playlist.push_str(&format!(
            "{}/api/stream/hls/{}/default.m3u8\n",
            base_url, content_id
        ));
    }

    playlist
}

/// Generate HLS variant playlist with segments
///
/// # Arguments
/// * `blob_hash` - Hash of the blob for this variant
/// * `duration_secs` - Total duration in seconds
/// * `segment_duration` - Target segment duration
/// * `base_url` - Base URL for chunk endpoints
/// * `chunk_size` - Size of each chunk in bytes
pub fn generate_hls_variant(
    blob_hash: &str,
    duration_secs: u32,
    segment_duration: u32,
    base_url: &str,
    chunk_size: usize,
) -> String {
    let mut playlist = String::new();

    // Header
    playlist.push_str("#EXTM3U\n");
    playlist.push_str("#EXT-X-VERSION:3\n");
    playlist.push_str(&format!("#EXT-X-TARGETDURATION:{}\n", DEFAULT_TARGET_DURATION));
    playlist.push_str("#EXT-X-MEDIA-SEQUENCE:0\n");

    // Calculate number of segments
    let num_segments = if duration_secs > 0 {
        (duration_secs + segment_duration - 1) / segment_duration
    } else {
        1
    };

    // Add each segment
    for i in 0..num_segments {
        let seg_duration = if i == num_segments - 1 && duration_secs > 0 {
            // Last segment might be shorter
            let remaining = duration_secs % segment_duration;
            if remaining > 0 {
                remaining as f32
            } else {
                segment_duration as f32
            }
        } else {
            segment_duration as f32
        };

        playlist.push_str(&format!("#EXTINF:{:.3},\n", seg_duration));
        playlist.push_str(&format!(
            "{}/api/stream/chunk/{}/{}\n",
            base_url, blob_hash, i
        ));
    }

    // End marker
    playlist.push_str("#EXT-X-ENDLIST\n");

    playlist
}

/// Generate DASH MPD manifest
///
/// # Arguments
/// * `metadata` - Blob metadata with variant information
/// * `content_id` - Content identifier
/// * `duration_secs` - Total duration in seconds
/// * `base_url` - Base URL for segment endpoints
pub fn generate_dash_mpd(
    metadata: &BlobMetadata,
    content_id: &str,
    duration_secs: u32,
    base_url: &str,
) -> String {
    let duration_iso = format_iso_duration(duration_secs);

    let mut mpd = String::new();

    // XML header
    mpd.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    mpd.push('\n');

    // MPD element
    mpd.push_str(&format!(
        r#"<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" mediaPresentationDuration="{}" minBufferTime="PT2S" profiles="urn:mpeg:dash:profile:isoff-on-demand:2011">"#,
        duration_iso
    ));
    mpd.push('\n');

    // Period
    mpd.push_str(&format!(r#"  <Period duration="{}">"#, duration_iso));
    mpd.push('\n');

    // Video AdaptationSet
    mpd.push_str(r#"    <AdaptationSet mimeType="video/mp4" segmentAlignment="true">"#);
    mpd.push('\n');

    // Add each variant as a Representation
    for (i, variant) in metadata.variants.iter().enumerate() {
        let bandwidth = (variant.bitrate_mbps * 1_000_000.0) as u64;
        let width = variant.width.unwrap_or(1920);
        let height = variant.height.unwrap_or(1080);

        mpd.push_str(&format!(
            r#"      <Representation id="{}" bandwidth="{}" width="{}" height="{}" codecs="avc1.640028">"#,
            i, bandwidth, width, height
        ));
        mpd.push('\n');

        // SegmentTemplate
        mpd.push_str(&format!(
            r#"        <BaseURL>{}/api/stream/chunk/{}/</BaseURL>"#,
            base_url, variant.hash
        ));
        mpd.push('\n');
        mpd.push_str(&format!(
            r#"        <SegmentTemplate media="$Number$" startNumber="0" duration="{}" timescale="1"/>"#,
            DEFAULT_SEGMENT_DURATION
        ));
        mpd.push('\n');

        mpd.push_str("      </Representation>\n");
    }

    // If no variants, add default representation
    if metadata.variants.is_empty() {
        let bandwidth = ((metadata.bitrate_mbps.unwrap_or(5.0)) * 1_000_000.0) as u64;

        mpd.push_str(&format!(
            r#"      <Representation id="0" bandwidth="{}" codecs="avc1.640028">"#,
            bandwidth
        ));
        mpd.push('\n');
        mpd.push_str(&format!(
            r#"        <BaseURL>{}/api/stream/chunk/{}/</BaseURL>"#,
            base_url, metadata.hash
        ));
        mpd.push('\n');
        mpd.push_str(&format!(
            r#"        <SegmentTemplate media="$Number$" startNumber="0" duration="{}" timescale="1"/>"#,
            DEFAULT_SEGMENT_DURATION
        ));
        mpd.push('\n');
        mpd.push_str("      </Representation>\n");
    }

    mpd.push_str("    </AdaptationSet>\n");
    mpd.push_str("  </Period>\n");
    mpd.push_str("</MPD>\n");

    mpd
}

/// Format duration as ISO 8601 duration string (e.g., "PT1H30M45S")
fn format_iso_duration(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    let mut duration = String::from("PT");
    if hours > 0 {
        duration.push_str(&format!("{}H", hours));
    }
    if minutes > 0 {
        duration.push_str(&format!("{}M", minutes));
    }
    duration.push_str(&format!("{}S", secs));

    duration
}

/// Create an error response
fn error_response(status: StatusCode, message: &str) -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": message,
        "status": status.as_u16()
    });

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

// ============================================================================
// Route Handler
// ============================================================================

/// Parse streaming route and dispatch to appropriate handler
pub async fn handle_stream_request(
    state: Arc<AppState>,
    path: &str,
    base_url: &str,
) -> Response<Full<Bytes>> {
    // Parse the path: /api/stream/{type}/{content_id}/{optional_variant}
    let parts: Vec<&str> = path
        .strip_prefix("/api/stream/")
        .unwrap_or("")
        .split('/')
        .collect();

    match parts.as_slice() {
        // HLS master playlist: /api/stream/hls/{content_id}
        ["hls", content_id] => {
            handle_hls_master(state, content_id, base_url).await
        }

        // HLS variant playlist: /api/stream/hls/{content_id}/{variant}.m3u8
        ["hls", content_id, variant] => {
            let variant_label = variant.strip_suffix(".m3u8").unwrap_or(variant);
            handle_hls_variant(state, content_id, variant_label, base_url).await
        }

        // DASH manifest: /api/stream/dash/{content_id}
        ["dash", content_id] => {
            handle_dash_mpd(state, content_id, base_url).await
        }

        // Chunk: /api/stream/chunk/{hash}/{index}
        ["chunk", hash, index] => {
            match index.parse::<usize>() {
                Ok(idx) => handle_chunk(state, hash, idx).await,
                Err(_) => error_response(StatusCode::BAD_REQUEST, "Invalid chunk index"),
            }
        }

        _ => error_response(StatusCode::NOT_FOUND, "Unknown streaming endpoint"),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_metadata() -> BlobMetadata {
        BlobMetadata {
            hash: "test_hash_abc123".to_string(),
            size_bytes: 100 * 1024 * 1024, // 100 MB
            mime_type: "video/mp4".to_string(),
            codec: Some("H.264".to_string()),
            bitrate_mbps: Some(5.0),
            duration_seconds: Some(120), // 2 minutes
            reach: "commons".to_string(),
            fallback_urls: vec!["https://example.com/blob/abc".to_string()],
            variants: vec![
                VariantMetadata {
                    label: "1080p".to_string(),
                    hash: "hash_1080p".to_string(),
                    bitrate_mbps: 5.0,
                    width: Some(1920),
                    height: Some(1080),
                    size_bytes: 100 * 1024 * 1024,
                    fallback_urls: vec![],
                },
                VariantMetadata {
                    label: "720p".to_string(),
                    hash: "hash_720p".to_string(),
                    bitrate_mbps: 2.5,
                    width: Some(1280),
                    height: Some(720),
                    size_bytes: 50 * 1024 * 1024,
                    fallback_urls: vec![],
                },
            ],
            captions: vec![],
            created_at: "2024-01-01T00:00:00Z".to_string(),
            verified_at: None,
            author_id: None,
            content_id: Some("content123".to_string()),
        }
    }

    #[test]
    fn test_generate_hls_master() {
        let metadata = test_metadata();
        let playlist = generate_hls_master(&metadata, "content123", "https://api.example.com");

        assert!(playlist.contains("#EXTM3U"));
        assert!(playlist.contains("#EXT-X-STREAM-INF:BANDWIDTH=5000000"));
        assert!(playlist.contains("RESOLUTION=1920x1080"));
        assert!(playlist.contains("/api/stream/hls/content123/1080p.m3u8"));
        assert!(playlist.contains("/api/stream/hls/content123/720p.m3u8"));
    }

    #[test]
    fn test_generate_hls_variant() {
        let playlist = generate_hls_variant(
            "hash_1080p",
            120, // 2 minutes
            6,   // 6 second segments
            "https://api.example.com",
            5 * 1024 * 1024, // 5 MB chunks
        );

        assert!(playlist.contains("#EXTM3U"));
        assert!(playlist.contains("#EXT-X-TARGETDURATION:7"));
        assert!(playlist.contains("#EXTINF:"));
        assert!(playlist.contains("/api/stream/chunk/hash_1080p/"));
        assert!(playlist.contains("#EXT-X-ENDLIST"));
    }

    #[test]
    fn test_generate_dash_mpd() {
        let metadata = test_metadata();
        let mpd = generate_dash_mpd(&metadata, "content123", 120, "https://api.example.com");

        assert!(mpd.contains(r#"<?xml version="1.0""#));
        assert!(mpd.contains("MPD"));
        assert!(mpd.contains("PT2M0S")); // 2 minutes
        assert!(mpd.contains(r#"bandwidth="5000000""#));
        assert!(mpd.contains(r#"width="1920""#));
        assert!(mpd.contains("/api/stream/chunk/hash_1080p/"));
    }

    #[test]
    fn test_format_iso_duration() {
        assert_eq!(format_iso_duration(0), "PT0S");
        assert_eq!(format_iso_duration(45), "PT45S");
        assert_eq!(format_iso_duration(120), "PT2M0S");
        assert_eq!(format_iso_duration(3661), "PT1H1M1S");
    }
}
