use std::io::{Error, ErrorKind, Read};

use sha2::{Digest, Sha256};

pub fn hash(data: &[u8]) -> u64 {
    Sha256::digest(data)
        .iter()
        .take(8)
        .fold(0u64, |acc, &byte| (acc << 8) | byte as u64)
}

#[derive(Debug)]
pub struct MerkleTree {
    leaf_size: usize,
    root: MerkleNode,
}

#[derive(Clone, Debug)]
struct MerkleNode {
    hash: u64,
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
}

impl MerkleNode {
    pub fn new_inner_node(left: MerkleNode, right: Option<MerkleNode>) -> MerkleNode {
        let left_hash = left.hash;
        let right_hash = if let Some(right_node) = &right {
            right_node.hash
        } else {
            left_hash
        };
        MerkleNode {
            hash: hash(&[left_hash.to_le_bytes(), right_hash.to_le_bytes()].concat()),
            left: Some(Box::new(left)),
            right: right.map(Box::new),
        }
    }

    pub fn new_leaf(hash: u64) -> MerkleNode {
        MerkleNode {
            hash,
            left: None,
            right: None,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }
}

impl MerkleTree {
    pub fn new(data: impl Read, leaf_size: usize) -> Result<Self, Error> {
        if leaf_size == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "leaf size must be greater than 0",
            ));
        }

        let mut descendants = get_leaves(data, leaf_size)?;

        if descendants.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "tree cannot be empty"));
        }

        while descendants.len() > 1 {
            let mut parents = Vec::new();
            let mut children = descendants.into_iter();

            // create one parent node for every two children nodes
            while let Some(left) = children.next() {
                let right = children.next();
                parents.push(MerkleNode::new_inner_node(left, right));
            }

            // continue pairing parents until we reach the root
            descendants = parents;
        }

        let root = descendants
            .pop()
            .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "tree cannot be empty"))?;

        Ok(Self { root, leaf_size })
    }

    pub fn append(mut self, data: impl Read) -> Result<Self, Error> {
        let leaves = get_leaves(data, self.leaf_size)?;
        for leaf in leaves {
            self = self.add_one_leaf(leaf)?;
        }
        Ok(self)
    }

    fn add_one_leaf(mut self, leaf: MerkleNode) -> Result<Self, Error> {
        let depth = self.depth();

        // walk down tree, collecting every node that may need to be updated
        let mut parents = vec![];
        let mut actual = Box::new(self.root);

        for _ in 0..depth {
            let next = if let Some(node) = actual.right.take() {
                Ok(node)
            } else {
                if let Some(node) = actual.left.take() {
                    Ok(node)
                } else {
                    // all leaves should be in the same depth
                    Err(Error::new(ErrorKind::InvalidInput, "invalid tree"))
                }
            }?;
            parents.push(actual);
            actual = next;
        }

        // insert the new leaf and create a new branch if needed
        // also update corresponding hashes
        let mut prev_node = actual; // pre-existing leaf node, the one that is furthest to the right
        let mut new_branch = Some(leaf); // highest node of the new branch
        while let Some(mut node) = parents.pop() {
            // reconstruct node by adding previous node
            if node.left.is_none() {
                node.left = Some(prev_node);
            } else {
                node.right = Some(prev_node);
            };

            match (&node.right, new_branch.take()) {
                (None, Some(top_of_new_branch)) => {
                    // add new node to right side and recompute hash
                    let updated_node = MerkleNode::new_inner_node(
                        *node.left.take().unwrap(),
                        Some(top_of_new_branch),
                    );
                    node = Box::new(updated_node);
                    // new branch has been merged, no need to keep it for the next iteration
                    new_branch = None;
                }
                (Some(_), Some(top_of_new_branch)) => {
                    // as new branch can't be merged yet, it has to be extended one level up
                    new_branch = Some(MerkleNode::new_inner_node(top_of_new_branch, None));
                }
                (_, None) => {
                    // new branch already merged, just recompute hash
                    let updated_node = MerkleNode::new_inner_node(
                        *node.left.take().unwrap(),    // left child is guaranteed to exist
                        node.right.take().map(|n| *n), // unbox if exists
                    );
                    node = Box::new(updated_node);
                }
            }
            // this node will be included in it's parent in the next iteration
            prev_node = node;
        }

        let prev_root = prev_node;
        if let Some(top_of_new_branch) = new_branch {
            // new branch didn't merge yet -- tree was complete before this append
            // merge new branch with previous root and update new root.
            let new_node = MerkleNode::new_inner_node(*prev_root, Some(top_of_new_branch));
            self.root = new_node;
        } else {
            self.root = *prev_root;
        }

        Ok(self)
    }

    fn depth(&self) -> usize {
        let mut depth = 0;
        let mut actual = &self.root;
        while let Some(node) = &actual.left {
            actual = node;
            depth += 1;
        }
        depth
    }

    pub fn contains(&self, data: &[u8]) -> bool {
        let target_hash = hash(data);
        self.contains_hash(&target_hash)
    }

    pub fn contains_hash(&self, target_hash: &u64) -> bool {
        self.contains_hash_recursive(&self.root, target_hash)
    }

    fn contains_hash_recursive(&self, node: &MerkleNode, target_hash: &u64) -> bool {
        if node.is_leaf() && node.hash == *target_hash {
            return true;
        }
        if let Some(ref left) = node.left
            && self.contains_hash_recursive(left, target_hash)
        {
            return true;
        }
        if let Some(ref right) = node.right
            && self.contains_hash_recursive(right, target_hash)
        {
            return true;
        }
        false
    }
}

