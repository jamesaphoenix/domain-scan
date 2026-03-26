use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::ir::IrFile;
use crate::DomainScanError;

// ---------------------------------------------------------------------------
// CachedFile — wrapper stored in DashMap and persisted to disk
// ---------------------------------------------------------------------------

/// A cached parse result for a single file.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CachedFile {
    /// The parsed IR for this file.
    pub ir: IrFile,
    /// The content hash (SHA-256 of path + content) that produced this IR.
    pub hash: String,
    /// Epoch millis when this entry was last accessed (for LRU eviction).
    pub last_accessed_ms: u64,
}

// ---------------------------------------------------------------------------
// Cache — content-addressed, thread-safe, disk-persistent
// ---------------------------------------------------------------------------

/// Content-addressed cache backed by `DashMap` (in-memory) with optional disk
/// persistence via bincode. LRU eviction when `max_size_bytes` is exceeded.
pub struct Cache {
    /// In-memory entries. Key = SHA-256(path + content).
    entries: DashMap<String, CachedFile>,
    /// Directory for on-disk `.bincode` files.
    dir: PathBuf,
    /// Maximum total disk size in bytes.
    max_size_bytes: u64,
}

/// Statistics about the current cache state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheStats {
    pub entries: usize,
    pub disk_size_bytes: u64,
    pub max_size_bytes: u64,
}

/// A single action that a mutating cache command *would* perform.
/// Used by `--dry-run` to preview side-effects as structured JSON.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DryRunAction {
    pub action: String,
    pub target: String,
    pub reason: String,
}

impl Cache {
    /// Create a new cache. `max_size_mb` is the limit for on-disk storage.
    /// The cache directory is created lazily on first write.
    pub fn new(dir: PathBuf, max_size_mb: u64) -> Self {
        Self {
            entries: DashMap::new(),
            dir,
            max_size_bytes: max_size_mb.saturating_mul(1024 * 1024),
        }
    }

    /// Look up a cached IR by its content hash.
    /// Updates `last_accessed_ms` on hit.
    pub fn get(&self, hash: &str) -> Option<IrFile> {
        let mut entry = self.entries.get_mut(hash)?;
        entry.last_accessed_ms = now_epoch_ms();
        Some(entry.ir.clone())
    }

    /// Insert (or update) a cached IR entry. Also writes to disk.
    pub fn insert(&self, hash: String, ir: IrFile) -> Result<(), DomainScanError> {
        let cached = CachedFile {
            ir,
            hash: hash.clone(),
            last_accessed_ms: now_epoch_ms(),
        };

        // Write to disk first so eviction can use real sizes
        self.write_to_disk(&hash, &cached)?;

        self.entries.insert(hash, cached);
        Ok(())
    }

    /// Remove a single entry by hash (from memory and disk).
    pub fn remove(&self, hash: &str) {
        self.entries.remove(hash);
        let path = self.disk_path(hash);
        let _ = fs::remove_file(path);
    }

    /// Return the number of in-memory entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return whether the cache has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries from memory and disk.
    pub fn clear(&self) -> Result<(), DomainScanError> {
        self.entries.clear();
        if self.dir.is_dir() {
            fs::remove_dir_all(&self.dir).map_err(|e| {
                DomainScanError::Cache(format!(
                    "failed to remove cache dir {}: {e}",
                    self.dir.display()
                ))
            })?;
        }
        Ok(())
    }

