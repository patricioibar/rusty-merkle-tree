#[cfg(test)]
mod tests {
    use rand::Rng;
    use std::io::{BufReader, Error};

    use crate::MerkleTree;

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

    #[test]
    fn test_tree_proof_case_1() -> Result<(), Error> {
        let data: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let tree = MerkleTree::new(data, 2)?;
        let proof = tree.generate_proof(0)?;
        let leaf_hash = crate::hash(&[0x01, 0x02]);
        let root_hash = tree.get_root_hash();
        assert!(proof.validate(root_hash, leaf_hash));
        Ok(())
    }

    #[test]
    fn test_tree_proof_case_2() -> Result<(), Error> {
        let data: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let tree = MerkleTree::new(data, 2)?;
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
        let tree = MerkleTree::new(file_bytes.as_slice(), 512)?;
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
        let tree = MerkleTree::new(file_bytes.as_slice(), 512)?;
        let proof = tree.generate_proof(20)?;
        let root_hash = tree.get_root_hash();
        let leaf_hash = crate::hash(&file_bytes[10240..10752]);
        assert!(proof.validate(root_hash, leaf_hash));
        Ok(())
    }

    #[test]
    fn test_proof_invalid_leaf_number() -> Result<(), Error> {
        let data: &[u8] = &[0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let tree = MerkleTree::new(data, 2)?;
        let result = tree.generate_proof(4);
        assert!(result.is_err());
        Ok(())
    }
}
