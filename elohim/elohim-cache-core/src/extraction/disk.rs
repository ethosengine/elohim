//! DiskBackend — filesystem-backed cache storage

use super::backend::CacheBackend;
use super::CacheError;
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

/// Filesystem-backed cache storage.
pub struct DiskBackend {
    root_dir: PathBuf,
}

impl DiskBackend {
    /// Create a new disk backend rooted at the given directory.
    pub async fn new(root_dir: PathBuf) -> Result<Self, CacheError> {
        fs::create_dir_all(&root_dir).await?;
        Ok(Self { root_dir })
    }

    /// Resolve a cache key to a filesystem path.
    fn resolve_path(&self, key: &str) -> Result<PathBuf, CacheError> {
        if key.contains("..") || key.contains('\0') || key.starts_with('/') {
            return Err(CacheError::InvalidKey(key.to_string()));
        }
        Ok(self.root_dir.join(key))
    }
}

#[async_trait]
impl CacheBackend for DiskBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let path = self.resolve_path(key)?;
        match fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(CacheError::Io(e)),
        }
    }

    async fn put(&self, key: &str, data: Vec<u8>) -> Result<bool, CacheError> {
        let path = self.resolve_path(key)?;
        let is_new = !path.exists();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, &data).await?;
        Ok(is_new)
    }

    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let path = self.resolve_path(key)?;
        match fs::remove_file(&path).await {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(CacheError::Io(e)),
        }
    }

    async fn delete_prefix(&self, prefix: &str) -> Result<u64, CacheError> {
        let path = self.resolve_path(prefix.trim_end_matches('/'))?;
        if !path.exists() {
            return Ok(0);
        }
        let count = count_files_recursive(&path).await;
        match fs::remove_dir_all(&path).await {
            Ok(()) => Ok(count),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(CacheError::Io(e)),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let path = self.resolve_path(key)?;
        Ok(fs::try_exists(&path).await.unwrap_or(false))
    }

    async fn total_size(&self) -> Result<u64, CacheError> {
        Ok(dir_size_recursive(&self.root_dir).await)
    }
}

async fn count_files_recursive(dir: &std::path::Path) -> u64 {
    let mut count = 0u64;
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(ft) = entry.file_type().await {
                if ft.is_file() {
                    count += 1;
                } else if ft.is_dir() {
                    count += Box::pin(count_files_recursive(&entry.path())).await;
                }
            }
        }
    }
    count
}

async fn dir_size_recursive(dir: &std::path::Path) -> u64 {
    let mut size = 0u64;
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(ft) = entry.file_type().await {
                if ft.is_file() {
                    if let Ok(meta) = entry.metadata().await {
                        size += meta.len();
                    }
                } else if ft.is_dir() {
                    size += Box::pin(dir_size_recursive(&entry.path())).await;
                }
            }
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn temp_backend() -> (DiskBackend, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let backend = DiskBackend::new(tmp.path().to_path_buf()).await.unwrap();
        (backend, tmp)
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let (backend, _tmp) = temp_backend().await;
        let is_new = backend
            .put("app1/index.html", b"<html>hello</html>".to_vec())
            .await
            .unwrap();
        assert!(is_new);
        let data = backend.get("app1/index.html").await.unwrap();
        assert_eq!(data, Some(b"<html>hello</html>".to_vec()));
    }

    #[tokio::test]
    async fn test_get_missing_returns_none() {
        let (backend, _tmp) = temp_backend().await;
        assert_eq!(backend.get("nonexistent/file.js").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_put_overwrite_returns_false() {
        let (backend, _tmp) = temp_backend().await;
        backend
            .put("app1/index.html", b"v1".to_vec())
            .await
            .unwrap();
        let is_new = backend
            .put("app1/index.html", b"v2".to_vec())
            .await
            .unwrap();
        assert!(!is_new);
        assert_eq!(
            backend.get("app1/index.html").await.unwrap(),
            Some(b"v2".to_vec())
        );
    }

    #[tokio::test]
    async fn test_delete() {
        let (backend, _tmp) = temp_backend().await;
        backend
            .put("app1/style.css", b"body{}".to_vec())
            .await
            .unwrap();
        assert!(backend.delete("app1/style.css").await.unwrap());
        assert!(!backend.delete("app1/style.css").await.unwrap());
        assert_eq!(backend.get("app1/style.css").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_delete_prefix() {
        let (backend, _tmp) = temp_backend().await;
        backend
            .put("app1/index.html", b"html".to_vec())
            .await
            .unwrap();
        backend
            .put("app1/js/main.js", b"js".to_vec())
            .await
            .unwrap();
        backend
            .put("app1/css/style.css", b"css".to_vec())
            .await
            .unwrap();
        backend
            .put("app2/index.html", b"other".to_vec())
            .await
            .unwrap();

        let deleted = backend.delete_prefix("app1").await.unwrap();
        assert_eq!(deleted, 3);
        assert_eq!(backend.get("app1/index.html").await.unwrap(), None);
        assert_eq!(
            backend.get("app2/index.html").await.unwrap(),
            Some(b"other".to_vec())
        );
    }

    #[tokio::test]
    async fn test_exists() {
        let (backend, _tmp) = temp_backend().await;
        assert!(!backend.exists("app1/index.html").await.unwrap());
        backend
            .put("app1/index.html", b"html".to_vec())
            .await
            .unwrap();
        assert!(backend.exists("app1/index.html").await.unwrap());
    }

    #[tokio::test]
    async fn test_total_size() {
        let (backend, _tmp) = temp_backend().await;
        backend.put("a/1.txt", b"hello".to_vec()).await.unwrap();
        backend.put("a/2.txt", b"world!".to_vec()).await.unwrap();
        assert_eq!(backend.total_size().await.unwrap(), 11);
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (backend, _tmp) = temp_backend().await;
        assert!(matches!(
            backend.put("../escape/file", b"bad".to_vec()).await,
            Err(CacheError::InvalidKey(_))
        ));
        assert!(matches!(
            backend.get("../../etc/passwd").await,
            Err(CacheError::InvalidKey(_))
        ));
    }
}