fn get_leaves(mut data: impl Read, leaf_size: usize) -> Result<Vec<MerkleNode>, Error> {
    let mut leaves = Vec::new();
    loop {
        let mut block = vec![0u8; leaf_size];
        let mut block_len = data.read(&mut block)?;
        if block_len == 0 {
            break;
        }
        while block_len < leaf_size {
            let res = data.read(&mut block[block_len..])?;
            if res == 0 {
                break;
            }
            block_len += res;
        }
        let leaf = MerkleNode::new_leaf(hash(&block[0..block_len]));
        leaves.push(leaf);
    }
    Ok(leaves)
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, Error};
    use rand::Rng;

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

    #[test]
    fn test_tree_append_case_1() -> Result<(), Error> {
        /*
            old nodes: o
            new nodes: x

                         x
                     /      \
                    o        x
                  /   \     /
                 o    o    x
                / \  / \  / \
               o  o o  o x  x
        */
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = BufReader::new(data);
        let mut tree = MerkleTree::new(&mut reader, 2)?;

        let data: &[u8] = &[0x09, 0x0A, 0x0B, 0x0C];
        let mut reader = BufReader::new(data);
        tree = tree.append(&mut reader)?;
        assert!(tree.contains(&[0x09, 0x0A]));
        assert!(tree.contains(&[0x0B, 0x0C]));
        Ok(())
    }

    #[test]
    fn test_tree_append_case_2() -> Result<(), Error> {
        /*
            old nodes: o
            new nodes: x

                         o
                     /       \
                    o         o
                  /   \     /   \
                 o    o    o    x
                / \  / \  / \  /
               o  o o  o o  o x
        */
        let data: &[u8] = &vec![
            0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C,
        ];
        let mut reader = BufReader::new(data);
        let mut tree = MerkleTree::new(&mut reader, 2)?;

        let data: &[u8] = &[0x00];
        let mut reader = BufReader::new(data);
        tree = tree.append(&mut reader)?;
        assert!(tree.contains(&[0x00]));
        Ok(())
    }

    #[test]
    fn test_tree_append() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = BufReader::new(data);
        let mut tree = MerkleTree::new(&mut reader, 3)?;

        assert!(tree.contains(&[0x04, 0x05, 0x06]));
        assert!(tree.contains(&[0x07, 0x08]));

        let data: &[u8] = &[0x09, 0x0A, 0x0B, 0x0C];
        let mut reader = BufReader::new(data);
        tree = tree.append(&mut reader)?;

        assert!(tree.contains(&[0x09, 0x0A, 0x0B]));
        assert!(tree.contains(&[0x0C]));
        assert!(!tree.contains(&[0x07, 0x08, 0x09]));
        Ok(())
    }

    #[test]
    fn test_tree_from_larger_data_kb() -> Result<(), Error> {
        let file_size = 1024 * 100;
        let mut file_bytes = vec![0u8; file_size];
        let mut rng: rand::rngs::ThreadRng = rand::rngs::ThreadRng::default();
        rng.fill_bytes(&mut file_bytes);
        let mut tree = MerkleTree::new(file_bytes.as_slice(), 512)?;

        assert!(tree.contains(&file_bytes[0..512]));
        assert!(tree.contains(&file_bytes[2048..2560]));
        assert!(!tree.contains(&file_bytes[1000..1512]));

        let new_file_size = 1024 * 5 + 123;
        let mut new_file_bytes = vec![0u8; new_file_size];
        rng.fill_bytes(&mut new_file_bytes);
        tree = tree.append(new_file_bytes.as_slice())?;

        assert!(tree.contains(&new_file_bytes[0..512]));
        assert!(tree.contains(&new_file_bytes[2048..2560]));
        assert!(!tree.contains(&new_file_bytes[1000..1512]));

        Ok(())
    }

    #[test]
    fn test_tree_from_larger_data_mb() -> Result<(), Error> {
        let file_size = 1024 * 10000;
        let mut file_bytes = vec![0u8; file_size];
        let mut rng: rand::rngs::ThreadRng = rand::rngs::ThreadRng::default();
        rng.fill_bytes(&mut file_bytes);
        let mut tree = MerkleTree::new(file_bytes.as_slice(), 512)?;

        assert!(tree.contains(&file_bytes[0..512]));
        assert!(tree.contains(&file_bytes[2048..2560]));
        assert!(tree.contains(&file_bytes[20480..20992]));
        assert!(!tree.contains(&file_bytes[100003..100512]));

        let new_file_size = 1024 * 1000 + 123;
        let mut new_file_bytes = vec![0u8; new_file_size];
        rng.fill_bytes(&mut new_file_bytes);
        tree = tree.append(new_file_bytes.as_slice())?;

        assert!(tree.contains(&new_file_bytes[0..512]));
        assert!(tree.contains(&new_file_bytes[2048..2560]));
        assert!(tree.contains(&new_file_bytes[20480..20992]));
        assert!(!tree.contains(&new_file_bytes[100003..100512]));

        Ok(())
    }
}
