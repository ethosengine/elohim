//! Doorway HTTP client for Tauri bootstrap
//!
//! Authenticates with a doorway instance and retrieves identity + network context
//! for installing the hApp with the same agent key the doorway provisioned.

use serde::{Deserialize, Serialize};

/// Configuration for connecting to a doorway instance
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DoorwayConfig {
    /// Base URL of the doorway (e.g. "https://doorway.elohim.host")
    pub url: String,
}

/// Login response from POST /auth/login
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    pub token: String,
    pub human_id: String,
    pub identifier: String,
}

/// Encrypted key bundle for identity import (mirrors doorway's KeyExportFormat)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyExportFormat {
    pub version: u32,
    pub identifier: String,
    pub human_id: String,
    pub public_key: String,
    pub encrypted_private_key: String,
    pub key_derivation_salt: String,
    pub encryption_nonce: String,
    pub exported_at: String,
    pub doorway_id: String,
}

/// Native handoff response from GET /auth/native-handoff
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeHandoffResponse {
    pub human_id: String,
    pub identifier: String,
    pub agent_pub_key: String,
    pub doorway_id: String,
    pub doorway_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_image_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signal_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_seed: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub installed_app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conductor_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_bundle: Option<KeyExportFormat>,
}

/// Response from POST /auth/confirm-stewardship
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StewardshipConfirmedResponse {
    pub success: bool,
    pub message: String,
    pub stewardship_at: String,
}

/// Error response from doorway API
#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(default)]
    pub code: Option<String>,
}

/// HTTP client for doorway authentication and identity handoff
pub struct DoorwayClient {
    config: DoorwayConfig,
    http: reqwest::Client,
}

impl DoorwayClient {
    /// Create a new doorway client for the given URL
    pub fn new(url: String) -> Self {
        Self {
            config: DoorwayConfig { url },
            http: reqwest::Client::new(),
        }
    }

    /// Authenticate with the doorway and get a JWT token
    pub async fn login(&self, identifier: &str, password: &str) -> Result<LoginResponse, String> {
        let url = format!("{}/auth/login", self.config.url);

        let body = serde_json::json!({
            "identifier": identifier,
            "password": password,
        });

        let response = self
            .http
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Login request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            // Try to parse error response
            if let Ok(err) = serde_json::from_str::<ErrorResponse>(&body) {
                return Err(format!("Login failed ({}): {}", status, err.error));
            }
            return Err(format!("Login failed ({}): {}", status, body));
        }

        response
            .json::<LoginResponse>()
            .await
            .map_err(|e| format!("Failed to parse login response: {}", e))
    }

    /// Confirm stewardship (graduation) — prove key possession to doorway
    pub async fn confirm_stewardship(
        &self,
        token: &str,
        signature_base64: &str,
    ) -> Result<StewardshipConfirmedResponse, String> {
        let url = format!("{}/auth/confirm-stewardship", self.config.url);

        let body = serde_json::json!({
            "signature": signature_base64,
        });

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Confirm stewardship request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if let Ok(err) = serde_json::from_str::<ErrorResponse>(&body) {
                return Err(format!(
                    "Confirm stewardship failed ({}): {}",
                    status, err.error
                ));
            }
            return Err(format!("Confirm stewardship failed ({}): {}", status, body));
        }

        response
            .json::<StewardshipConfirmedResponse>()
            .await
            .map_err(|e| format!("Failed to parse stewardship response: {}", e))
    }

    /// Retrieve identity + network context for native session bootstrap
    pub async fn native_handoff(&self, token: &str) -> Result<NativeHandoffResponse, String> {
        let url = format!("{}/auth/native-handoff", self.config.url);

        let response = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| format!("Handoff request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if let Ok(err) = serde_json::from_str::<ErrorResponse>(&body) {
                return Err(format!("Handoff failed ({}): {}", status, err.error));
            }
            return Err(format!("Handoff failed ({}): {}", status, body));
        }

        response
            .json::<NativeHandoffResponse>()
            .await
            .map_err(|e| format!("Failed to parse handoff response: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = DoorwayClient::new("https://doorway.test.local".to_string());
        assert_eq!(client.config.url, "https://doorway.test.local");
    }

    #[test]
    fn test_key_export_format_deserialization() {
        let json = r#"{
            "version": 1,
            "identifier": "test@example.com",
            "humanId": "uhCAk_test",
            "publicKey": "base64pubkey==",
            "encryptedPrivateKey": "base64encrypted==",
            "keyDerivationSalt": "base64salt==",
            "encryptionNonce": "base64nonce==",
            "exportedAt": "2025-01-01T00:00:00Z",
            "doorwayId": "doorway-alpha"
        }"#;

        let format: KeyExportFormat = serde_json::from_str(json).unwrap();
        assert_eq!(format.version, 1);
        assert_eq!(format.identifier, "test@example.com");
    }

    #[test]
    fn test_handoff_response_deserialization() {
        let json = r#"{
            "humanId": "uhCAk_test",
            "identifier": "test@example.com",
            "agentPubKey": "uhCAk_agent",
            "doorwayId": "doorway-alpha",
            "doorwayUrl": "https://doorway.test.local",
            "bootstrapUrl": "http://localhost:8888/bootstrap",
            "signalUrl": "ws://localhost:8888"
        }"#;

        let resp: NativeHandoffResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.agent_pub_key, "uhCAk_agent");
        assert_eq!(
            resp.bootstrap_url,
            Some("http://localhost:8888/bootstrap".to_string())
        );
        assert!(resp.key_bundle.is_none());
    }
}
