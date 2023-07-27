#[cfg(test)]
mod block_tests {
    use crate::block::Block;
    #[test]
    fn test_new_block() {
        use crate::block::Block;
        let block = Block::initial(3);
        assert_eq!(block.generation, 0);
        assert_eq!(block.difficulty, 3);
        assert_eq!(block.data, String::default());
        assert_eq!(block.proof, None);
    }
    #[test]
    fn test_next_block() {
        let mut block = Block::initial(3);
        block.set_proof(0x0000_0000_0000_0000);
        let next_block = Block::next(&block, String::from("Hello"));
        assert_eq!(next_block.generation, 1);
        assert_eq!(next_block.difficulty, 3);
        assert_eq!(next_block.data, String::from("Hello"));
        assert_eq!(next_block.proof, None);
    }

    #[test]
    fn test_hash_string() {
        let mut block = Block::initial(3);
        block.mine_serial();
        println!("Hash: {}", block.hash_string());
    }


    #[test]
    fn test_hash_for_proof() {
        // Here is a test for the hash function
        // the hash function should return a expected hash value
        // for a given input


        let mut b0 = Block::initial(16);
        b0.mine(1);
        let hash_b0 = format!("{:02x}", b0.hash());
        let mut b1 = Block::next(&b0, String::from("message"));
        b1.mine(1);
        let hash_b1 = format!("{:02x}", b1.hash());
        // "6c71ff02a08a22309b7dbbcee45d291d4ce955caa32031c50d941e3e9dbd0000"
        // "964417b36afa6d31c728eed7abc14dd84468fdb055d8f3cbe308b0179df40000"
        assert_eq!(hash_b0, String::from("6c71ff02a08a22309b7dbbcee45d291d4ce955caa32031c50d941e3e9dbd0000"));
        assert_eq!(hash_b1, String::from("9b4417b36afa6d31c728eed7abc14dd84468fdb055d8f3cbe308b0179df40000"));
    }

    #[test]
    fn test_hash_for_proof2() {

        let mut b0 = Block::initial(7);
        b0.mine(1);
        let b0_hash = format!("{:02x}", b0.hash());

        let mut b1 = Block::next(&b0, String::from("this is an interesting message"));
        b1.mine(1);
        let b1_hash = format!("{:02x}", b1.hash());

        let mut b2 = Block::next(&b1, String::from("this is not interesting"));
        b2.mine(1);
        let b2_hash = format!("{:02x}", b2.hash());

        assert_eq!(b0_hash, String::from("379bf2fb1a558872f09442a45e300e72f00f03f2c6f4dd29971f67ea4f3d5300"));
        assert_eq!(b1_hash, String::from("4a1c722d8021346fa2f440d7f0bbaa585e632f68fd20fed812fc944613b92500"));
        assert_eq!(b2_hash, String::from("ba2f9bf0f9ec629db726f1a5fe7312eb76270459e3f5bfdc4e213df9e47cd380"));
    }
}
