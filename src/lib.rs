// ============================================================================
// Secure File Sharing System with Integrity Verification
// ============================================================================

pub mod crypto;
pub mod core;
pub mod storage;
pub mod filter;
pub mod auth;
pub mod service;
pub mod db;

// Re-export commonly used types
pub use crypto::hash::{HashAlgo, HashValue};
pub use core::file_metadata::FileMetadata;
pub use service::file_sharing::FileSharingService;
pub use db::database::Database;
pub use db::models::{User, SharedFile};