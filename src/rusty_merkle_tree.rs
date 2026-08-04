use std::io::{BufReader, Error, Read};

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
    pub fn new(data: &mut BufReader<&[u8]>, leaf_size: usize) -> Result<Self, Error> {
        let mut leaves = Vec::new();

        loop {
            let mut block = vec![0u8; leaf_size];
            let mut block_len = data.read(&mut block)?;
            if block_len == 0 { break; }
            while block_len < leaf_size {
                let res = data.read(&mut block[block_len..])?;
                if res == 0 { break; }
                block_len += res;
            }
            let leaf = MerkleNode {
                hash: hash(&block[0..block_len]),
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
        Ok(Self {
            root: parents[0].clone(),
        })
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

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Error};

    use super::MerkleTree;

    #[test]
    fn test_tree_leaf_size_1_contains_element() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::new(&mut reader, 1)?;

        assert!(tree.contains(&[0x01]));
        assert!(!tree.contains(&[0x07]));
        assert!(!tree.contains(&[0x01, 0x02]));
        Ok(())
    }

    #[test]
    fn test_tree_leaf_size_1_contains_element_hash() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::new(&mut reader, 1)?;

        let hash_01 = crate::hash(&[0x01]);
        let hash_07 = crate::hash(&[0x07]);

        assert!(tree.contains_hash(&hash_01));
        assert!(!tree.contains_hash(&hash_07));
        Ok(())
    }

    #[test]
    fn test_tree_leaf_size_4_contains_element() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::new(&mut reader, 4)?;

        assert!(tree.contains(&[0x01, 0x02, 0x03, 0x04]));
        assert!(tree.contains(&[0x05, 0x06, 0x07, 0x08]));
        assert!(!tree.contains(&[0x09, 0x0A, 0x0B, 0x0C]));
        Ok(())
    }

    #[test]
    fn test_tree_leaf_size_4_contains_element_hash() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::new(&mut reader, 4)?;

        let hash_first = crate::hash(&[0x01, 0x02, 0x03, 0x04]);
        let hash_second = crate::hash(&[0x05, 0x06, 0x07, 0x08]);
        let hash_invalid = crate::hash(&[0x09, 0x0A, 0x0B, 0x0C]);

        assert!(tree.contains_hash(&hash_first));
        assert!(tree.contains_hash(&hash_second));
        assert!(!tree.contains_hash(&hash_invalid));
        Ok(())
    }

    #[test]
    fn test_tree_leaf_size_4_not_contains_invalid_subarray() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::new(&mut reader, 4)?;

        assert!(!tree.contains(&[0x01, 0x02]));
        assert!(!tree.contains(&[0x02, 0x03, 0x04, 0x05]));
        assert!(!tree.contains(&[0x08]));
        Ok(())
    }

        #[test]
    fn test_tree_leaf_size_data_len_is_not_a_multiple_of_size() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::new(&mut reader, 4)?;

        assert!(tree.contains(&[0x01, 0x02, 0x03, 0x04]));
        assert!(tree.contains(&[0x05, 0x06, 0x07, 0x08]));
        assert!(tree.contains(&[0x09, 0x0A]));
        assert!(!tree.contains(&[0x09, 0x0A, 0x0B, 0x0C]));
        Ok(())
    }
}