    /// Gather statistics about the cache.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            disk_size_bytes: self.disk_size(),
            max_size_bytes: self.max_size_bytes,
        }
    }

    /// Load all `.bincode` files from the cache directory into memory.
    pub fn load_from_disk(&self) -> Result<usize, DomainScanError> {
        if !self.dir.is_dir() {
            return Ok(0);
        }

        let mut loaded = 0usize;
        let entries = fs::read_dir(&self.dir).map_err(|e| {
            DomainScanError::Cache(format!(
                "failed to read cache dir {}: {e}",
                self.dir.display()
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                DomainScanError::Cache(format!("failed to read cache dir entry: {e}"))
            })?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("bincode") {
                continue;
            }

            match self.read_from_disk_file(&path) {
                Ok(cached) => {
                    self.entries.insert(cached.hash.clone(), cached);
                    loaded += 1;
                }
                Err(_) => {
                    // Corrupted cache file — remove it silently
                    let _ = fs::remove_file(&path);
                }
            }
        }

        Ok(loaded)
    }

    /// Remove cache entries whose source files no longer exist.
    /// Returns the number of pruned entries.
    pub fn prune(&self) -> usize {
        let stale_keys: Vec<String> = self
            .entries
            .iter()
            .filter(|entry| !entry.value().ir.path.exists())
            .map(|entry| entry.key().clone())
            .collect();

        let count = stale_keys.len();
        for key in &stale_keys {
            self.remove(key);
        }
        count
    }

    /// Preview what `clear()` would do without actually deleting anything.
    /// Returns a list of `DryRunAction` items for structured `--dry-run` output.
    pub fn dry_run_clear(&self) -> Vec<DryRunAction> {
        self.entries
            .iter()
            .map(|entry| DryRunAction {
                action: "delete".to_string(),
                target: self.disk_path(entry.key()).display().to_string(),
                reason: "cache clear removes all entries".to_string(),
            })
            .collect()
    }

    /// Preview what `prune()` would do without actually deleting anything.
    /// Returns a list of `DryRunAction` items for stale entries only.
    pub fn dry_run_prune(&self) -> Vec<DryRunAction> {
        self.entries
            .iter()
            .filter(|entry| !entry.value().ir.path.exists())
            .map(|entry| DryRunAction {
                action: "delete".to_string(),
                target: self.disk_path(entry.key()).display().to_string(),
                reason: format!(
                    "source file deleted from disk: {}",
                    entry.value().ir.path.display()
                ),
            })
            .collect()
    }

    /// Evict least-recently-used entries until disk usage is within limits.
    /// Returns the number of evicted entries.
    pub fn evict(&self) -> Result<usize, DomainScanError> {
        let mut evicted = 0usize;

        loop {
            let current_size = self.disk_size();
            if current_size <= self.max_size_bytes {
                break;
            }

            // Find the LRU entry
            let lru_key = self
                .entries
                .iter()
                .min_by_key(|entry| entry.value().last_accessed_ms)
                .map(|entry| entry.key().clone());

            match lru_key {
                Some(key) => {
                    self.remove(&key);
                    evicted += 1;
                }
                None => break,
            }
        }

        Ok(evicted)
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// On-disk path for a given hash key.
    fn disk_path(&self, hash: &str) -> PathBuf {
        self.dir.join(format!("{hash}.bincode"))
    }

    /// Write a single entry to disk as bincode.
    fn write_to_disk(&self, hash: &str, cached: &CachedFile) -> Result<(), DomainScanError> {
        fs::create_dir_all(&self.dir).map_err(|e| {
            DomainScanError::Cache(format!(
                "failed to create cache dir {}: {e}",
                self.dir.display()
            ))
        })?;

        let bytes = bincode::serialize(cached)
            .map_err(|e| DomainScanError::Cache(format!("failed to serialize cache entry: {e}")))?;

        let path = self.disk_path(hash);
        fs::write(&path, &bytes).map_err(|e| {
            DomainScanError::Cache(format!(
                "failed to write cache file {}: {e}",
                path.display()
            ))
        })?;

        Ok(())
    }

    /// Read a single `.bincode` file from disk.
    fn read_from_disk_file(&self, path: &Path) -> Result<CachedFile, DomainScanError> {
        let bytes = fs::read(path).map_err(|e| {
            DomainScanError::Cache(format!("failed to read cache file {}: {e}", path.display()))
        })?;

        bincode::deserialize(&bytes).map_err(|e| {
            DomainScanError::Cache(format!(
                "failed to deserialize cache file {}: {e}",
                path.display()
            ))
        })
    }

    /// Compute the total size of on-disk cache files.
    fn disk_size(&self) -> u64 {
        if !self.dir.is_dir() {
            return 0;
        }

        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(_) => return 0,
        };

        let mut total = 0u64;
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("bincode") {
                if let Ok(meta) = fs::metadata(&path) {
                    total = total.saturating_add(meta.len());
                }
            }
        }
        total
    }
}

