// ============================================================================
// Storage Engine with Deduplication
// ============================================================================

use crate::crypto::hash::{HashAlgo, HashValue};
use crate::core::file_metadata::FileMetadata;
use crate::core::merkle_tree::MerkleTree;
use anyhow::{Result, Context};
use serde_json;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use chrono::Utc;

#[derive(Debug, Default)]
pub struct DedupStats {
    pub total_files: usize,
    pub unique_files: usize,
    pub total_bytes: u64,
    pub saved_bytes: u64,
}

pub struct StorageEngine {
    storage_dir: PathBuf,
    hash_to_path: HashMap<String, PathBuf>,     // hex hash -> file on disk
    hash_to_metadata: HashMap<String, FileMetadata>, // hex hash -> metadata
    pub dedup_stats: DedupStats,  // Made public
}

impl StorageEngine {
    pub fn new(storage_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(storage_dir)?;
        Ok(Self {
            storage_dir: storage_dir.to_path_buf(),
            hash_to_path: HashMap::new(),
            hash_to_metadata: HashMap::new(),
            dedup_stats: DedupStats::default(),
        })
    }

    pub fn store_file(&mut self, data: &[u8], filename: &str, owner: &str) -> Result<FileMetadata> {
        let hash = HashValue::compute(data, HashAlgo::Sha256);
        let hex = hash.to_hex();
        
        // Deduplication: if file exists, return metadata only
        if let Some(existing) = self.hash_to_metadata.get(&hex) {
            self.dedup_stats.total_files += 1;
            self.dedup_stats.total_bytes += data.len() as u64;
            self.dedup_stats.saved_bytes += data.len() as u64;
            println!("♻️  duplicate detected: {} -> refers to existing file", filename);
            return Ok(existing.clone());
        }

        // New file - split into 1MB chunks
        let chunks: Vec<HashValue> = data.chunks(1024 * 1024).enumerate().map(|(i, chunk)| {
            let chunk_hash = HashValue::compute(chunk, HashAlgo::Sha256);
            let chunk_path = self.storage_dir.join(format!("{}_{}.chunk", hex, i));
            let mut file = File::create(&chunk_path).unwrap();
            file.write_all(chunk).unwrap();
            chunk_hash
        }).collect();

        // Build Merkle Tree
        let merkle_tree = MerkleTree::new(&chunks);
        let merkle_root = merkle_tree.root();

        // Save metadata
        let metadata = FileMetadata {
            path: PathBuf::from(filename),
            size: data.len() as u64,
            hash: hash.clone(),
            chunks: chunks.clone(),  // Clone here
            merkle_root,
            created_at: Utc::now(),
            modified_at: Utc::now(),
            owner: owner.to_string(),
        };

        let meta_path = self.storage_dir.join(format!("{}.meta", hex));
        let meta_json = serde_json::to_string_pretty(&metadata)?;
        std::fs::write(&meta_path, meta_json)?;

        // Update state
        self.hash_to_path.insert(hex.clone(), meta_path);
        self.hash_to_metadata.insert(hex, metadata.clone());
        
        self.dedup_stats.total_files += 1;
        self.dedup_stats.unique_files += 1;
        self.dedup_stats.total_bytes += data.len() as u64;

        println!(" new file stored: {} ({} bytes, {} chunks)", 
            filename, data.len(), chunks.len());
        
        Ok(metadata)
    }

    pub fn retrieve_file(&self, hash: &HashValue) -> Result<Vec<u8>> {
        let hex = hash.to_hex();
        let metadata = self.hash_to_metadata.get(&hex)
            .context("file not found")?;

        let mut full_data = Vec::new();
        for (i, chunk_hash) in metadata.chunks.iter().enumerate() {
            let chunk_path = self.storage_dir.join(format!("{}_{}.chunk", hex, i));
            let mut file = File::open(chunk_path)?;
            let mut chunk_data = Vec::new();
            file.read_to_end(&mut chunk_data)?;
            
            // Verify chunk integrity
            let computed = HashValue::compute(&chunk_data, HashAlgo::Sha256);
            if computed != *chunk_hash {
                anyhow::bail!("chunk {} integrity check failed", i);
            }
            full_data.extend(chunk_data);
        }
        Ok(full_data)
    }

    pub fn stats(&self) -> f64 {
        if self.dedup_stats.total_bytes == 0 { 
            0.0 
        } else {
            (self.dedup_stats.saved_bytes as f64 / self.dedup_stats.total_bytes as f64) * 100.0
        }
    }
}