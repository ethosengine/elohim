//! WebRTC Media Session Types
//!
//! Extends the SBD signaling protocol with structured media session management.
//! While the existing message forwarding works for WebRTC signaling, these types
//! provide:
//! - Session lifecycle tracking
//! - Structured SDP offer/answer exchange
//! - ICE candidate batching
//! - Media type negotiation
//!
//! ## Media Flow
//!
//! 1. Caller sends MediaOffer with SDP and session ID
//! 2. Callee responds with MediaAnswer
//! 3. Both exchange IceCandidate messages
//! 4. Either party sends MediaEnd to terminate
//!
//! ## Wire Format
//!
//! Media commands use JSON-encoded payloads with a type field:
//! ```json
//! { "type": "media_offer", "session_id": "...", "sdp": "...", ... }
//! ```

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

// ============================================================================
// Media Types
// ============================================================================

/// Media types available in a session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    /// Audio only (voice call)
    Audio,
    /// Video (includes audio)
    Video,
    /// Screen sharing
    Screen,
    /// Data channel only (no A/V)
    Data,
}

impl MediaType {
    /// Check if this media type includes audio
    pub fn has_audio(&self) -> bool {
        matches!(self, MediaType::Audio | MediaType::Video)
    }

    /// Check if this media type includes video
    pub fn has_video(&self) -> bool {
        matches!(self, MediaType::Video | MediaType::Screen)
    }
}

/// Quality preference for media streams
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaQuality {
    /// Low bandwidth (audio: 32kbps, video: 240p)
    Low,
    /// Standard (audio: 64kbps, video: 480p)
    Standard,
    /// High (audio: 128kbps, video: 720p)
    High,
    /// Maximum (audio: 256kbps, video: 1080p)
    Max,
}

impl MediaQuality {
    /// Get approximate bandwidth requirement in kbps
    pub fn bandwidth_kbps(&self, media_type: MediaType) -> u32 {
        match (self, media_type) {
            (MediaQuality::Low, MediaType::Audio) => 32,
            (MediaQuality::Standard, MediaType::Audio) => 64,
            (MediaQuality::High, MediaType::Audio) => 128,
            (MediaQuality::Max, MediaType::Audio) => 256,
            (MediaQuality::Low, MediaType::Video) => 250,
            (MediaQuality::Standard, MediaType::Video) => 500,
            (MediaQuality::High, MediaType::Video) => 1500,
            (MediaQuality::Max, MediaType::Video) => 4000,
            (MediaQuality::Low, MediaType::Screen) => 500,
            (MediaQuality::Standard, MediaType::Screen) => 1000,
            (MediaQuality::High, MediaType::Screen) => 2000,
            (MediaQuality::Max, MediaType::Screen) => 5000,
            (_, MediaType::Data) => 100, // Minimal for data channels
        }
    }
}

impl Default for MediaQuality {
    fn default() -> Self {
        MediaQuality::Standard
    }
}

// ============================================================================
// Media Commands
// ============================================================================

/// WebRTC media signaling commands
///
/// These are JSON-encoded and sent through the SBD message channel.
/// The `session_id` field groups related messages for a single call/session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MediaCmd {
    /// Initiate a media session with SDP offer
    MediaOffer {
        /// Unique session identifier
        session_id: String,
        /// WebRTC SDP offer
        sdp: String,
        /// Requested media types
        media_types: Vec<MediaType>,
        /// Requested quality (optional)
        #[serde(default)]
        quality: MediaQuality,
        /// Whether to record this session
        #[serde(default)]
        record: bool,
    },

    /// Accept a media session with SDP answer
    MediaAnswer {
        /// Session identifier from offer
        session_id: String,
        /// WebRTC SDP answer
        sdp: String,
        /// Accepted media types (subset of offer)
        media_types: Vec<MediaType>,
    },

    /// ICE candidate for connection establishment
    IceCandidate {
        /// Session identifier
        session_id: String,
        /// ICE candidate string
        candidate: String,
        /// SDP mid
        #[serde(default)]
        sdp_mid: Option<String>,
        /// SDP mline index
        #[serde(default)]
        sdp_mline_index: Option<u32>,
    },

    /// Batch of ICE candidates (reduces round-trips)
    IceCandidateBatch {
        /// Session identifier
        session_id: String,
        /// List of candidates
        candidates: Vec<IceCandidate>,
    },

    /// End a media session
    MediaEnd {
        /// Session identifier
        session_id: String,
        /// Reason for ending
        #[serde(default)]
        reason: MediaEndReason,
    },

    /// Renegotiate media (e.g., add screen share)
    MediaRenegotiate {
        /// Session identifier
        session_id: String,
        /// New SDP offer
        sdp: String,
        /// Updated media types
        media_types: Vec<MediaType>,
    },

    /// Quality change request
    QualityChange {
        /// Session identifier
        session_id: String,
        /// New quality level
        quality: MediaQuality,
    },
}

/// Individual ICE candidate for batching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    /// ICE candidate string
    pub candidate: String,
    /// SDP mid
    #[serde(default)]
    pub sdp_mid: Option<String>,
    /// SDP mline index
    #[serde(default)]
    pub sdp_mline_index: Option<u32>,
}

/// Reason for ending a media session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaEndReason {
    /// Normal hangup
    #[default]
    Normal,
    /// Call declined
    Declined,
    /// Network failure
    NetworkError,
    /// Timeout (no answer)
    Timeout,
    /// Recording stopped
    RecordingStopped,
    /// Other reason
    Other,
}

impl MediaCmd {
    /// Parse a JSON payload into a media command
    pub fn parse(json: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(json)
    }

