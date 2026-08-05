//! This module implements a simple Merkle tree data structure in Rust.

use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io::{Error, ErrorKind, Read},
};

pub fn hash(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Merkle tree data structure that allows for efficient and secure verification of data integrity.
/// The tree can be built from raw data or from a list of leaves.
/// The tree can be modified by appending new leaves or raw data, and it supports generating proofs for the presence of specific leaves.
#[derive(Debug)]
pub struct MerkleTree {
    root: MerkleNode,
}

/// Merkle node structure that represents a node in the Merkle tree.
/// Leaf nodes can be created from raw data or from a hash, to construct or append to a Merkle tree.
/// Inner nodes are created by combining two child nodes, and their hash is computed from the hashes of the child nodes.
/// Inner nodes are managed internally by the Merkle tree and should not be created or modified directly by users.
#[derive(Clone, Debug)]
pub struct MerkleNode {
    hash: u64,
    left: Option<Box<MerkleNode>>,
    right: Option<Box<MerkleNode>>,
}

impl MerkleNode {
    fn new_inner_node(left: MerkleNode, right: Option<MerkleNode>) -> MerkleNode {
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

    /// Creates a new leaf node with the given hash.
    pub fn new_leaf(hash: u64) -> MerkleNode {
        MerkleNode {
            hash,
            left: None,
            right: None,
        }
    }

    /// Creates a new leaf node from the given data.
    /// The data is read and hashed to create the leaf node.
    pub fn new_leaf_from_data(data: impl Read) -> Result<Self, Error> {
        let mut buffer = Vec::new();
        let _ = data.take(usize::MAX as u64).read_to_end(&mut buffer)?;
        Ok(MerkleNode {
            hash: hash(&buffer),
            left: None,
            right: None,
        })
    }

    fn is_leaf(&self) -> bool {
        self.left.is_none() && self.right.is_none()
    }
}

impl MerkleTree {
    /// Creates a new Merkle tree from raw data, reading the data in chunks of the specified leaf size.
    /// The leaf size must be greater than 0, and the data must not be empty
    ///
    /// The tree is constructed by creating leaf nodes from the data and then pairing them to create inner nodes until a single root node is reached.
    pub fn from_raw_data(data: impl Read, leaf_size: usize) -> Result<Self, Error> {
        if leaf_size == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "leaf size must be greater than 0",
            ));
        }
        let leaves = get_leaves_from_raw_data(data, leaf_size)?;
        Self::from_leaves(leaves)
    }

    /// Creates a new Merkle tree from a list of leaf nodes.
    /// The list of leaves must not be empty, and all nodes must be leaves (i.e., they must not have any children).
    ///
    /// The tree is constructed by pairing leaf nodes to create inner nodes until a single root node is reached.
    pub fn from_leaves(leaves: Vec<MerkleNode>) -> Result<Self, Error> {
        if leaves.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "tree cannot be empty"));
        }

        for leaf in &leaves {
            if !leaf.is_leaf() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "all nodes must be leaves",
                ));
            }
        }

        let mut descendants = leaves;

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

        Ok(Self { root })
    }

    /// Appends raw data to the Merkle tree, reading the data in chunks of the specified leaf size.
    /// The leaf size must be greater than 0, and the data must not be empty
    ///
    /// The new leaves are created from the data and added to the tree, updating the root if necessary.
    /// The tree is modified in place, and a new Merkle tree is returned with the updated structure.
    pub fn append_raw_data(mut self, data: impl Read, leaf_size: usize) -> Result<Self, Error> {
        let leaves = get_leaves_from_raw_data(data, leaf_size)?;
        for leaf in leaves {
            self = self.add_one_leaf(leaf)?;
        }
        Ok(self)
    }

    /// Appends a list of leaf nodes to the Merkle tree.
    /// The list of leaves must not be empty, and all nodes must be leaves (i.e., they must not have any children).
    ///
    /// The new leaves are added to the tree, updating the root if necessary.
    /// The tree is modified in place, and a new Merkle tree is returned with the updated structure.
    pub fn append(mut self, leaves: Vec<MerkleNode>) -> Result<Self, Error> {
        for leaf in &leaves {
            if !leaf.is_leaf() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "all nodes must be leaves",
                ));
            }
        }
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

    /// Checks if the Merkle tree contains a leaf node with the given data.
    /// The data is hashed and compared to the hashes of the leaf nodes in the tree.
    /// Returns true if the leaf node is found, false otherwise.
    pub fn contains(&self, data: &[u8]) -> bool {
        let target_hash = hash(data);
        self.contains_hash(&target_hash)
    }

    /// Checks if the Merkle tree contains a leaf node with the given hash.
    /// Returns true if the leaf node is found, false otherwise.
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

    /// Returns the root hash of the Merkle tree.
    pub fn get_root_hash(&self) -> u64 {
        self.root.hash
    }

    /// Generates a Merkle proof for the leaf node at the given index.
    ///
    /// The leaf number must be within the bounds of the tree (i.e., less than 2^depth).
    pub fn generate_proof(&self, mut leaf_number: usize) -> Result<MerkleProof, Error> {
        if leaf_number >= 2_u32.pow(self.depth() as u32) as usize {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "leaf number is out of bounds",
            ));
        }
        let mut depth = self.depth();
        let mut sibling_nodes = vec![];
        let mut actual_node = &self.root;
        while depth > 0 {
            // unwrap left and right node
            let (left_node, right_node) = match (&actual_node.left, &actual_node.right) {
                (Some(left), Some(right)) => Ok((left, right)),
                (Some(left), None) => Ok((left, left)),
                _ => Err(Error::new(ErrorKind::InvalidData, "invalid tree")),
            }?;

            let remaining_nodes = 2_u32.pow(depth as u32) as usize;
            let move_left = leaf_number < remaining_nodes / 2;
            if move_left {
                sibling_nodes.push((right_node.hash, Direction::Right));
                actual_node = left_node;
            } else {
                sibling_nodes.push((left_node.hash, Direction::Left));
                actual_node = right_node;
                leaf_number -= remaining_nodes / 2;
            }
            depth -= 1;
        }

        Ok(MerkleProof {
            path: sibling_nodes,
        })
    }
}

