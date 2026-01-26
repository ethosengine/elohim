//! Update download and verification
//!
//! Downloads update binaries from doorway and verifies:
//! - SHA256 checksum
//! - Ed25519 signature (if provided)

use std::path::{Path, PathBuf};
use std::io::Write;
use tracing::{info, warn};

use super::UpdateError;

/// Handles downloading and verifying updates
pub struct UpdateDownloader {
    update_dir: PathBuf,
}

impl UpdateDownloader {
    pub fn new(update_dir: PathBuf) -> Self {
        Self { update_dir }
    }

    /// Download an update file
    pub async fn download<F>(
        &self,
        url: &str,
        version: &str,
        expected_checksum: &str,
        progress_callback: F,
    ) -> Result<PathBuf, UpdateError>
    where
        F: Fn(u8),
    {
        // Ensure update directory exists
        std::fs::create_dir_all(&self.update_dir)
            .map_err(|e| UpdateError::Io(e.to_string()))?;

        let download_path = self.update_dir.join(format!("elohim-node-{}.download", version));
        let final_path = self.update_dir.join(format!("elohim-node-{}", version));

        // Check if already downloaded and verified
        if final_path.exists() {
            if self.verify_checksum(&final_path, expected_checksum)? {
                info!("Update {} already downloaded and verified", version);
                return Ok(final_path);
            } else {
                // Checksum mismatch, re-download
                std::fs::remove_file(&final_path)
                    .map_err(|e| UpdateError::Io(e.to_string()))?;
            }
        }

        info!("Downloading update from {}", url);

        let client = reqwest::Client::new();
        let response = client.get(url)
            .send()
            .await
            .map_err(|e| UpdateError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(UpdateError::DownloadFailed(format!(
                "HTTP {}", response.status()
            )));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;

        // Stream to file
        let mut file = std::fs::File::create(&download_path)
            .map_err(|e| UpdateError::Io(e.to_string()))?;

        let mut stream = response.bytes_stream();
        use futures::StreamExt;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| UpdateError::Network(e.to_string()))?;
            file.write_all(&chunk)
                .map_err(|e| UpdateError::Io(e.to_string()))?;

            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let progress = ((downloaded * 100) / total_size) as u8;
                progress_callback(progress);
            }
        }

        file.flush().map_err(|e| UpdateError::Io(e.to_string()))?;
        drop(file);

        info!("Download complete, verifying checksum");

        // Verify checksum
        if !self.verify_checksum(&download_path, expected_checksum)? {
            std::fs::remove_file(&download_path).ok();
            return Err(UpdateError::ChecksumMismatch);
        }

        // Rename to final path
        std::fs::rename(&download_path, &final_path)
            .map_err(|e| UpdateError::Io(e.to_string()))?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&final_path)
                .map_err(|e| UpdateError::Io(e.to_string()))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&final_path, perms)
                .map_err(|e| UpdateError::Io(e.to_string()))?;
        }

        info!("Update {} downloaded and verified", version);
        Ok(final_path)
    }

    /// Verify SHA256 checksum of a file
    pub fn verify_checksum(&self, path: &Path, expected: &str) -> Result<bool, UpdateError> {
        use sha2::{Sha256, Digest};

        let mut file = std::fs::File::open(path)
            .map_err(|e| UpdateError::Io(e.to_string()))?;

        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)
            .map_err(|e| UpdateError::Io(e.to_string()))?;

        let result = hasher.finalize();
        let hex = format!("{:x}", result);

        Ok(hex == expected.to_lowercase())
    }

    /// Verify Ed25519 signature
    pub fn verify_signature(&self, path: &Path, signature: &str) -> Result<(), UpdateError> {
        // TODO: Implement Ed25519 signature verification
        // For now, just warn and accept
        warn!("Signature verification not yet implemented");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_verify_checksum() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let content = b"hello world";
        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(content).unwrap();

        let downloader = UpdateDownloader::new(temp_dir.path().to_path_buf());

        // SHA256 of "hello world"
        let expected = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(downloader.verify_checksum(&file_path, expected).unwrap());

        // Wrong checksum
        assert!(!downloader.verify_checksum(&file_path, "wrong").unwrap());
    }
}