    /// Serialize to JSON bytes
    pub fn to_json(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Get the session ID for this command
    pub fn session_id(&self) -> &str {
        match self {
            MediaCmd::MediaOffer { session_id, .. } => session_id,
            MediaCmd::MediaAnswer { session_id, .. } => session_id,
            MediaCmd::IceCandidate { session_id, .. } => session_id,
            MediaCmd::IceCandidateBatch { session_id, .. } => session_id,
            MediaCmd::MediaEnd { session_id, .. } => session_id,
            MediaCmd::MediaRenegotiate { session_id, .. } => session_id,
            MediaCmd::QualityChange { session_id, .. } => session_id,
        }
    }
}

// ============================================================================
// Media Session Tracking
// ============================================================================

/// Media session state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaSessionState {
    /// Offer sent, waiting for answer
    Offering,
    /// Answer received, establishing connection
    Connecting,
    /// Media flowing
    Connected,
    /// Session ending
    Ending,
    /// Session ended
    Ended,
}

/// Tracked media session
#[derive(Debug, Clone)]
pub struct MediaSession {
    /// Unique session ID
    pub id: String,
    /// Initiator public key (base64)
    pub initiator: String,
    /// Receiver public key (base64)
    pub receiver: String,
    /// Current state
    pub state: MediaSessionState,
    /// Active media types
    pub media_types: Vec<MediaType>,
    /// Quality level
    pub quality: MediaQuality,
    /// Whether recording is enabled
    pub recording: bool,
    /// Session start time
    pub started_at: Instant,
    /// Last activity time
    pub last_activity: Instant,
}

impl MediaSession {
    /// Create a new media session from an offer
    pub fn new(
        id: String,
        initiator: String,
        receiver: String,
        media_types: Vec<MediaType>,
        quality: MediaQuality,
        recording: bool,
    ) -> Self {
        let now = Instant::now();
        Self {
            id,
            initiator,
            receiver,
            state: MediaSessionState::Offering,
            media_types,
            quality,
            recording,
            started_at: now,
            last_activity: now,
        }
    }

    /// Update last activity timestamp
    pub fn touch(&mut self) {
        self.last_activity = Instant::now();
    }

    /// Get session duration
    pub fn duration(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get idle duration (since last activity)
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }

    /// Check if session is expired (no activity for given duration)
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.idle_duration() > timeout
    }

    /// Transition to next state
    pub fn transition(&mut self, new_state: MediaSessionState) {
        self.state = new_state;
        self.touch();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_offer_serialization() {
        let cmd = MediaCmd::MediaOffer {
            session_id: "sess-123".to_string(),
            sdp: "v=0\r\n...".to_string(),
            media_types: vec![MediaType::Audio, MediaType::Video],
            quality: MediaQuality::High,
            record: true,
        };

        let json = cmd.to_json().unwrap();
        let parsed: MediaCmd = MediaCmd::parse(&json).unwrap();

        match parsed {
            MediaCmd::MediaOffer {
                session_id,
                media_types,
                quality,
                record,
                ..
            } => {
                assert_eq!(session_id, "sess-123");
                assert_eq!(media_types.len(), 2);
                assert_eq!(quality, MediaQuality::High);
                assert!(record);
            }
            _ => panic!("Expected MediaOffer"),
        }
    }

    #[test]
    fn test_media_answer_serialization() {
        let cmd = MediaCmd::MediaAnswer {
            session_id: "sess-123".to_string(),
            sdp: "v=0\r\n...".to_string(),
            media_types: vec![MediaType::Audio],
        };

        let json = cmd.to_json().unwrap();
        let parsed: MediaCmd = MediaCmd::parse(&json).unwrap();

        assert_eq!(parsed.session_id(), "sess-123");
    }

    #[test]
    fn test_ice_candidate_batch() {
        let cmd = MediaCmd::IceCandidateBatch {
            session_id: "sess-123".to_string(),
            candidates: vec![
                IceCandidate {
                    candidate: "candidate:1".to_string(),
                    sdp_mid: Some("0".to_string()),
                    sdp_mline_index: Some(0),
                },
                IceCandidate {
                    candidate: "candidate:2".to_string(),
                    sdp_mid: Some("0".to_string()),
                    sdp_mline_index: Some(0),
                },
            ],
        };

        let json = cmd.to_json().unwrap();
        let parsed: MediaCmd = MediaCmd::parse(&json).unwrap();

        match parsed {
            MediaCmd::IceCandidateBatch { candidates, .. } => {
                assert_eq!(candidates.len(), 2);
            }
            _ => panic!("Expected IceCandidateBatch"),
        }
    }

    #[test]
    fn test_media_quality_bandwidth() {
        assert_eq!(MediaQuality::Low.bandwidth_kbps(MediaType::Audio), 32);
        assert_eq!(MediaQuality::High.bandwidth_kbps(MediaType::Video), 1500);
        assert_eq!(MediaQuality::Max.bandwidth_kbps(MediaType::Screen), 5000);
    }

    #[test]
    fn test_media_session_lifecycle() {
        let mut session = MediaSession::new(
            "sess-1".to_string(),
            "initiator-pk".to_string(),
            "receiver-pk".to_string(),
            vec![MediaType::Video],
            MediaQuality::Standard,
            false,
        );

        assert_eq!(session.state, MediaSessionState::Offering);

        session.transition(MediaSessionState::Connecting);
        assert_eq!(session.state, MediaSessionState::Connecting);

        session.transition(MediaSessionState::Connected);
        assert_eq!(session.state, MediaSessionState::Connected);
    }
}
