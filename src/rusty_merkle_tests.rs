#[cfg(test)]
mod tests {
    use rand::Rng;
    use std::io::{BufReader, Error};

    use crate::{MerkleTree, rusty_merkle_tree::MerkleNode};

    #[test]
    fn test_tree_leaf_size_1_contains_element() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::from_raw_data(&mut reader, 1)?;

        assert!(tree.contains(&[0x01]));
        assert!(!tree.contains(&[0x07]));
        assert!(!tree.contains(&[0x01, 0x02]));
        Ok(())
    }

    #[test]
    fn test_tree_leaf_size_1_contains_element_hash() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::from_raw_data(&mut reader, 1)?;

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
        let tree = MerkleTree::from_raw_data(&mut reader, 4)?;

        assert!(tree.contains(&[0x01, 0x02, 0x03, 0x04]));
        assert!(tree.contains(&[0x05, 0x06, 0x07, 0x08]));
        assert!(!tree.contains(&[0x09, 0x0A, 0x0B, 0x0C]));
        Ok(())
    }

    #[test]
    fn test_tree_leaf_size_4_contains_element_hash() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::from_raw_data(&mut reader, 4)?;

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
        let tree = MerkleTree::from_raw_data(&mut reader, 4)?;

        assert!(!tree.contains(&[0x01, 0x02]));
        assert!(!tree.contains(&[0x02, 0x03, 0x04, 0x05]));
        assert!(!tree.contains(&[0x08]));
        Ok(())
    }

    #[test]
    fn test_tree_leaf_size_data_len_is_not_a_multiple_of_size() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A];
        let mut reader = BufReader::new(data);
        let tree = MerkleTree::from_raw_data(&mut reader, 4)?;

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
        let mut tree = MerkleTree::from_raw_data(&mut reader, 2)?;

        let data: &[u8] = &[0x09, 0x0A, 0x0B, 0x0C];
        let mut reader = BufReader::new(data);
        tree = tree.append_raw_data(&mut reader, 2)?;
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
        let mut tree = MerkleTree::from_raw_data(&mut reader, 2)?;

        let data: &[u8] = &[0x00];
        let mut reader = BufReader::new(data);
        tree = tree.append_raw_data(&mut reader, 2)?;
        assert!(tree.contains(&[0x00]));
        Ok(())
    }

    #[test]
    fn test_tree_append() -> Result<(), Error> {
        let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let mut reader = BufReader::new(data);
        let mut tree = MerkleTree::from_raw_data(&mut reader, 3)?;

        assert!(tree.contains(&[0x04, 0x05, 0x06]));
        assert!(tree.contains(&[0x07, 0x08]));

        let data: &[u8] = &[0x09, 0x0A, 0x0B, 0x0C];
        let mut reader = BufReader::new(data);
        tree = tree.append_raw_data(&mut reader, 3)?;

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
        let mut tree = MerkleTree::from_raw_data(file_bytes.as_slice(), 512)?;

        assert!(tree.contains(&file_bytes[0..512]));
        assert!(tree.contains(&file_bytes[2048..2560]));
        assert!(!tree.contains(&file_bytes[1000..1512]));

        let new_file_size = 1024 * 5 + 123;
        let mut new_file_bytes = vec![0u8; new_file_size];
        rng.fill_bytes(&mut new_file_bytes);
        tree = tree.append_raw_data(new_file_bytes.as_slice(), 512)?;

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
        let mut tree = MerkleTree::from_raw_data(file_bytes.as_slice(), 512)?;

        assert!(tree.contains(&file_bytes[0..512]));
        assert!(tree.contains(&file_bytes[2048..2560]));
        assert!(tree.contains(&file_bytes[20480..20992]));
        assert!(!tree.contains(&file_bytes[100003..100512]));

        let new_file_size = 1024 * 1000 + 123;
        let mut new_file_bytes = vec![0u8; new_file_size];
        rng.fill_bytes(&mut new_file_bytes);
        tree = tree.append_raw_data(new_file_bytes.as_slice(), 512)?;

        assert!(tree.contains(&new_file_bytes[0..512]));
        assert!(tree.contains(&new_file_bytes[2048..2560]));
        assert!(tree.contains(&new_file_bytes[20480..20992]));
        assert!(!tree.contains(&new_file_bytes[100003..100512]));

        Ok(())
    }

    #[test]
    fn test_tree_proof_case_1() -> Result<(), Error> {
        let data: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let tree = MerkleTree::from_raw_data(data, 2)?;
        let proof = tree.generate_proof(0)?;
        let leaf_hash = crate::hash(&[0x01, 0x02]);
        let root_hash = tree.get_root_hash();
        assert!(proof.validate(root_hash, leaf_hash));
        Ok(())
    }

    #[test]
    fn test_tree_proof_case_2() -> Result<(), Error> {
        let data: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let tree = MerkleTree::from_raw_data(data, 2)?;
        let proof = tree.generate_proof(2)?;
        let leaf_hash = crate::hash(&[0x05, 0x06]);
        let root_hash = tree.get_root_hash();
        assert!(proof.validate(root_hash, leaf_hash));
        Ok(())
    }

    #[test]
    fn test_proof_from_larger_data_kb() -> Result<(), Error> {
        let file_size = 1024 * 100;
        let mut file_bytes = vec![0u8; file_size];
        let mut rng: rand::rngs::ThreadRng = rand::rngs::ThreadRng::default();
        rng.fill_bytes(&mut file_bytes);
        let tree = MerkleTree::from_raw_data(file_bytes.as_slice(), 512)?;
        let proof = tree.generate_proof(2)?;
        let root_hash = tree.get_root_hash();
        let leaf_hash = crate::hash(&file_bytes[1024..1536]);
        assert!(proof.validate(root_hash, leaf_hash));
        Ok(())
    }

    #[test]
    fn test_proof_from_larger_data_mb() -> Result<(), Error> {
        let file_size = 1024 * 10000;
        let mut file_bytes = vec![0u8; file_size];
        let mut rng: rand::rngs::ThreadRng = rand::rngs::ThreadRng::default();
        rng.fill_bytes(&mut file_bytes);
        let tree = MerkleTree::from_raw_data(file_bytes.as_slice(), 512)?;
        let proof = tree.generate_proof(20)?;
        let root_hash = tree.get_root_hash();
        let leaf_hash = crate::hash(&file_bytes[10240..10752]);
        assert!(proof.validate(root_hash, leaf_hash));
        Ok(())
    }

    #[test]
    fn test_proof_invalid_leaf_number() -> Result<(), Error> {
        let data: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let tree = MerkleTree::from_raw_data(data, 2)?;
        let result = tree.generate_proof(4);
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn test_tree_arbitrary_leave_sizes() -> Result<(), Error> {
        let leaves = vec![
            MerkleNode::new_leaf_from_data(&[1u8, 2u8, 3u8] as &[u8])?,
            MerkleNode::new_leaf_from_data(&[3u8, 3u8] as &[u8])?,
            MerkleNode::new_leaf_from_data(&[0u8, 0xFF, 0xFF, 0xFF, 0xFF] as &[u8])?,
            MerkleNode::new_leaf_from_data(&[0u8] as &[u8])?,
        ];
        let mut tree = MerkleTree::from_leaves(leaves)?;

        assert!(tree.contains(&[3u8, 3u8] as &[u8]));

        tree = tree.append(vec![
            MerkleNode::new_leaf_from_data(&[12u8, 13, 14] as &[u8])?,
            MerkleNode::new_leaf_from_data(&[0u8, 0, 0, 0, 0, 3, 3, 3] as &[u8])?,
            MerkleNode::new_leaf_from_data(&[10u8, 9, 8, 7, 7] as &[u8])?,
        ])?;

        assert!(tree.contains(&[0u8, 0, 0, 0, 0, 3, 3, 3] as &[u8]));

        let proof = tree.generate_proof(4)?;
        let leaf_hash = crate::hash(&[12u8, 13, 14] as &[u8]);
        let root_hash = tree.get_root_hash();
        assert!(proof.validate(root_hash, leaf_hash));

        Ok(())
    }

    #[test]
    fn test_tree_and_proof_from_pseudo_transactions() -> Result<(), Error> {
        struct Transaction {
            origin: u64,
            destiny: u64,
            amount: f64,
        }
        let block = vec![
            Transaction {
                origin: 1,
                destiny: 2,
                amount: 10.5,
            },
            Transaction {
                origin: 2,
                destiny: 3,
                amount: 20.0,
            },
            Transaction {
                origin: 3,
                destiny: 4,
                amount: 5.25,
            },
            Transaction {
                origin: 4,
                destiny: 5,
                amount: 99.0,
            },
            Transaction {
                origin: 5,
                destiny: 6,
                amount: 1.0,
            },
            Transaction {
                origin: 6,
                destiny: 7,
                amount: 42.42,
            },
            Transaction {
                origin: 7,
                destiny: 8,
                amount: 7.7,
            },
            Transaction {
                origin: 8,
                destiny: 9,
                amount: 0.5,
            },
            Transaction {
                origin: 9,
                destiny: 10,
                amount: 1000.0,
            },
        ];

        fn transaction_to_bytes(tx: &Transaction) -> Vec<u8> {
            let mut data = Vec::new();
            data.extend_from_slice(&tx.origin.to_le_bytes());
            data.extend_from_slice(&tx.destiny.to_le_bytes());
            data.extend_from_slice(&tx.amount.to_le_bytes());
            data
        }

        let leaves = block
            .iter()
            .map(|tx| MerkleNode::new_leaf_from_data(transaction_to_bytes(tx).as_slice()))
            .collect::<Result<Vec<_>, Error>>()?;

        let tree = MerkleTree::from_leaves(leaves)?;

        let real_transaction = transaction_to_bytes(&block[6]);
        let made_up_transaction = transaction_to_bytes(&Transaction {
            origin: 94870, // you
            destiny: 109569, // me
            amount: 999999.9999, // all your money
        });

        assert!(tree.contains(&real_transaction));
        assert!(!tree.contains(&made_up_transaction));

        let proof = tree.generate_proof(3)?;
        let leaf_data = transaction_to_bytes(&block[3]);
        let leaf_hash = crate::hash(leaf_data.as_slice());

        assert!(proof.validate(tree.get_root_hash(), leaf_hash));

        Ok(())
    }
}