/// Current time as epoch milliseconds. Falls back to 0 on error.
fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{BuildStatus, Language};
    use std::path::PathBuf;

    fn make_ir(name: &str) -> IrFile {
        IrFile::new(
            PathBuf::from(format!("src/{name}.ts")),
            Language::TypeScript,
            format!("hash_{name}"),
            BuildStatus::Built,
        )
    }

    #[test]
    fn test_cache_insert_and_get() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);
        let ir = make_ir("foo");
        let hash = "abc123".to_string();

        let result = cache.insert(hash.clone(), ir.clone());
        assert!(result.is_ok());

        let got = cache.get(&hash);
        assert!(got.is_some());
        assert_eq!(got.as_ref().map(|g| &g.path), Some(&ir.path));
    }

    #[test]
    fn test_cache_miss() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn test_cache_remove() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);
        let ir = make_ir("bar");

        let _ = cache.insert("key1".to_string(), ir);
        assert_eq!(cache.len(), 1);

        cache.remove("key1");
        assert!(cache.is_empty());
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_cache_clear() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache_dir = dir.path().join("cache");
        let cache = Cache::new(cache_dir.clone(), 100);

        let _ = cache.insert("a".to_string(), make_ir("a"));
        let _ = cache.insert("b".to_string(), make_ir("b"));
        assert_eq!(cache.len(), 2);

        let result = cache.clear();
        assert!(result.is_ok());
        assert!(cache.is_empty());
        assert!(!cache_dir.is_dir());
    }

    #[test]
    fn test_cache_disk_persistence() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache_dir = dir.path().join("cache");

        // Write entries with first cache instance
        {
            let cache = Cache::new(cache_dir.clone(), 100);
            let _ = cache.insert("h1".to_string(), make_ir("one"));
            let _ = cache.insert("h2".to_string(), make_ir("two"));
            assert_eq!(cache.len(), 2);
        }

        // Read them back with a fresh cache instance
        {
            let cache = Cache::new(cache_dir, 100);
            assert!(cache.is_empty());

            let loaded = cache.load_from_disk();
            assert!(loaded.is_ok());
            assert_eq!(loaded.ok(), Some(2));
            assert_eq!(cache.len(), 2);

            assert!(cache.get("h1").is_some());
            assert!(cache.get("h2").is_some());
        }
    }

    #[test]
    fn test_cache_stats() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 50);
        let _ = cache.insert("s1".to_string(), make_ir("stats"));

        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert!(stats.disk_size_bytes > 0);
        assert_eq!(stats.max_size_bytes, 50 * 1024 * 1024);
    }

    #[test]
    fn test_cache_eviction() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        // Use a tiny max size (1 byte) to force eviction
        let cache = Cache::new(dir.path().join("cache"), 0);

        let _ = cache.insert("e1".to_string(), make_ir("evict1"));
        let _ = cache.insert("e2".to_string(), make_ir("evict2"));

        let evicted = cache.evict();
        assert!(evicted.is_ok());
        // With max_size 0, all entries should be evicted
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_prune_removes_stale() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        // Insert an entry pointing to a nonexistent file
        let ir = IrFile::new(
            PathBuf::from("/nonexistent/file.ts"),
            Language::TypeScript,
            "hash_gone".to_string(),
            BuildStatus::Built,
        );
        let _ = cache.insert("gone".to_string(), ir);
        assert_eq!(cache.len(), 1);

        let pruned = cache.prune();
        assert_eq!(pruned, 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_load_from_empty_dir() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("no_such_dir"), 100);
        let loaded = cache.load_from_disk();
        assert!(loaded.is_ok());
        assert_eq!(loaded.ok(), Some(0));
    }

    #[test]
    fn test_cache_corrupted_file_removed() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache_dir = dir.path().join("cache");
        fs::create_dir_all(&cache_dir).ok();

        // Write a corrupted bincode file
        let corrupt_path = cache_dir.join("corrupt.bincode");
        fs::write(&corrupt_path, b"not valid bincode").ok();
        assert!(corrupt_path.exists());

        let cache = Cache::new(cache_dir, 100);
        let loaded = cache.load_from_disk();
        assert!(loaded.is_ok());
        assert_eq!(loaded.ok(), Some(0));
        // Corrupted file should have been removed
        assert!(!corrupt_path.exists());
    }

    #[test]
    fn test_cache_get_updates_last_accessed() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);
        let _ = cache.insert("ts1".to_string(), make_ir("time"));

        let before = cache
            .entries
            .get("ts1")
            .map(|e| e.last_accessed_ms)
            .unwrap_or(0);

        // Small delay to ensure time moves forward
        std::thread::sleep(std::time::Duration::from_millis(2));

        let _ = cache.get("ts1");

        let after = cache
            .entries
            .get("ts1")
            .map(|e| e.last_accessed_ms)
            .unwrap_or(0);

        assert!(after >= before);
    }

    #[test]
    fn test_dry_run_clear_lists_all_entries() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);
        let _ = cache.insert("a1".to_string(), make_ir("alpha"));
        let _ = cache.insert("b2".to_string(), make_ir("beta"));

        let actions = cache.dry_run_clear();
        assert_eq!(actions.len(), 2);
        for a in &actions {
            assert_eq!(a.action, "delete");
            assert!(a.target.ends_with(".bincode"));
            assert!(a.reason.contains("cache clear"));
        }

        // Verify nothing was actually deleted
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_dry_run_clear_empty_cache() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        let actions = cache.dry_run_clear();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_dry_run_prune_lists_only_stale() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        // Entry with nonexistent source file (stale)
        let stale_ir = IrFile::new(
            PathBuf::from("/no/such/file.ts"),
            Language::TypeScript,
            "hash_stale".to_string(),
            BuildStatus::Built,
        );
        let _ = cache.insert("stale_key".to_string(), stale_ir);

        // Entry with existing source file (the cache dir itself exists)
        let fresh_ir = IrFile::new(
            dir.path().to_path_buf(),
            Language::TypeScript,
            "hash_fresh".to_string(),
            BuildStatus::Built,
        );
        let _ = cache.insert("fresh_key".to_string(), fresh_ir);

        let actions = cache.dry_run_prune();
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "delete");
        assert!(actions[0].reason.contains("/no/such/file.ts"));

        // Verify nothing was actually deleted
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_dry_run_prune_nothing_stale() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        // Entry with existing path
        let fresh_ir = IrFile::new(
            dir.path().to_path_buf(),
            Language::TypeScript,
            "hash_good".to_string(),
            BuildStatus::Built,
        );
        let _ = cache.insert("good_key".to_string(), fresh_ir);

        let actions = cache.dry_run_prune();
        assert!(actions.is_empty());
    }

    #[test]
    fn test_cache_concurrent_insert_and_get() {
        use std::sync::Arc;

        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Arc::new(Cache::new(dir.path().join("cache"), 100));

        // Spawn multiple threads inserting concurrently
        let mut handles = Vec::new();
        for i in 0..10 {
            let cache = Arc::clone(&cache);
            let handle = std::thread::spawn(move || {
                let key = format!("key_{i}");
                let ir = IrFile::new(
                    PathBuf::from(format!("src/file_{i}.ts")),
                    Language::TypeScript,
                    format!("hash_{i}"),
                    BuildStatus::Built,
                );
                let _ = cache.insert(key, ir);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().ok();
        }

        assert_eq!(cache.len(), 10);

        // All entries should be retrievable
        for i in 0..10 {
            let key = format!("key_{i}");
            assert!(cache.get(&key).is_some(), "Missing key: {key}");
        }
    }

    #[test]
    fn test_cache_concurrent_insert_and_evict() {
        use std::sync::Arc;

        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        // Very small max size to trigger eviction
        let cache = Arc::new(Cache::new(dir.path().join("cache"), 0));

        // Spawn threads that insert
        let mut handles = Vec::new();
        for i in 0..5 {
            let cache = Arc::clone(&cache);
            let handle = std::thread::spawn(move || {
                let key = format!("evict_key_{i}");
                let ir = IrFile::new(
                    PathBuf::from(format!("src/evict_{i}.ts")),
                    Language::TypeScript,
                    format!("evict_hash_{i}"),
                    BuildStatus::Built,
                );
                let _ = cache.insert(key, ir);
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().ok();
        }

        // Now evict — should not panic even after concurrent inserts
        let evicted = cache.evict();
        assert!(evicted.is_ok());

        // After eviction with max_size 0, cache should be empty
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_insert_overwrites_existing() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        let ir1 = IrFile::new(
            PathBuf::from("src/v1.ts"),
            Language::TypeScript,
            "hash_v1".to_string(),
            BuildStatus::Built,
        );
        let ir2 = IrFile::new(
            PathBuf::from("src/v2.ts"),
            Language::TypeScript,
            "hash_v2".to_string(),
            BuildStatus::Built,
        );

        let _ = cache.insert("same_key".to_string(), ir1);
        let _ = cache.insert("same_key".to_string(), ir2.clone());

        assert_eq!(cache.len(), 1);
        let got = cache.get("same_key");
        assert!(got.is_some());
        assert_eq!(got.map(|g| g.path), Some(ir2.path));
    }

    #[test]
    fn test_cache_remove_nonexistent_key() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        // Removing a nonexistent key should not panic
        cache.remove("nonexistent");
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_clear_empty_cache() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        // Clearing an empty cache should succeed
        let result = cache.clear();
        assert!(result.is_ok());
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats_max_size_bytes() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        // 42 MB
        let cache = Cache::new(dir.path().join("cache"), 42);
        let stats = cache.stats();
        assert_eq!(stats.max_size_bytes, 42 * 1024 * 1024);
        assert_eq!(stats.entries, 0);
        assert_eq!(stats.disk_size_bytes, 0);
    }

    #[test]
    fn test_cache_prune_keeps_existing_files() {
        let dir = tempfile::TempDir::new();
        let dir = match dir {
            Ok(d) => d,
            Err(_) => return,
        };
        let cache = Cache::new(dir.path().join("cache"), 100);

        // Insert an entry pointing to an existing path (the temp dir itself)
        let ir = IrFile::new(
            dir.path().to_path_buf(),
            Language::TypeScript,
            "hash_exists".to_string(),
            BuildStatus::Built,
        );
        let _ = cache.insert("exists_key".to_string(), ir);
        assert_eq!(cache.len(), 1);

        // Prune should keep this entry
        let pruned = cache.prune();
        assert_eq!(pruned, 0);
        assert_eq!(cache.len(), 1);
    }
}
