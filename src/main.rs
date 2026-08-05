mod rusty_merkle_tree;
mod rusty_merkle_tests;
use std::io::{BufReader, Error};

use rusty_merkle_tree::{MerkleTree, hash};

fn main() -> Result<(), Error> {
    let data: &[u8] = &[0x01, 0x02, 0x03, 0x04];
    let mut reader = BufReader::new(data);
    let mut tree = MerkleTree::new(&mut reader, 2)?;
    println!(
        "tree contains 0x01, 0x02: {:?}",
        tree.contains(&[0x01, 0x02])
    );
    println!("tree contains 0x05: {:?}", tree.contains(&[0x05]));
    println!(
        "tree contains 0x02, 0x03: {:?}",
        tree.contains(&[0x02, 0x03])
    );

    println!(
        "tree contains hash of 0x03, 0x04: {:?}",
        tree.contains_hash(&hash(&[0x03, 0x04]))
    );
    println!(
        "tree contains hash of 0x11: {:?}",
        tree.contains_hash(&hash(&[0x11]))
    );

    let data: &[u8] = &[0x05, 0x06, 0x07];
    reader = BufReader::new(data);
    tree = tree.append(&mut reader)?;
    println!("modified tree: {:#?}", tree);
    println!(
        "tree contains 0x05, 0x06?: {:?}",
        tree.contains(&[0x05, 0x06])
    );

    let proof = tree.generate_proof(1)?;
    println!("proof: {:#?}", proof);
    let root_hash = tree.get_root_hash();
    let leaf_hash = hash(&[0x03, 0x04]);
    assert!(proof.validate(root_hash, leaf_hash));

    Ok(())
}
