#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

pub mod build_status;
pub mod cache;
pub mod config;
pub mod field_mask;
pub mod index;
pub mod ir;
pub mod lang;
pub mod manifest;
pub mod output;
pub mod parser;
pub mod prompt;
pub mod query_engine;
pub mod schema;
pub mod resolver;
pub mod validate;
pub mod types;
pub mod walker;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// Crate-wide error type. All errors via `thiserror`, all propagation via `?`.
#[derive(Debug, Error)]
pub enum DomainScanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Walk error: {0}")]
    Walk(String),

    #[error("Tree-sitter language error: {0}")]
    TreeSitterLanguage(String),

    #[error("Parse failed for: {0}")]
    ParseFailed(std::path::PathBuf),

    #[error("Language not supported: {0:?}")]
    UnsupportedLanguage(ir::Language),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Query compilation error: {0}")]
    QueryCompile(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Cache error: {0}")]
    Cache(String),

    #[error("Field mask error: {0}")]
    FieldMask(String),
}

/// Compute SHA-256 content hash for caching.
pub fn content_hash(content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_hash_deterministic() {
        let a = content_hash(b"hello world");
        let b = content_hash(b"hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn test_content_hash_differs() {
        let a = content_hash(b"hello");
        let b = content_hash(b"world");
        assert_ne!(a, b);
    }
}
