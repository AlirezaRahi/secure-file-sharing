//============================================================================
// Cryptographic Commitment Module
// ============================================================================

use super::hash::{HashAlgo, HashValue};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commitment {
    hash: HashValue,
    nonce: Vec<u8>,
}

impl Commitment {
    pub fn commit(secret: &[u8]) -> Self {
        use rand::RngCore;
        
        let mut nonce = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut nonce);
        
        let mut combined = secret.to_vec();
        combined.extend(&nonce);
        let hash = HashValue::compute(&combined, HashAlgo::Sha3_256);
        Self { hash, nonce }
    }

    pub fn verify(&self, secret: &[u8]) -> bool {
        let mut combined = secret.to_vec();
        combined.extend(&self.nonce);
        let computed = HashValue::compute(&combined, HashAlgo::Sha3_256);
        computed == self.hash
    }

    pub fn hash(&self) -> &HashValue { 
        &self.hash 
    }
}