// ============================================================================
// Hash Functions Core Module
// ============================================================================

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HashAlgo {
    Sha256,    // 32 bytes - Standard
    Sha512,    // 64 bytes - Fast on 64-bit
    Sha3_256,  // 32 bytes - Length extension attack resistant
    Sha3_512,  // 64 bytes - High security
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HashValue {
    pub algo: HashAlgo,
    pub bytes: Vec<u8>,
}

impl HashValue {
    pub fn compute(data: &[u8], algo: HashAlgo) -> Self {
        match algo {
            HashAlgo::Sha256 => {
                use sha2::Digest;
                Self { algo, bytes: sha2::Sha256::digest(data).to_vec() }
            },
            HashAlgo::Sha512 => {
                use sha2::Digest;
                Self { algo, bytes: sha2::Sha512::digest(data).to_vec() }
            },
            HashAlgo::Sha3_256 => {
                use sha3::Digest;
                Self { algo, bytes: sha3::Sha3_256::digest(data).to_vec() }
            },
            HashAlgo::Sha3_512 => {
                use sha3::Digest;
                Self { algo, bytes: sha3::Sha3_512::digest(data).to_vec() }
            },
        }
    }

    pub fn to_hex(&self) -> String { 
        hex::encode(&self.bytes) 
    }
    
    pub fn prefix(&self, len: usize) -> String { 
        hex::encode(&self.bytes[..len.min(self.bytes.len())]) 
    }
    
    pub fn size(&self) -> usize { 
        self.bytes.len() 
    }
}