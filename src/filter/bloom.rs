// ============================================================================
// Bloom Filter Implementation
// ============================================================================

use crate::crypto::hash::{HashAlgo, HashValue};

pub struct BloomFilter {
    bits: Vec<bool>,
    hashers: Vec<HashAlgo>,
    size: usize,
    num_items: usize,
}

impl BloomFilter {
    pub fn new(expected_items: usize, fp_rate: f64) -> Self {
        let m = (- (expected_items as f64) * fp_rate.ln() / (std::f64::consts::LN_2.powi(2))).ceil() as usize;
        let k = ((m as f64 / expected_items as f64) * std::f64::consts::LN_2).ceil() as usize;
        
        let hashers = vec![
            HashAlgo::Sha256,
            HashAlgo::Sha512,
            HashAlgo::Sha3_256,
        ].into_iter().take(k).collect();
        
        Self {
            bits: vec![false; m],
            hashers,
            size: m,
            num_items: 0,
        }
    }

    pub fn add(&mut self, item: &[u8]) {
        for algo in &self.hashers {
            let hash = HashValue::compute(item, *algo);
            let idx = self.hash_to_index(&hash);
            self.bits[idx] = true;
        }
        self.num_items += 1;
    }

    pub fn contains(&self, item: &[u8]) -> bool {
        for algo in &self.hashers {
            let hash = HashValue::compute(item, *algo);
            let idx = self.hash_to_index(&hash);
            if !self.bits[idx] { 
                return false; 
            }
        }
        true
    }

    fn hash_to_index(&self, hash: &HashValue) -> usize {
        let mut val = 0u64;
        for &b in &hash.bytes[..8] {
            val = (val << 8) | b as u64;
        }
        (val % self.size as u64) as usize
    }

    pub fn false_positive_rate(&self) -> f64 {
        let k = self.hashers.len() as f64;
        let m = self.size as f64;
        let n = self.num_items as f64;
        (1.0 - (-k * n / m).exp()).powf(k)
    }
}