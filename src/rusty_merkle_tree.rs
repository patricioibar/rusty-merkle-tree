use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io::{Error, ErrorKind, Read},
};

pub fn hash(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

#[derive(Debug)]
pub struct MerkleTree {
    root: MerkleNode,
}

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

    pub fn new_leaf(hash: u64) -> MerkleNode {
        MerkleNode {
            hash,
            left: None,
            right: None,
        }
    }

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

    pub fn append_raw_data(mut self, data: impl Read, leaf_size: usize) -> Result<Self, Error> {
        let leaves = get_leaves_from_raw_data(data, leaf_size)?;
        for leaf in leaves {
            self = self.add_one_leaf(leaf)?;
        }
        Ok(self)
    }

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

    pub fn get_root_hash(&self) -> u64 {
        self.root.hash
    }

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

#[derive(Debug)]
pub struct MerkleProof {
    path: Vec<(u64, Direction)>, // could extract (u64, bool) in a struct "SiblingNode"?
}

impl MerkleProof {
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
