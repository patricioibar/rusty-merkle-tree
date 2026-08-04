mod rusty_merkle_tree;
use rusty_merkle_tree::{MerkleTree, hash};

fn main() {
    let data = vec![0x01, 0x02, 0x03, 0x04];
    let tree = MerkleTree::new(&data);

    println!("tree: {:#?}", &tree);

    println!("hash 0x02: {:?}", hash(&[0x02]));
    println!("hash 0x05: {:?}", hash(&[0x05]));
    println!("tree contains 0x02: {:?}", tree.contains(&[0x02]));
    println!("tree contains 0x05: {:?}", tree.contains(&[0x05]));

    println!(
        "tree contains hash of 0x01: {:?}",
        tree.contains_hash(&hash(&[0x01]))
    );
    println!(
        "tree contains hash of 0x11: {:?}",
        tree.contains_hash(&hash(&[0x11]))
    );
}
