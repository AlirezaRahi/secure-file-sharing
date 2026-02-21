// ============================================================================
// File Authentication System
// ============================================================================

use crate::crypto::hash::{HashAlgo, HashValue};
use crate::filter::bloom::BloomFilter;
use anyhow::{Result, Context};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

pub struct FileAuthenticator {
    known_files: HashMap<PathBuf, HashValue>,
    pub watch_dir: PathBuf,  // Made public
    pub bloom: BloomFilter,
}

impl FileAuthenticator {
    pub fn new(watch_dir: &Path) -> Self {
        Self {
            known_files: HashMap::new(),
            watch_dir: watch_dir.to_path_buf(),
            bloom: BloomFilter::new(1000, 0.01),
        }
    }

    pub fn register(&mut self, path: &Path) -> Result<()> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        let hash = HashValue::compute(&data, HashAlgo::Sha256);
        self.known_files.insert(path.to_path_buf(), hash.clone());
        self.bloom.add(path.to_string_lossy().as_bytes());
        
        println!("📋 registered: {} -> {}", path.display(), hash.prefix(8));
        Ok(())
    }

    pub fn verify(&self, path: &Path) -> Result<bool> {
        let old_hash = self.known_files.get(path)
            .context("file not registered")?;
        
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        
        let new_hash = HashValue::compute(&data, HashAlgo::Sha256);
        Ok(old_hash == &new_hash)
    }

    pub fn quick_check(&self, path: &Path) -> bool {
        self.bloom.contains(path.to_string_lossy().as_bytes())
    }
}