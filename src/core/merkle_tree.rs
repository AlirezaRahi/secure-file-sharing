// ============================================================================
// Merkle Tree Implementation
// ============================================================================

use crate::crypto::hash::{HashAlgo, HashValue};

#[derive(Debug, Clone)]
pub struct MerkleTree {
    root: HashValue,
    leaves: Vec<HashValue>,
    levels: Vec<Vec<HashValue>>,
}

#[derive(Debug, Clone)]
pub struct MerkleProof {
    leaf_hash: HashValue,
    siblings: Vec<(HashValue, bool)>, // (hash, is_right)
    root_hash: HashValue,
}

impl MerkleTree {
    pub fn new(leaves: &[HashValue]) -> Self {
        if leaves.is_empty() {
            return Self {
                root: HashValue::compute(b"", HashAlgo::Sha256),
                leaves: vec![],
                levels: vec![],
            };
        }

        let mut levels = vec![leaves.to_vec()];
        let mut current = leaves.to_vec();

        while current.len() > 1 {
            let mut next = Vec::new();
            for pair in current.chunks(2) {
                let combined = if pair.len() == 2 {
                    let mut bytes = pair[0].bytes.clone();
                    bytes.extend(&pair[1].bytes);
                    HashValue::compute(&bytes, HashAlgo::Sha256)
                } else {
                    let mut bytes = pair[0].bytes.clone();
                    bytes.extend(&pair[0].bytes);
                    HashValue::compute(&bytes, HashAlgo::Sha256)
                };
                next.push(combined);
            }
            levels.push(next.clone());
            current = next;
        }

        Self {
            root: current[0].clone(),
            leaves: leaves.to_vec(),
            levels,
        }
    }

    pub fn root(&self) -> HashValue { 
        self.root.clone() 
    }

    pub fn generate_proof(&self, leaf_idx: usize) -> Option<MerkleProof> {
        if leaf_idx >= self.leaves.len() { 
            return None; 
        }
        
        let mut siblings = Vec::new();
        let mut idx = leaf_idx;
        
        for level in 0..self.levels.len()-1 {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            if sibling_idx < self.levels[level].len() {
                let is_right = idx % 2 == 0;
                siblings.push((self.levels[level][sibling_idx].clone(), is_right));
            }
            idx /= 2;
        }
        
        Some(MerkleProof {
            leaf_hash: self.leaves[leaf_idx].clone(),
            siblings,
            root_hash: self.root.clone(),
        })
    }

    pub fn verify_proof(proof: &MerkleProof) -> bool {
        let mut current = proof.leaf_hash.clone();
        for (sibling, is_right) in &proof.siblings {
            let combined = if *is_right {
                let mut bytes = current.bytes.clone();
                bytes.extend(&sibling.bytes);
                bytes
            } else {
                let mut bytes = sibling.bytes.clone();
                bytes.extend(&current.bytes);
                bytes
            };
            current = HashValue::compute(&combined, HashAlgo::Sha256);
        }
        current == proof.root_hash
    }
}