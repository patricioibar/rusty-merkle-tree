use sha2::{Digest, Sha256};

pub fn hash(data: &[u8]) -> u64 {
    Sha256::digest(data)
        .iter()
        .take(8)
        .fold(0u64, |acc, &byte| (acc << 8) | byte as u64)
}

#[derive(Debug)]
pub struct MerkleTree {
    root: Box<MerkleNode>,
}

#[derive(Clone, Debug)]
struct MerkleNode {
    hash: u64,
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
}

impl MerkleTree {
    pub fn new(data: &[u8]) -> Self {
        let n = data.len();
        let n_leaves = n;
        let mut leaves = Vec::with_capacity(n_leaves);

        for block in data.chunks(n / n_leaves) {
            let leaf = MerkleNode {
                hash: hash(block),
                left: None,
                right: None,
            };
            leaves.push(Box::new(leaf));
        }

        let mut finalized = false;
        let mut parents = Vec::new();
        while !finalized {
            for couple in leaves.chunks_mut(2) {
                let left_hash = couple[0].hash;
                let right_hash = if couple.len() > 1 {
                    couple[1].hash
                } else {
                    left_hash
                };
                let parent_hash =
                    hash(&[left_hash.to_le_bytes(), right_hash.to_le_bytes()].concat());
                let parent_node = MerkleNode {
                    hash: parent_hash,
                    left: Some(Box::new(*couple[0].clone())),
                    right: if left_hash == right_hash {
                        None
                    } else {
                        Some(Box::new(*couple[1].clone()))
                    },
                };
                parents.push(Box::new(parent_node));
            }
            if parents.len() == 1 {
                finalized = true;
            } else {
                leaves = parents;
                parents = Vec::new();
            }
        }
        Self {
            root: parents[0].clone(),
        }
    }

    pub fn contains(&self, data: &[u8]) -> bool {
        let target_hash = hash(data);
        self.contains_hash(&target_hash)
    }

    pub fn contains_hash(&self, target_hash: &u64) -> bool {
        self.contains_hash_recursive(&self.root, target_hash)
    }

    fn contains_hash_recursive(&self, node: &MerkleNode, target_hash: &u64) -> bool {
        if node.hash == *target_hash {
            return true;
        }
        if let Some(ref left) = node.left {
            if self.contains_hash_recursive(left, target_hash) {
                return true;
            }
        }
        if let Some(ref right) = node.right {
            if self.contains_hash_recursive(right, target_hash) {
                return true;
            }
        }
        false
    }
}
