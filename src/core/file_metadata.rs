// ============================================================================
// File Metadata Structures
// ============================================================================

use crate::crypto::hash::HashValue;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub size: u64,
    pub hash: HashValue,
    pub chunks: Vec<HashValue>,
    pub merkle_root: HashValue,
    pub created_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
    pub owner: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChunk {
    pub index: usize,
    pub hash: HashValue,
    pub data: Vec<u8>,
}