#[derive(Debug, Clone)]
enum Direction {
    Left,
    Right,
}

/// Merkle proof structure that represents a proof of the presence of a leaf node in the Merkle tree.
/// The proof consists of the hashes of the sibling nodes along the path from the leaf to the root, along with the direction (left or right) of each sibling node.
/// The proof can be validated by recomputing the hash from the leaf to the root and comparing it to the root hash of the tree.
#[derive(Debug)]
pub struct MerkleProof {
    path: Vec<(u64, Direction)>, // could extract (u64, bool) in a struct "SiblingNode"?
}

impl MerkleProof {
    /// Validates the Merkle proof against the given root hash and leaf hash.
    /// The proof is valid if the recomputed hash from the leaf to the root matches the given root hash.
    /// Returns true if the proof is valid, false otherwise.
    pub fn validate(&self, root_hash: u64, leaf_hash: u64) -> bool {
        let mut actual = leaf_hash;

        // hash following the path until it reach root
        for (sibling_hash, direction) in self.path.iter().rev() {
            let concat = match direction {
                Direction::Left => &[sibling_hash.to_le_bytes(), actual.to_le_bytes()].concat(),
                Direction::Right => &[actual.to_le_bytes(), sibling_hash.to_le_bytes()].concat(),
            };

            actual = hash(concat);
        }
        actual == root_hash
    }
}

fn get_leaves_from_raw_data(
    mut data: impl Read,
    leaf_size: usize,
) -> Result<Vec<MerkleNode>, Error> {
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
