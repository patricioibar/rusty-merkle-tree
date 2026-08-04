mod rusty_merkle_tree;
use std::io::{BufReader, Error};

use rusty_merkle_tree::{MerkleTree, hash};

fn main() -> Result<(), Error> {
    let data: &[u8] = &vec![0x01, 0x02, 0x03, 0x04];
    let mut reader = BufReader::new(data);
    let tree = MerkleTree::new(&mut reader, 2)?;

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
        tree.contains_hash(&hash(&[0x03, 0x0]))
    );
    println!(
        "tree contains hash of 0x11: {:?}",
        tree.contains_hash(&hash(&[0x11]))
    );

    Ok(())
}
